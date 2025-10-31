use crate::{
    error::ServiceResult,
    fs_service::{FileSystemService, utils::filesize_in_range},
};
use glob_match::glob_match;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
use tokio::{fs::File, io::AsyncReadExt};
use walkdir::WalkDir;

impl FileSystemService {
    /// Searches for files in the directory tree starting at `root_path` that match the given `pattern`,
    /// excluding paths that match any of the `exclude_patterns`.
    ///
    /// # Arguments
    /// * `root_path` - The root directory to start the search from.
    /// * `pattern` - A glob pattern to match file names (case-insensitive). If no wildcards are provided,
    ///   the pattern is wrapped in '*' for partial matching.
    /// * `exclude_patterns` - A list of glob patterns to exclude paths (case-sensitive).
    ///
    /// # Returns
    /// A `ServiceResult` containing a vector of`walkdir::DirEntry` objects for matching files,
    /// or a `ServiceError` if an error occurs.
    pub async fn search_files(
        &self,
        root_path: &Path,
        pattern: String,
        exclude_patterns: Vec<String>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<walkdir::DirEntry>> {
        let result = self
            .search_files_iter(root_path, pattern, exclude_patterns, min_bytes, max_bytes)
            .await?;
        Ok(result.collect::<Vec<walkdir::DirEntry>>())
    }

    /// Returns an iterator over files in the directory tree starting at `root_path` that match
    /// the given `pattern`, excluding paths that match any of the `exclude_patterns`.
    ///
    /// # Arguments
    /// * `root_path` - The root directory to start the search from.
    /// * `pattern` - A glob pattern to match file names. If no wildcards are provided, the pattern is wrapped in `**/*{pattern}*` for partial matching.
    /// * `exclude_patterns` - A list of glob patterns to exclude paths (case-sensitive).
    ///
    /// # Returns
    /// A `ServiceResult` containing an iterator yielding `walkdir::DirEntry` objects for matching files,
    /// or a `ServiceError` if an error occurs.
    pub async fn search_files_iter<'a>(
        &'a self,
        // root_path: impl Into<PathBuf>,
        root_path: &'a Path,
        pattern: String,
        exclude_patterns: Vec<String>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<impl Iterator<Item = walkdir::DirEntry> + 'a> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(root_path, allowed_directories.clone())?;

        let updated_pattern = if pattern.contains('*') {
            pattern.to_lowercase()
        } else {
            format!("**/*{}*", &pattern.to_lowercase())
        };
        let glob_pattern = updated_pattern;

        let result = WalkDir::new(valid_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(move |dir_entry| {
                let full_path = dir_entry.path();

                // Validate each path before processing
                let validated_path = self
                    .validate_path(full_path, allowed_directories.clone())
                    .ok();

                if validated_path.is_none() {
                    // Skip invalid paths during search
                    return false;
                }

                // Get the relative path from the root_path
                let relative_path = full_path.strip_prefix(root_path).unwrap_or(full_path);

                let mut should_exclude = exclude_patterns.iter().any(|pattern| {
                    let glob_pattern = if pattern.contains('*') {
                        pattern.strip_prefix("/").unwrap_or(pattern).to_owned()
                    } else {
                        format!("*{pattern}*")
                    };

                    glob_match(&glob_pattern, relative_path.to_str().unwrap_or(""))
                });

                // enforce min/max bytes
                if !should_exclude && (min_bytes.is_none() || max_bytes.is_none()) {
                    match dir_entry.metadata().ok() {
                        Some(metadata) => {
                            if !filesize_in_range(metadata.len(), min_bytes, max_bytes) {
                                should_exclude = true;
                            }
                        }
                        None => {
                            should_exclude = true;
                        }
                    }
                }

                !should_exclude
            })
            .filter_map(|v| v.ok())
            .filter(move |entry| {
                if root_path == entry.path() {
                    return false;
                }

                glob_match(
                    &glob_pattern,
                    &entry.file_name().to_str().unwrap_or("").to_lowercase(),
                )
            });

        Ok(result)
    }

    /// Finds groups of duplicate files within the given root path.
    /// Returns a vector of vectors, where each inner vector contains paths to files with identical content.
    /// Files are considered duplicates if they have the same size and SHA-256 hash.
    pub async fn find_duplicate_files(
        &self,
        root_path: &Path,
        pattern: Option<String>,
        exclude_patterns: Option<Vec<String>>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<Vec<String>>> {
        // Validate root path against allowed directories
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(root_path, allowed_directories)?;

        // Get Tokio runtime handle
        let rt = tokio::runtime::Handle::current();

        // Step 1: Collect files and group by size
        let mut size_map: HashMap<u64, Vec<String>> = HashMap::new();
        let entries = self
            .search_files_iter(
                &valid_path,
                pattern.unwrap_or("**/*".to_string()),
                exclude_patterns.unwrap_or_default(),
                min_bytes,
                max_bytes,
            )
            .await?
            .filter(|e| e.file_type().is_file()); // Only files

        for entry in entries {
            if let Ok(metadata) = entry.metadata() {
                if let Some(path_str) = entry.path().to_str() {
                    size_map
                        .entry(metadata.len())
                        .or_default()
                        .push(path_str.to_string());
                }
            }
        }

        // Filter out sizes with only one file (no duplicates possible)
        let size_groups: Vec<Vec<String>> = size_map
            .into_iter()
            .collect::<Vec<_>>() // Collect into Vec to enable parallel iteration
            .into_par_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|(_, paths)| paths)
            .collect();

        // Step 2: Group by quick hash (first 4KB)
        let mut quick_hash_map: HashMap<Vec<u8>, Vec<String>> = HashMap::new();
        for paths in size_groups.into_iter() {
            let quick_hashes: Vec<(String, Vec<u8>)> = paths
                .into_par_iter()
                .filter_map(|path| {
                    let rt = rt.clone(); // Clone the runtime handle for this task
                    rt.block_on(async {
                        let file = File::open(&path).await.ok()?;
                        let mut reader = tokio::io::BufReader::new(file);
                        let mut buffer = vec![0u8; 4096]; // Read first 4KB
                        let bytes_read = reader.read(&mut buffer).await.ok()?;
                        let mut hasher = Sha256::new();
                        hasher.update(&buffer[..bytes_read]);
                        Some((path, hasher.finalize().to_vec()))
                    })
                })
                .collect();

            for (path, hash) in quick_hashes {
                quick_hash_map.entry(hash).or_default().push(path);
            }
        }

        // Step 3: Group by full hash for groups with multiple files
        let mut full_hash_map: HashMap<Vec<u8>, Vec<String>> = HashMap::new();
        let filtered_quick_hashes: Vec<(Vec<u8>, Vec<String>)> = quick_hash_map
            .into_iter()
            .collect::<Vec<_>>()
            .into_par_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .collect();

        for (_quick_hash, paths) in filtered_quick_hashes {
            let full_hashes: Vec<(String, Vec<u8>)> = paths
                .into_par_iter()
                .filter_map(|path| {
                    let rt = rt.clone(); // Clone the runtime handle for this task
                    rt.block_on(async {
                        let file = File::open(&path).await.ok()?;
                        let mut reader = tokio::io::BufReader::new(file);
                        let mut hasher = Sha256::new();
                        let mut buffer = vec![0u8; 8192]; // 8KB chunks
                        loop {
                            let bytes_read = reader.read(&mut buffer).await.ok()?;
                            if bytes_read == 0 {
                                break;
                            }
                            hasher.update(&buffer[..bytes_read]);
                        }
                        Some((path, hasher.finalize().to_vec()))
                    })
                })
                .collect();

            for (path, hash) in full_hashes {
                full_hash_map.entry(hash).or_default().push(path);
            }
        }

        // Collect groups of duplicates (only groups with more than one file)
        let duplicates: Vec<Vec<String>> = full_hash_map
            .into_values()
            .filter(|group| group.len() > 1)
            .collect();

        Ok(duplicates)
    }
}
