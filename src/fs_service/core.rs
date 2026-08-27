use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::utils::{expand_home, normalize_windows_drive_path, parse_file_path},
};
use cap_std::{ambient_authority, fs::Dir};
use std::{
    collections::HashSet,
    env,
    io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;

type PathResultList = Vec<Result<PathBuf, ServiceError>>;

/// A confined, capability-based handle to one of the allowed directories.
///
/// All filesystem access is performed relative to [`AllowedDir::dir`], so that
/// symlink escapes (existing, dangling, or raced) are rejected by the OS layer
/// rather than by path arithmetic.
pub struct AllowedDir {
    /// Canonical absolute path, used only for prefix matching and display.
    pub path: PathBuf,
    /// The `cap-std` directory handle that confines access to this subtree.
    pub dir: Dir,
}

/// The result of resolving a requested path against the allowed directories.
///
/// Carries the confined directory handle and the relative path to operate on.
pub struct Resolved {
    pub dir: Dir,
    pub rel: PathBuf,
    /// Canonical absolute path, used for display and error messages only.
    pub display: PathBuf,
}

/// A filesystem entry discovered during a confined directory traversal.
pub struct FsEntry {
    pub dir: Dir,
    pub rel: PathBuf,
    pub display: PathBuf,
    pub file_name: String,
    pub is_dir: bool,
    pub len: u64,
}

impl FsEntry {
    /// The absolute (display) path of this entry.
    pub fn path(&self) -> &Path {
        &self.display
    }

    /// The file name of this entry.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn is_file(&self) -> bool {
        !self.is_dir
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Opens this entry for reading through its confined directory handle.
    pub fn open(&self) -> io::Result<cap_std::fs::File> {
        self.dir.open(&self.rel)
    }
}

/// Recursively collects all entries (files and directories) under `dir`/`base`
/// into `entries`, confined to `dir`'s subtree. Symlinks are followed only when
/// they resolve within the subtree; escaping symlinks are skipped.
pub fn walk_dir(
    dir: &Dir,
    base: &Path,
    display_base: &Path,
    entries: &mut Vec<FsEntry>,
) -> io::Result<()> {
    let read_dir = if base.as_os_str().is_empty() {
        dir.entries()?
    } else {
        dir.read_dir(base)?
    };
    for entry in read_dir {
        let entry = entry?;
        let name = entry.file_name();
        let name_string = name.to_string_lossy().into_owned();

        let rel = if base.as_os_str().is_empty() {
            PathBuf::from(&name)
        } else {
            base.join(&name)
        };

        // Follow symlinks (confined) to determine type and length.
        let meta = dir.metadata(&rel).ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
        let len = meta.as_ref().map_or(0, |m| m.len());

        entries.push(FsEntry {
            dir: dir.try_clone()?,
            rel: rel.clone(),
            display: display_base.join(&name),
            file_name: name_string,
            is_dir,
            len,
        });

        if is_dir {
            walk_dir(dir, &rel, &display_base.join(&name), entries)?;
        }
    }
    Ok(())
}

pub struct FileSystemService {
    allowed_dirs: RwLock<Arc<Vec<AllowedDir>>>,
}

impl FileSystemService {
    pub fn try_new(allowed_directories: &[String]) -> ServiceResult<Self> {
        let normalized_dirs: ServiceResult<Vec<AllowedDir>> = allowed_directories
            .iter()
            .map(fix_dockerhub_mcp_registry_gateway)
            .map(|dir| {
                let expand_result = expand_home(normalize_windows_drive_path(Path::new(dir)));
                if !expand_result.is_dir() {
                    return Err(ServiceError::InvalidConfig(format!(
                        "Error: The path `{dir}` is not a valid directory. Please double-check your server configuration to ensure the directory exists and is accessible."
                    )));
                }
                let canonical = expand_result.canonicalize().map_err(ServiceError::from)?;
                let dir = Dir::open_ambient_dir(&canonical, ambient_authority())
                    .map_err(ServiceError::from)?;
                Ok(AllowedDir {
                    path: canonical,
                    dir,
                })
            })
            .collect();

        Ok(Self {
            allowed_dirs: RwLock::new(Arc::new(normalized_dirs?)),
        })
    }

    pub async fn allowed_directories(&self) -> Arc<Vec<PathBuf>> {
        let guard = self.allowed_dirs.read().await;
        Arc::new(guard.iter().map(|ad| ad.path.clone()).collect())
    }

    pub async fn update_allowed_paths(&self, valid_roots: Vec<PathBuf>) {
        let allowed_dirs: Vec<AllowedDir> = valid_roots
            .into_iter()
            .filter_map(|root| {
                let canonical = root.canonicalize().ok()?;
                let dir = Dir::open_ambient_dir(&canonical, ambient_authority()).ok()?;
                Some(AllowedDir {
                    path: canonical,
                    dir,
                })
            })
            .collect();

        let mut guard = self.allowed_dirs.write().await;
        *guard = Arc::new(allowed_dirs);
    }

    /// Resolves `requested_path` to a confined directory handle and a relative
    /// path within it, or rejects it if it falls outside all allowed directories.
    ///
    /// The actual confinement is enforced by `cap-std`: the returned `Dir` can
    /// only access its own subtree, so a symlink (existing, dangling, or raced)
    /// cannot escape it.
    pub async fn resolve(&self, requested_path: &Path) -> ServiceResult<Resolved> {
        let allowed = self.allowed_dirs.read().await.clone();
        if allowed.is_empty() {
            return Err(ServiceError::FromString(
                "Allowed directories list is empty. Client did not provide any valid root directories.".to_string(),
            ));
        }

        // Expand ~ to home directory
        let expanded_path = expand_home(normalize_windows_drive_path(requested_path));

        // Resolve the absolute path
        let absolute_path = if expanded_path.is_absolute() {
            expanded_path
        } else {
            env::current_dir()
                .map_err(ServiceError::from)?
                .join(&expanded_path)
        };

        // Resolve to a canonical path. If the full path cannot be canonicalized
        // (e.g. it does not exist yet), canonicalize the deepest existing
        // ancestor and re-join the remaining suffix lexically. This mapping is
        // used only to select the root and relative path; the security boundary
        // is enforced by `cap-std` on the returned `Dir`.
        let canonical = match absolute_path.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => {
                let mut suffix: Vec<std::ffi::OsString> = Vec::new();
                let mut ancestor = absolute_path.as_path();
                let canonical_ancestor = loop {
                    match ancestor.canonicalize() {
                        Ok(canonical) => break canonical,
                        Err(_) => {
                            let file_name = ancestor.file_name().ok_or_else(|| {
                                ServiceError::FromString(
                                    "Invalid path: cannot resolve a non-existent path".into(),
                                )
                            })?;
                            suffix.push(file_name.to_os_string());
                            let parent = ancestor.parent().ok_or_else(|| {
                                ServiceError::FromString("Invalid path".into())
                            })?;
                            if parent == ancestor {
                                return Err(ServiceError::FromString(
                                    "Invalid path: cannot resolve a non-existent path".into(),
                                ));
                            }
                            ancestor = parent;
                        }
                    }
                };

                let mut result = canonical_ancestor;
                for component in suffix.iter().rev() {
                    result.push(component);
                }
                result
            }
        };

        for allowed_dir in allowed.iter() {
            if let Ok(rel) = canonical.strip_prefix(&allowed_dir.path) {
                // Defence-in-depth: reject unresolved parent directory components.
                if rel
                    .components()
                    .any(|c| c == Component::ParentDir)
                {
                    return Err(ServiceError::FromString(
                        "Path contains unresolved parent directory components".into(),
                    ));
                }

                return Ok(Resolved {
                    dir: allowed_dir.dir.try_clone().map_err(ServiceError::from)?,
                    rel: rel.to_path_buf(),
                    display: canonical,
                });
            }
        }

        Err(ServiceError::FromString(format!(
            "Access denied - path is outside allowed directories: {} not in {}",
            absolute_path.display(),
            allowed
                .iter()
                .map(|ad| ad.path.display().to_string())
                .collect::<Vec<_>>()
                .join(",\n"),
        )))
    }

    pub fn valid_roots(&self, roots: Vec<&str>) -> ServiceResult<(Vec<PathBuf>, Option<String>)> {
        let paths: Vec<Result<PathBuf, ServiceError>> =
            roots.iter().map(|p| parse_file_path(p)).collect::<Vec<_>>();

        // Partition into Ok and Err results
        let (ok_paths, err_paths): (PathResultList, PathResultList) =
            paths.into_iter().partition(|p| p.is_ok());

        // using HashSet to remove duplicates
        let (valid_roots, no_dir_roots): (HashSet<PathBuf>, HashSet<PathBuf>) = ok_paths
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(expand_home)
            .partition(|path| path.is_dir());

        let skipped_roots = if !err_paths.is_empty() || !no_dir_roots.is_empty() {
            Some(format!(
                "Warning: skipped {} invalid roots.",
                err_paths.len() + no_dir_roots.len()
            ))
        } else {
            None
        };

        let valid_roots = valid_roots.into_iter().collect();

        Ok((valid_roots, skipped_roots))
    }
}

/// This addresses the issue with the DockerHub mcp-registry & mcp-gateway where tool discovery fails to resolve
/// references to 'example' or 'default' values when running the run->command from the server.yaml file
/// should be removed once mcp-gateway is more mature
/// reference: https://github.com/docker/mcp-registry/blob/7d815fac2f3b7a9717eebc3f3db215de3ce3c3c7/internal/mcp/client.go#L170-L173
#[allow(clippy::ptr_arg)]
fn fix_dockerhub_mcp_registry_gateway(input: &String) -> &str {
    if input.contains("{{rust-mcp-filesystem.allowed_directories|volume-target|into}}") {
        "."
    } else {
        input
    }
}
