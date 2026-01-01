use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::utils::{contains_symlink, expand_home, normalize_path, parse_file_path},
};
use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;

type PathResultList = Vec<Result<PathBuf, ServiceError>>;

pub struct FileSystemService {
    allowed_path: RwLock<Arc<Vec<PathBuf>>>,
}

impl FileSystemService {
    pub fn try_new(allowed_directories: &[String]) -> ServiceResult<Self> {
        let normalized_dirs: Vec<PathBuf> = allowed_directories
            .iter()
            .map(fix_dockerhub_mcp_registry_gateway)
            .map_while(|dir| {
                let expand_result = expand_home(dir.into());
                if !expand_result.is_dir() {
                    panic!("{}", format!("Error: {dir} is not a directory"));
                }
                Some(expand_result)
            })
            .collect();

        Ok(Self {
            allowed_path: RwLock::new(Arc::new(normalized_dirs)),
        })
    }

    pub async fn allowed_directories(&self) -> Arc<Vec<PathBuf>> {
        let guard = self.allowed_path.read().await;
        guard.clone()
    }

    pub async fn update_allowed_paths(&self, valid_roots: Vec<PathBuf>) {
        let mut guard = self.allowed_path.write().await;
        *guard = Arc::new(valid_roots)
    }

    pub fn validate_path(
        &self,
        requested_path: &Path,
        allowed_directories: Arc<Vec<PathBuf>>,
    ) -> ServiceResult<PathBuf> {
        if allowed_directories.is_empty() {
            return Err(ServiceError::FromString(
                "Allowed directories list is empty. Client did not provide any valid root directories.".to_string()
            ));
        }

        // Expand ~ to home directory
        let expanded_path = expand_home(requested_path.to_path_buf());

        // Resolve the absolute path
        let absolute_path = if expanded_path.as_path().is_absolute() {
            expanded_path.clone()
        } else {
            env::current_dir().unwrap().join(&expanded_path)
        };

        // Normalize the path
        let normalized_requested = normalize_path(&absolute_path);

        // Check if path is within allowed directories
        if !allowed_directories.iter().any(|dir| {
            // Must account for both scenarios - the requested path may not exist yet, making canonicalization impossible.
            normalized_requested.starts_with(dir)
                || normalized_requested.starts_with(normalize_path(dir))
        }) {
            let symlink_target = if contains_symlink(&absolute_path)? {
                "a symlink target path"
            } else {
                "path"
            };
            return Err(ServiceError::FromString(format!(
                "Access denied - {} is outside allowed directories: {} not in {}",
                symlink_target,
                absolute_path.display(),
                allowed_directories
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(",\n"),
            )));
        }

        Ok(absolute_path)
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
