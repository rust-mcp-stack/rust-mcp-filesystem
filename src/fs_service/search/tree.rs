use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::{FileSystemService, FsEntry, utils::is_system_metadata_file, walk_dir},
};
use glob_match::glob_match;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn is_excluded(exclude_patterns: &[String], rel: &Path) -> bool {
    exclude_patterns.iter().any(|pattern| {
        let glob = if pattern.contains('*') {
            pattern.strip_prefix('/').unwrap_or(pattern).to_owned()
        } else {
            format!("*{pattern}*")
        };
        glob_match(&glob, rel.to_str().unwrap_or(""))
    })
}

impl FileSystemService {
    /// Generates a JSON representation of a directory tree starting at the given path.
    ///
    /// The function supports optional constraints to limit the tree size:
    /// - `max_depth`: Limits the depth of directory traversal.
    /// - `max_files`: Limits the total number of entries (files and directories).
    pub async fn directory_tree<P: AsRef<Path>>(
        &self,
        root_path: P,
        max_depth: Option<usize>,
        max_files: Option<usize>,
        current_count: &mut usize,
    ) -> ServiceResult<(Value, bool)> {
        let resolved = self.resolve(root_path.as_ref()).await?;

        let metadata = if resolved.rel.as_os_str().is_empty() {
            resolved.dir.dir_metadata()?
        } else {
            resolved.dir.metadata(&resolved.rel)?
        };
        if !metadata.is_dir() {
            return Err(ServiceError::FromString(
                "Root path must be a directory".into(),
            ));
        }

        let (children, reached_max_depth) = self.build_tree(
            &resolved,
            &resolved.rel,
            max_depth,
            max_files,
            current_count,
        )?;
        Ok((Value::Array(children), reached_max_depth))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn build_tree(
        &self,
        resolved: &crate::fs_service::Resolved,
        rel: &Path,
        max_depth: Option<usize>,
        max_files: Option<usize>,
        current_count: &mut usize,
    ) -> ServiceResult<(Vec<Value>, bool)> {
        let mut children = Vec::new();
        let mut reached_max_depth = false;

        if max_depth == Some(0) {
            return Ok((children, true));
        }

        let names = resolved.read_dir_names(rel)?;
        for name in names {
            let entry_name = name.to_string_lossy().into_owned();

            let child_rel = if rel.as_os_str().is_empty() {
                PathBuf::from(&name)
            } else {
                rel.join(&name)
            };

            let child_meta = resolved.entry_metadata(&child_rel).ok();
            let is_dir = child_meta.as_ref().is_some_and(|m| m.is_dir());

            // Increment the count for this entry
            *current_count += 1;

            // Check if we've exceeded max_files (if set)
            if let Some(max) = max_files
                && *current_count > max
            {
                continue;
            }

            let mut json_entry = json!({
                "name": entry_name,
                "type": if is_dir { "directory" } else { "file" }
            });

            if is_dir {
                let next_depth = max_depth.map(|d| d - 1);
                let (child_children, child_reached_max_depth) =
                    self.build_tree(resolved, &child_rel, next_depth, max_files, current_count)?;
                json_entry
                    .as_object_mut()
                    .unwrap()
                    .insert("children".to_string(), Value::Array(child_children));
                reached_max_depth |= child_reached_max_depth;
            }
            children.push(json_entry);
        }

        Ok((children, reached_max_depth))
    }

    /// Calculates the total size (in bytes) of all files within a directory tree.
    pub async fn calculate_directory_size(&self, root_path: &Path) -> ServiceResult<u64> {
        let entries = self
            .search_files_iter(root_path, "**/*".to_string(), vec![], None, None)
            .await?;

        let total_size: u64 = entries
            .into_iter()
            .filter(|e| e.is_file())
            .map(|e| e.len)
            .sum();

        Ok(total_size)
    }

    /// Recursively finds all empty directories within the given root path.
    pub async fn find_empty_directories(
        &self,
        root_path: &Path,
        exclude_patterns: Option<Vec<String>>,
    ) -> ServiceResult<Vec<String>> {
        let resolved = self.resolve(root_path).await?;

        let exclude_patterns = exclude_patterns.unwrap_or_default();
        let mut entries = Vec::new();
        walk_dir(&resolved, &mut entries)?;

        let mut empty_dirs = Vec::new();

        for entry in &entries {
            if !entry.is_dir {
                continue;
            }

            // Relative path from the search root for exclusion matching.
            let rel_from_root = entry.rel.strip_prefix(&resolved.rel).unwrap_or(&entry.rel);
            if is_excluded(&exclude_patterns, rel_from_root) {
                continue;
            }

            // A directory is empty if no non-metadata file is a strict descendant.
            let has_file = entries.iter().any(|f| {
                !f.is_dir
                    && f.rel.starts_with(&entry.rel)
                    && f.rel != entry.rel
                    && !is_system_metadata_file(std::ffi::OsStr::new(f.file_name.as_str()))
            });

            if !has_file && let Some(path_str) = entry.display.to_str() {
                empty_dirs.push(path_str.to_string());
            }
        }

        Ok(empty_dirs)
    }

    pub async fn list_directory(&self, dir_path: &Path) -> ServiceResult<Vec<FsEntry>> {
        let resolved = self.resolve(dir_path).await?;

        let mut entries = Vec::new();
        let names = resolved.read_dir_names(&resolved.rel)?;

        for name in names {
            let name_string = name.to_string_lossy().into_owned();

            let rel = if resolved.rel.as_os_str().is_empty() {
                PathBuf::from(&name)
            } else {
                resolved.rel.join(&name)
            };

            let meta = resolved.entry_metadata(&rel).ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
            let len = meta.as_ref().map_or(0, |m| m.len());

            entries.push(FsEntry {
                dir: resolved.dir.try_clone()?,
                rel,
                display: resolved.display.join(&name),
                file_name: name_string,
                is_dir,
                len,
            });
        }

        Ok(entries)
    }
}
