use crate::{
    error::ServiceResult,
    fs_service::{FileSystemService, FsEntry, utils::filesize_in_range, walk_dir},
};
use glob_match::glob_match;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io::Read, path::Path};

impl FileSystemService {
    /// Searches for files in the directory tree starting at `root_path` that match the given `pattern`,
    /// excluding paths that match any of the `exclude_patterns`.
    pub async fn search_files(
        &self,
        root_path: &Path,
        pattern: String,
        exclude_patterns: Vec<String>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<FsEntry>> {
        self.search_files_iter(root_path, pattern, exclude_patterns, min_bytes, max_bytes)
            .await
    }

    /// Returns files and directories in the tree rooted at `root_path` matching
    /// the given `pattern`, excluding paths matching `exclude_patterns`.
    ///
    /// Traversal is confined to the allowed directory by `cap-std`, so symlink
    /// escapes are never followed.
    pub async fn search_files_iter(
        &self,
        root_path: &Path,
        pattern: String,
        exclude_patterns: Vec<String>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<FsEntry>> {
        let resolved = self.resolve(root_path).await?;

        let updated_pattern = if pattern.contains('*') {
            pattern.to_lowercase()
        } else {
            format!("**/*{}*", &pattern.to_lowercase())
        };
        let glob_pattern = updated_pattern;

        // Confined recursive traversal.
        let mut all_entries = Vec::new();
        walk_dir(&resolved.dir, &resolved.rel, &resolved.display, &mut all_entries)?;

        let mut result = Vec::new();
        for entry in all_entries {
            // Compute the path relative to the search root for exclusion matching.
            let relative_path = entry.rel.strip_prefix(&resolved.rel).unwrap_or(&entry.rel);

            let should_exclude = exclude_patterns.iter().any(|pattern| {
                let glob_pattern = if pattern.contains('*') {
                    pattern.strip_prefix('/').unwrap_or(pattern).to_owned()
                } else {
                    format!("*{pattern}*")
                };
                glob_match(&glob_pattern, relative_path.to_str().unwrap_or(""))
            });

            if should_exclude {
                continue;
            }

            // Enforce min/max bytes for files.
            if !entry.is_dir
                && (min_bytes.is_some() || max_bytes.is_some())
                && !filesize_in_range(entry.len, min_bytes, max_bytes)
            {
                continue;
            }

            if !glob_match(&glob_pattern, &entry.file_name.to_lowercase()) {
                continue;
            }

            result.push(entry);
        }

        Ok(result)
    }

    /// Finds groups of duplicate files within the given root path.
    pub async fn find_duplicate_files(
        &self,
        root_path: &Path,
        pattern: Option<String>,
        exclude_patterns: Option<Vec<String>>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<Vec<String>>> {
        let entries = self
            .search_files_iter(
                root_path,
                pattern.unwrap_or("**/*".to_string()),
                exclude_patterns.unwrap_or_default(),
                min_bytes,
                max_bytes,
            )
            .await?;

        // Step 1: Group files by size.
        let mut size_map: HashMap<u64, Vec<FsEntry>> = HashMap::new();
        for entry in entries.into_iter().filter(|e| e.is_file()) {
            size_map.entry(entry.len).or_default().push(entry);
        }

        // Filter out sizes with only one file.
        let mut duplicate_groups: Vec<Vec<String>> = Vec::new();
        for (_size, mut group) in size_map.into_iter().filter(|(_, g)| g.len() > 1) {
            // Step 2: Group by SHA-256 of the first 4KB.
            let mut quick_map: HashMap<Vec<u8>, Vec<FsEntry>> = HashMap::new();
            for entry in group.drain(..) {
                let hash = match quick_hash(&entry) {
                    Some(h) => h,
                    None => continue,
                };
                quick_map.entry(hash).or_default().push(entry);
            }

            // Step 3: For groups with multiple files, group by full hash.
            for (_quick, mut quick_group) in quick_map.into_iter().filter(|(_, g)| g.len() > 1) {
                let mut full_map: HashMap<Vec<u8>, Vec<String>> = HashMap::new();
                for entry in quick_group.drain(..) {
                    if let (Some(hash), Some(path)) = (full_hash(&entry), entry.display.to_str()) {
                        full_map.entry(hash).or_default().push(path.to_string());
                    }
                }
                for (_hash, paths) in full_map.into_iter().filter(|(_, p)| p.len() > 1) {
                    duplicate_groups.push(paths);
                }
            }
        }

        Ok(duplicate_groups)
    }
}

fn quick_hash(entry: &FsEntry) -> Option<Vec<u8>> {
    let mut file = entry.open().ok()?.into_std();
    let mut buffer = vec![0u8; 4096];
    let bytes_read = file.read(&mut buffer).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&buffer[..bytes_read]);
    Some(hasher.finalize().to_vec())
}

fn full_hash(entry: &FsEntry) -> Option<Vec<u8>> {
    let mut file = entry.open().ok()?.into_std();
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Some(hasher.finalize().to_vec())
}
