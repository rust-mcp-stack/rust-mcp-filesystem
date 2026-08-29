use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::utils::{
        expand_home, is_unc_path, normalize_windows_drive_path, parse_file_path,
        strip_prefix_platform, strip_verbatim_prefix, trim_trailing_separator,
    },
};
use cap_std::{ambient_authority, fs::Dir};
use std::{
    collections::HashSet,
    env,
    ffi::OsString,
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
/// rather than by path arithmetic. For UNC roots, path-based operations use the
/// `unc_root` fallback (see [`Resolved`]).
pub struct AllowedDir {
    /// De-verbatimized canonical absolute path, used for prefix matching and
    /// display (e.g. `\\server\share\Movies`, never `\\?\UNC\...`).
    pub path: PathBuf,
    /// Verbatim canonical root (`\\?\UNC\server\share`) used only by the
    /// `std::fs` fallback operations for Windows UNC shares, preserving
    /// long-path support. `None` for local roots.
    pub unc_root: Option<PathBuf>,
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
    /// Verbatim canonical root when the allowed root is a Windows UNC share,
    /// used by the `std::fs` fallback operations (`None` for local roots).
    pub unc_root: Option<PathBuf>,
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

impl Resolved {
    /// Returns `true` when this path was resolved against a Windows UNC root.
    pub fn is_unc(&self) -> bool {
        self.unc_root.is_some()
    }

    /// The ambient absolute path used by the `std::fs` fallback operations.
    /// For UNC roots this is the verbatim canonical path (keeps long-path
    /// support); for local roots it falls back to the display path.
    fn fallback_abs(&self) -> PathBuf {
        match &self.unc_root {
            Some(root) => root.join(&self.rel),
            None => self.display.clone(),
        }
    }

    /// Verbatim canonical root, de-verbatimized, for containment checks.
    fn unc_root_clean(&self) -> Option<PathBuf> {
        self.unc_root
            .as_ref()
            .map(|root| strip_verbatim_prefix(root))
    }

    /// Lists the names of the entries directly under `base` (a path relative to
    /// the root). For UNC roots this uses ambient `std::fs` (cap-std cannot
    /// enumerate UNC directories); for local roots it uses the confined handle.
    pub fn read_dir_names(&self, base: &Path) -> io::Result<Vec<OsString>> {
        if let Some(root) = &self.unc_root {
            let abs = if base.as_os_str().is_empty() {
                root.clone()
            } else {
                root.join(base)
            };
            std::fs::read_dir(abs)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect()
        } else if base.as_os_str().is_empty() {
            self.dir
                .entries()?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect()
        } else {
            self.dir
                .read_dir(base)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect()
        }
    }

    /// Metadata for a path relative to the root. For UNC roots this uses
    /// `symlink_metadata` (no following) so a symlink can never escape the share.
    pub fn entry_metadata(&self, rel: &Path) -> io::Result<std::fs::Metadata> {
        if let Some(root) = &self.unc_root {
            std::fs::symlink_metadata(root.join(rel))
        } else {
            match self.dir.open(rel) {
                Ok(file) => file.into_std().metadata(),
                Err(_) => self.dir.open_dir(rel)?.into_std_file().metadata(),
            }
        }
    }

    /// Creates a directory tree (all missing parents) at `self.rel`.
    ///
    /// For UNC roots it uses `std::fs::create_dir_all` and then verifies the
    /// canonicalized result still lies within the allowed root, removing it and
    /// failing if it escaped (e.g. via a raced symlink).
    pub fn create_dir_all(&self) -> ServiceResult<()> {
        if let Some(root) = &self.unc_root {
            let abs = root.join(&self.rel);
            std::fs::create_dir_all(&abs)?;
            self.verify_unc_path(&abs, "Path escaped the allowed directory")?;
            Ok(())
        } else {
            self.dir.create_dir_all(&self.rel)?;
            Ok(())
        }
    }

    /// Creates a directory tree at `rel` (a path relative to the root); used by
    /// archive extraction.
    pub fn create_dir_all_rel(&self, rel: &Path) -> ServiceResult<()> {
        if let Some(root) = &self.unc_root {
            let abs = root.join(rel);
            std::fs::create_dir_all(&abs)?;
            self.verify_unc_path(&abs, "Path escaped the allowed directory")?;
            Ok(())
        } else {
            self.dir.create_dir_all(rel)?;
            Ok(())
        }
    }

    /// Renames this path to `dest`. Local roots use the confined cap-std handles;
    /// any UNC-involved move uses `std::fs::rename` with containment verification.
    pub fn rename_to(&self, dest: &Resolved) -> ServiceResult<()> {
        match (&self.unc_root, &dest.unc_root) {
            (None, None) => {
                self.dir.rename(&self.rel, &dest.dir, &dest.rel)?;
                Ok(())
            }
            _ => {
                let src_abs = self.fallback_abs();
                let dst_abs = dest.fallback_abs();
                if let Some(root) = &self.unc_root
                    && let Some(parent) = src_abs.parent()
                    && let Ok(canonical_parent) = std::fs::canonicalize(parent)
                {
                    let parent_clean = strip_verbatim_prefix(&canonical_parent);
                    let root_clean = strip_verbatim_prefix(root);
                    if strip_prefix_platform(&parent_clean, &root_clean).is_none() {
                        return Err(ServiceError::FromString(
                            "Source path escaped the allowed directory".into(),
                        ));
                    }
                }
                std::fs::rename(&src_abs, &dst_abs)?;
                dest.verify_unc_path(&dst_abs, "Destination path escaped the allowed directory")
            }
        }
    }

    /// Post-operation containment check for the ambient `std::fs` fallbacks:
    /// canonicalize `abs` and reject it if it falls outside the UNC root.
    fn verify_unc_path(&self, abs: &Path, message: &str) -> ServiceResult<()> {
        let Some(root) = self.unc_root_clean() else {
            return Ok(());
        };
        if let Ok(canonical) = std::fs::canonicalize(abs) {
            let canonical_clean = strip_verbatim_prefix(&canonical);
            if strip_prefix_platform(&canonical_clean, &root).is_none() {
                return Err(ServiceError::FromString(message.into()));
            }
        }
        Ok(())
    }
}

/// Recursively collects all entries (files and directories) under `resolved`'s
/// root into `entries`. Local roots are confined by the cap-std handle; UNC
/// roots use ambient enumeration with `symlink_metadata` (no-follow) so that a
/// symlinked directory can never be traversed outside the allowed share.
pub fn walk_dir(resolved: &Resolved, entries: &mut Vec<FsEntry>) -> io::Result<()> {
    walk_dir_inner(resolved, &resolved.rel, &resolved.display, entries)
}

fn walk_dir_inner(
    resolved: &Resolved,
    base: &Path,
    display_base: &Path,
    entries: &mut Vec<FsEntry>,
) -> io::Result<()> {
    let names = resolved.read_dir_names(base)?;
    for name in names {
        let name_string = name.to_string_lossy().into_owned();

        let rel = if base.as_os_str().is_empty() {
            PathBuf::from(&name)
        } else {
            base.join(&name)
        };

        let meta = resolved.entry_metadata(&rel).ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
        let len = meta.as_ref().map_or(0, |m| m.len());

        entries.push(FsEntry {
            dir: resolved.dir.try_clone()?,
            rel: rel.clone(),
            display: display_base.join(&name),
            file_name: name_string,
            is_dir,
            len,
        });

        if is_dir {
            walk_dir_inner(resolved, &rel, &display_base.join(&name), entries)?;
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
                let path = trim_trailing_separator(strip_verbatim_prefix(&canonical));
                let unc_root = is_unc_path(&canonical).then(|| canonical.clone());
                let dir = Dir::open_ambient_dir(&canonical, ambient_authority())
                    .map_err(ServiceError::from)?;
                Ok(AllowedDir {
                    path,
                    unc_root,
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
                let path = trim_trailing_separator(strip_verbatim_prefix(&canonical));
                let unc_root = is_unc_path(&canonical).then(|| canonical.clone());
                let dir = Dir::open_ambient_dir(&canonical, ambient_authority()).ok()?;
                Some(AllowedDir {
                    path,
                    unc_root,
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
                            let parent = ancestor
                                .parent()
                                .ok_or_else(|| ServiceError::FromString("Invalid path".into()))?;
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

        // De-verbatimize so matching/display use `\\server\share`, not `\\?\UNC\...`.
        let clean = strip_verbatim_prefix(&canonical);

        for allowed_dir in allowed.iter() {
            if let Some(rel) = strip_prefix_platform(&clean, &allowed_dir.path) {
                // Defence-in-depth: reject unresolved parent directory components.
                if rel.components().any(|c| c == Component::ParentDir) {
                    return Err(ServiceError::FromString(
                        "Path contains unresolved parent directory components".into(),
                    ));
                }

                return Ok(Resolved {
                    dir: allowed_dir.dir.try_clone().map_err(ServiceError::from)?,
                    rel,
                    display: clean,
                    unc_root: allowed_dir.unc_root.clone(),
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
