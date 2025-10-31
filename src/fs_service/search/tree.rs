use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::{FileSystemService, utils::is_system_metadata_file},
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde_json::{Value, json};
use std::{
    fs::{self},
    path::{Path, PathBuf},
    sync::Arc,
};
use walkdir::WalkDir;

impl FileSystemService {
    /// Generates a JSON representation of a directory tree starting at the given path.
    ///
    /// This function recursively builds a JSON array object representing the directory structure,
    /// where each entry includes a `name` (file or directory name), `type` ("file" or "directory"),
    /// and for directories, a `children` array containing their contents. Files do not have a
    /// `children` field.
    ///
    /// The function supports optional constraints to limit the tree size:
    /// - `max_depth`: Limits the depth of directory traversal.
    /// - `max_files`: Limits the total number of entries (files and directories).
    ///
    /// # IMPORTANT NOTE
    ///
    /// use max_depth or max_files could lead to partial or skewed representations of actual directory tree
    pub fn directory_tree<P: AsRef<Path>>(
        &self,
        root_path: P,
        max_depth: Option<usize>,
        max_files: Option<usize>,
        current_count: &mut usize,
        allowed_directories: Arc<Vec<PathBuf>>,
    ) -> ServiceResult<(Value, bool)> {
        let valid_path = self.validate_path(root_path.as_ref(), allowed_directories.clone())?;

        let metadata = fs::metadata(&valid_path)?;
        if !metadata.is_dir() {
            return Err(ServiceError::FromString(
                "Root path must be a directory".into(),
            ));
        }

        let mut children = Vec::new();
        let mut reached_max_depth = false;

        if max_depth != Some(0) {
            for entry in WalkDir::new(valid_path)
                .min_depth(1)
                .max_depth(1)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let child_path = entry.path();
                let metadata = fs::metadata(child_path)?;

                let entry_name = child_path
                    .file_name()
                    .ok_or(ServiceError::FromString("Invalid path".to_string()))?
                    .to_string_lossy()
                    .into_owned();

                // Increment the count for this entry
                *current_count += 1;

                // Check if we've exceeded max_files (if set)
                if let Some(max) = max_files {
                    if *current_count > max {
                        continue; // Skip this entry but continue processing others
                    }
                }

                let mut json_entry = json!({
                    "name": entry_name,
                    "type": if metadata.is_dir() { "directory" } else { "file" }
                });

                if metadata.is_dir() {
                    let next_depth = max_depth.map(|d| d - 1);
                    let (child_children, child_reached_max_depth) = self.directory_tree(
                        child_path,
                        next_depth,
                        max_files,
                        current_count,
                        allowed_directories.clone(),
                    )?;
                    json_entry
                        .as_object_mut()
                        .unwrap()
                        .insert("children".to_string(), child_children);
                    reached_max_depth |= child_reached_max_depth;
                }
                children.push(json_entry);
            }
        } else {
            // If max_depth is 0, we skip processing this directory's children
            reached_max_depth = true;
        }
        Ok((Value::Array(children), reached_max_depth))
    }

    /// Calculates the total size (in bytes) of all files within a directory tree.
    ///
    /// This function recursively searches the specified `root_path` for files,
    /// filters out directories and non-file entries, and sums the sizes of all found files.
    /// The size calculation is parallelized using Rayon for improved performance on large directories.
    ///
    /// # Arguments
    /// * `root_path` - The root directory path to start the size calculation.
    ///
    /// # Returns
    /// Returns a `ServiceResult<u64>` containing the total size in bytes of all files under the `root_path`.
    ///
    /// # Notes
    /// - Only files are included in the size calculation; directories and other non-file entries are ignored.
    /// - The search pattern is `"**/*"` (all files) and no exclusions are applied.
    /// - Parallel iteration is used to speed up the metadata fetching and summation.
    pub async fn calculate_directory_size(&self, root_path: &Path) -> ServiceResult<u64> {
        let entries = self
            .search_files_iter(root_path, "**/*".to_string(), vec![], None, None)
            .await?
            .filter(|e| e.file_type().is_file()); // Only process files

        // Use rayon to parallelize size summation
        let total_size: u64 = entries
            .par_bridge() // Convert to parallel iterator
            .filter_map(|entry| entry.metadata().ok().map(|meta| meta.len()))
            .sum();

        Ok(total_size)
    }

    /// Recursively finds all empty directories within the given root path.
    ///
    /// A directory is considered empty if it contains no files in itself or any of its subdirectories
    /// except OS metadata files: `.DS_Store` (macOS) and `Thumbs.db` (Windows)
    /// Empty subdirectories are allowed. You can optionally provide a list of glob-style patterns in
    /// `exclude_patterns` to ignore certain paths during the search (e.g., to skip system folders or hidden directories).
    ///
    /// # Arguments
    /// - `root_path`: The starting directory to search.
    /// - `exclude_patterns`: Optional list of glob patterns to exclude from the search.
    ///   Directories matching these patterns will be ignored.
    ///
    /// # Errors
    /// Returns an error if the root path is invalid or inaccessible.
    ///
    /// # Returns
    /// A list of paths to empty directories, as strings, including parent directories that contain only empty subdirectories.
    /// Recursively finds all empty directories within the given root path.
    ///
    /// A directory is considered empty if it contains no files in itself or any of its subdirectories.
    /// Empty subdirectories are allowed. You can optionally provide a list of glob-style patterns in
    /// `exclude_patterns` to ignore certain paths during the search (e.g., to skip system folders or hidden directories).
    ///
    /// # Arguments
    /// - `root_path`: The starting directory to search.
    /// - `exclude_patterns`: Optional list of glob patterns to exclude from the search.
    ///   Directories matching these patterns will be ignored.
    ///
    /// # Errors
    /// Returns an error if the root path is invalid or inaccessible.
    ///
    /// # Returns
    /// A list of paths to all empty directories, as strings, including parent directories that contain only empty subdirectories.
    pub async fn find_empty_directories(
        &self,
        root_path: &Path,
        exclude_patterns: Option<Vec<String>>,
    ) -> ServiceResult<Vec<String>> {
        let walker = self
            .search_files_iter(
                root_path,
                "**/*".to_string(),
                exclude_patterns.unwrap_or_default(),
                None,
                None,
            )
            .await?
            .filter(|e| e.file_type().is_dir()); // Only directories

        let mut empty_dirs = Vec::new();

        // Check each directory for emptiness
        for entry in walker {
            let is_empty = WalkDir::new(entry.path())
                .into_iter()
                .filter_map(|e| e.ok())
                .all(|e| !e.file_type().is_file() || is_system_metadata_file(e.file_name())); // Directory is empty if no files are found in it or subdirs, ".DS_Store" will be ignores on Mac

            if is_empty {
                if let Some(path_str) = entry.path().to_str() {
                    empty_dirs.push(path_str.to_string());
                }
            }
        }

        Ok(empty_dirs)
    }

    pub async fn list_directory(&self, dir_path: &Path) -> ServiceResult<Vec<tokio::fs::DirEntry>> {
        let allowed_directories = self.allowed_directories().await;

        let valid_path = self.validate_path(dir_path, allowed_directories)?;

        let mut dir = tokio::fs::read_dir(valid_path).await?;

        let mut entries = Vec::new();

        // Use a loop to collect the directory entries
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry);
        }

        Ok(entries)
    }
}
