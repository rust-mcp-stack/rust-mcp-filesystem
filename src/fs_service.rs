pub mod file_info;
pub mod utils;
use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::utils::is_system_metadata_file,
    tools::EditOperation,
};
use async_zip::tokio::{read::seek::ZipFileReader, write::ZipFileWriter};
use base64::{engine::general_purpose, write::EncoderWriter};
use file_info::FileInfo;
use futures::{StreamExt, stream};
use glob::Pattern;
use grep::{
    matcher::{Match, Matcher},
    regex::RegexMatcherBuilder,
    searcher::{BinaryDetection, Searcher, sinks::UTF8},
};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use rust_mcp_sdk::schema::RpcError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use similar::TextDiff;
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self},
    io::{SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{File, metadata},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    sync::RwLock,
};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use utils::{
    contains_symlink, expand_home, format_bytes, normalize_line_endings, normalize_path,
    write_zip_entry,
};
use walkdir::WalkDir;

const SNIPPET_MAX_LENGTH: usize = 200;
const SNIPPET_BACKWARD_CHARS: usize = 30;
const MAX_CONCURRENT_FILE_READ: usize = 5;

#[cfg(windows)]
pub const OS_LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
pub const OS_LINE_ENDING: &str = "\n";

type PathResultList = Vec<Result<PathBuf, ServiceError>>;

pub struct FileSystemService {
    allowed_path: RwLock<Arc<Vec<PathBuf>>>,
}

/// Represents a single match found in a file's content.
#[derive(Debug, Clone)]
pub struct ContentMatchResult {
    /// The line number where the match occurred (1-based).
    pub line_number: u64,
    pub start_pos: usize,
    /// The line of text containing the match.
    /// If the line exceeds 255 characters (excluding the search term), only a truncated portion will be shown.
    pub line_text: String,
}

/// Represents all matches found in a specific file.
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    /// The path to the file where matches were found.
    pub file_path: PathBuf,
    /// All individual match results within the file.
    pub matches: Vec<ContentMatchResult>,
}

impl FileSystemService {
    pub fn try_new(allowed_directories: &[String]) -> ServiceResult<Self> {
        let normalized_dirs: Vec<PathBuf> = allowed_directories
            .iter()
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
}

impl FileSystemService {
    pub fn valid_roots(&self, roots: Vec<&str>) -> ServiceResult<(Vec<PathBuf>, Option<String>)> {
        let paths: Vec<Result<PathBuf, ServiceError>> = roots
            .iter()
            .map(|p| self.parse_file_path(p))
            .collect::<Vec<_>>();

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

    pub async fn update_allowed_paths(&self, valid_roots: Vec<PathBuf>) {
        let mut guard = self.allowed_path.write().await;
        *guard = Arc::new(valid_roots)
    }

    /// Converts a string to a `PathBuf`, supporting both raw paths and `file://` URIs.
    fn parse_file_path(&self, input: &str) -> ServiceResult<PathBuf> {
        Ok(PathBuf::from(
            input.strip_prefix("file://").unwrap_or(input).trim(),
        ))
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
            // Must account for both scenarios — the requested path may not exist yet, making canonicalization impossible.
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

    // Get file stats
    pub async fn get_file_stats(&self, file_path: &Path) -> ServiceResult<FileInfo> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;

        let metadata = fs::metadata(valid_path)?;

        let size = metadata.len();
        let created = metadata.created().ok();
        let modified = metadata.modified().ok();
        let accessed = metadata.accessed().ok();
        let is_directory = metadata.is_dir();
        let is_file = metadata.is_file();

        Ok(FileInfo {
            size,
            created,
            modified,
            accessed,
            is_directory,
            is_file,
            metadata,
        })
    }

    fn detect_line_ending(&self, text: &str) -> &str {
        if text.contains("\r\n") {
            "\r\n"
        } else if text.contains('\r') {
            "\r"
        } else {
            "\n"
        }
    }

    pub async fn zip_directory(
        &self,
        input_dir: String,
        pattern: String,
        target_zip_file: String,
    ) -> ServiceResult<String> {
        let allowed_directories = self.allowed_directories().await;
        let valid_dir_path =
            self.validate_path(Path::new(&input_dir), allowed_directories.clone())?;

        let input_dir_str = &valid_dir_path
            .as_os_str()
            .to_str()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid UTF-8 in file name",
            ))?;

        let target_path =
            self.validate_path(Path::new(&target_zip_file), allowed_directories.clone())?;

        if target_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_zip_file}' already exists!"),
            )
            .into());
        }

        let updated_pattern = if pattern.contains('*') {
            pattern.to_lowercase()
        } else {
            format!("*{}*", &pattern.to_lowercase())
        };

        let glob_pattern = Pattern::new(&updated_pattern)?;

        let entries: Vec<_> = WalkDir::new(&valid_dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let full_path = entry.path();

                self.validate_path(full_path, allowed_directories.clone())
                    .ok()
                    .and_then(|path| {
                        if path != valid_dir_path
                            && glob_pattern.matches(&path.display().to_string())
                        {
                            Some(path)
                        } else {
                            None
                        }
                    })
            })
            .collect();

        let zip_file = File::create(&target_path).await?;
        let mut zip_writer = ZipFileWriter::new(zip_file.compat());

        for entry_path_buf in &entries {
            if entry_path_buf.is_dir() {
                continue;
            }
            let entry_path = entry_path_buf.as_path();
            let entry_str = entry_path.as_os_str().to_str().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid UTF-8 in file name",
            ))?;

            if !entry_str.starts_with(input_dir_str) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Entry file path does not start with base input directory path.",
                )
                .into());
            }

            let entry_str = &entry_str[input_dir_str.len() + 1..];
            write_zip_entry(entry_str, entry_path, &mut zip_writer).await?;
        }

        let z_file = zip_writer.close().await?;
        let zip_file_size = if let Ok(meta_data) = z_file.into_inner().metadata().await {
            format_bytes(meta_data.len())
        } else {
            "unknown".to_string()
        };
        let result_message = format!(
            "Successfully compressed '{}' directory into '{}' ({}).",
            input_dir,
            target_path.display(),
            zip_file_size
        );
        Ok(result_message)
    }

    pub async fn zip_files(
        &self,
        input_files: Vec<String>,
        target_zip_file: String,
    ) -> ServiceResult<String> {
        let file_count = input_files.len();

        if file_count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No file(s) to zip. The input files array is empty.",
            )
            .into());
        }
        let allowed_directories = self.allowed_directories().await;
        let target_path =
            self.validate_path(Path::new(&target_zip_file), allowed_directories.clone())?;

        if target_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_zip_file}' already exists!"),
            )
            .into());
        }

        let source_paths = input_files
            .iter()
            .map(|p| self.validate_path(Path::new(p), allowed_directories.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let zip_file = File::create(&target_path).await?;
        let mut zip_writer = ZipFileWriter::new(zip_file.compat());
        for path in source_paths {
            let filename = path.file_name().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid path!",
            ))?;

            let filename = filename.to_str().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid UTF-8 in file name",
            ))?;

            write_zip_entry(filename, &path, &mut zip_writer).await?;
        }
        let z_file = zip_writer.close().await?;

        let zip_file_size = if let Ok(meta_data) = z_file.into_inner().metadata().await {
            format_bytes(meta_data.len())
        } else {
            "unknown".to_string()
        };

        let result_message = format!(
            "Successfully compressed {} {} into '{}' ({}).",
            file_count,
            if file_count == 1 { "file" } else { "files" },
            target_path.display(),
            zip_file_size
        );
        Ok(result_message)
    }

    pub async fn unzip_file(&self, zip_file: &str, target_dir: &str) -> ServiceResult<String> {
        let allowed_directories = self.allowed_directories().await;

        let zip_file = self.validate_path(Path::new(&zip_file), allowed_directories.clone())?;
        let target_dir_path = self.validate_path(Path::new(target_dir), allowed_directories)?;
        if !zip_file.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Zip file does not exists.",
            )
            .into());
        }

        if target_dir_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_dir}' directory already exists!"),
            )
            .into());
        }

        let file = BufReader::new(File::open(zip_file).await?);
        let mut zip = ZipFileReader::with_tokio(file).await?;

        let file_count = zip.file().entries().len();

        for index in 0..file_count {
            let entry = zip.file().entries().get(index).unwrap();
            let entry_path = target_dir_path.join(entry.filename().as_str()?);
            // Ensure the parent directory exists
            if let Some(parent) = entry_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            // Extract the file
            let reader = zip.reader_without_entry(index).await?;
            let mut compat_reader = reader.compat();
            let mut output_file = File::create(&entry_path).await?;

            tokio::io::copy(&mut compat_reader, &mut output_file).await?;
            output_file.flush().await?;
        }

        let result_message = format!(
            "Successfully extracted {} {} into '{}'.",
            file_count,
            if file_count == 1 { "file" } else { "files" },
            target_dir_path.display()
        );

        Ok(result_message)
    }

    pub fn mime_from_path(&self, path: &Path) -> ServiceResult<infer::Type> {
        let is_svg = path
            .extension()
            .is_some_and(|e| e.to_str().is_some_and(|s| s == "svg"));
        // consider it is a svg file as we cannot detect svg from bytes pattern
        if is_svg {
            return Ok(infer::Type::new(
                infer::MatcherType::Image,
                "image/svg+xml",
                "svg",
                |_: &[u8]| true,
            ));

            // infer::Type::new(infer::MatcherType::Image, "", "svg",);
        }
        let kind = infer::get_from_path(path)?.ok_or(ServiceError::FromString(
            "File tyle is unknown!".to_string(),
        ))?;
        Ok(kind)
    }

    pub fn filesize_in_range(
        &self,
        file_size: u64,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> bool {
        if min_bytes.is_none() && max_bytes.is_none() {
            return true;
        }
        match (min_bytes, max_bytes) {
            (_, Some(max)) if file_size > max => false,
            (Some(min), _) if file_size < min => false,
            _ => true,
        }
    }

    pub async fn validate_file_size<P: AsRef<Path>>(
        &self,
        path: P,
        min_bytes: Option<usize>,
        max_bytes: Option<usize>,
    ) -> ServiceResult<()> {
        if min_bytes.is_none() && max_bytes.is_none() {
            return Ok(());
        }

        let file_size = metadata(&path).await?.len() as usize;

        match (min_bytes, max_bytes) {
            (_, Some(max)) if file_size > max => Err(ServiceError::FileTooLarge(max)),
            (Some(min), _) if file_size < min => Err(ServiceError::FileTooSmall(min)),
            _ => Ok(()),
        }
    }

    pub async fn read_media_files(
        &self,
        paths: Vec<String>,
        max_bytes: Option<usize>,
    ) -> ServiceResult<Vec<(infer::Type, String)>> {
        let results = stream::iter(paths)
            .map(|path| async {
                self.read_media_file(Path::new(&path), max_bytes)
                    .await
                    .map_err(|e| (path, e))
            })
            .buffer_unordered(MAX_CONCURRENT_FILE_READ) // Process up to MAX_CONCURRENT_FILE_READ files concurrently
            .filter_map(|result| async move { result.ok() })
            .collect::<Vec<_>>()
            .await;
        Ok(results)
    }

    pub async fn read_media_file(
        &self,
        file_path: &Path,
        max_bytes: Option<usize>,
    ) -> ServiceResult<(infer::Type, String)> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;
        self.validate_file_size(&valid_path, None, max_bytes)
            .await?;
        let kind = self.mime_from_path(&valid_path)?;
        let content = self.read_file_as_base64(&valid_path).await?;
        Ok((kind, content))
    }

    // reads file as base64 efficiently in a streaming manner
    async fn read_file_as_base64(&self, file_path: &Path) -> ServiceResult<String> {
        let file = File::open(file_path).await?;
        let mut reader = BufReader::new(file);

        let mut output = Vec::new();
        {
            // Wrap output Vec<u8> in a Base64 encoder writer
            let mut encoder = EncoderWriter::new(&mut output, &general_purpose::STANDARD);

            let mut buffer = [0u8; 8192];
            loop {
                let n = reader.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                // Write raw bytes to the Base64 encoder
                encoder.write_all(&buffer[..n])?;
            }
            // Make sure to flush any remaining bytes
            encoder.flush()?;
        } // drop encoder before consuming output

        // Convert the Base64 bytes to String (safe UTF-8)
        let base64_string =
            String::from_utf8(output).map_err(|err| ServiceError::FromString(format!("{err}")))?;
        Ok(base64_string)
    }

    pub async fn read_text_file(&self, file_path: &Path) -> ServiceResult<String> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;
        let content = tokio::fs::read_to_string(valid_path).await?;
        Ok(content)
    }

    pub async fn create_directory(&self, file_path: &Path) -> ServiceResult<()> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;
        tokio::fs::create_dir_all(valid_path).await?;
        Ok(())
    }

    pub async fn move_file(&self, src_path: &Path, dest_path: &Path) -> ServiceResult<()> {
        let allowed_directories = self.allowed_directories().await;
        let valid_src_path = self.validate_path(src_path, allowed_directories.clone())?;
        let valid_dest_path = self.validate_path(dest_path, allowed_directories)?;
        tokio::fs::rename(valid_src_path, valid_dest_path).await?;
        Ok(())
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

    pub async fn write_file(&self, file_path: &Path, content: &String) -> ServiceResult<()> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;
        tokio::fs::write(valid_path, content).await?;
        Ok(())
    }

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
        let glob_pattern = Pattern::new(&updated_pattern);

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
                        pattern.clone()
                    } else {
                        format!("*{pattern}*")
                    };

                    Pattern::new(&glob_pattern)
                        .map(|glob| glob.matches(relative_path.to_str().unwrap_or("")))
                        .unwrap_or(false)
                });

                // enforce min/max bytes
                if !should_exclude && (min_bytes.is_none() || max_bytes.is_none()) {
                    match dir_entry.metadata().ok() {
                        Some(metadata) => {
                            if !self.filesize_in_range(metadata.len(), min_bytes, max_bytes) {
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
                glob_pattern
                    .as_ref()
                    .map(|glob| {
                        glob.matches(&entry.file_name().to_str().unwrap_or("").to_lowercase())
                    })
                    .unwrap_or(false)
            });

        Ok(result)
    }

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

    pub fn create_unified_diff(
        &self,
        original_content: &str,
        new_content: &str,
        filepath: Option<String>,
    ) -> String {
        // Ensure consistent line endings for diff
        let normalized_original = normalize_line_endings(original_content);
        let normalized_new = normalize_line_endings(new_content);

        // // Generate the diff using TextDiff
        let diff = TextDiff::from_lines(&normalized_original, &normalized_new);

        let file_name = filepath.unwrap_or("file".to_string());
        // Format the diff as a unified diff
        let patch = diff
            .unified_diff()
            .header(
                format!("{file_name}\toriginal").as_str(),
                format!("{file_name}\tmodified").as_str(),
            )
            .context_radius(4)
            .to_string();

        format!("Index: {}\n{}\n{}", file_name, "=".repeat(68), patch)
    }

    pub async fn apply_file_edits(
        &self,
        file_path: &Path,
        edits: Vec<EditOperation>,
        dry_run: Option<bool>,
        save_to: Option<&Path>,
    ) -> ServiceResult<String> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;

        // Read file content and normalize line endings
        let content_str = tokio::fs::read_to_string(&valid_path).await?;
        let original_line_ending = self.detect_line_ending(&content_str);
        let content_str = normalize_line_endings(&content_str);

        // Apply edits sequentially
        let mut modified_content = content_str.clone();

        for edit in edits {
            let normalized_old = normalize_line_endings(&edit.old_text);
            let normalized_new = normalize_line_endings(&edit.new_text);
            // If exact match exists, use it
            if modified_content.contains(&normalized_old) {
                modified_content = modified_content.replacen(&normalized_old, &normalized_new, 1);
                continue;
            }

            // Otherwise, try line-by-line matching with flexibility for whitespace
            let old_lines: Vec<String> = normalized_old
                .trim_end()
                .split('\n')
                .map(|s| s.to_string())
                .collect();

            let content_lines: Vec<String> = modified_content
                .trim_end()
                .split('\n')
                .map(|s| s.to_string())
                .collect();

            let mut match_found = false;

            // skip when the match is impossible:
            if old_lines.len() > content_lines.len() {
                let error_message = format!(
                    "Cannot apply edit: the original text spans more lines ({}) than the file content ({}).",
                    old_lines.len(),
                    content_lines.len()
                );

                return Err(RpcError::internal_error()
                    .with_message(error_message)
                    .into());
            }

            let max_start = content_lines.len().saturating_sub(old_lines.len());
            for i in 0..=max_start {
                let potential_match = &content_lines[i..i + old_lines.len()];

                // Compare lines with normalized whitespace
                let is_match = old_lines.iter().enumerate().all(|(j, old_line)| {
                    let content_line = &potential_match[j];
                    old_line.trim() == content_line.trim()
                });

                if is_match {
                    // Preserve original indentation of first line
                    let original_indent = content_lines[i]
                        .chars()
                        .take_while(|&c| c.is_whitespace())
                        .collect::<String>();

                    let new_lines: Vec<String> = normalized_new
                        .split('\n')
                        .enumerate()
                        .map(|(j, line)| {
                            // Keep indentation of the first line
                            if j == 0 {
                                return format!("{}{}", original_indent, line.trim_start());
                            }

                            // For subsequent lines, preserve relative indentation and original whitespace type
                            let old_indent = old_lines
                                .get(j)
                                .map(|line| {
                                    line.chars()
                                        .take_while(|&c| c.is_whitespace())
                                        .collect::<String>()
                                })
                                .unwrap_or_default();

                            let new_indent = line
                                .chars()
                                .take_while(|&c| c.is_whitespace())
                                .collect::<String>();

                            // Use the same whitespace character as original_indent (tabs or spaces)
                            let indent_char = if original_indent.contains('\t') {
                                "\t"
                            } else {
                                " "
                            };
                            let relative_indent = if new_indent.len() >= old_indent.len() {
                                new_indent.len() - old_indent.len()
                            } else {
                                0 // Don't reduce indentation below original
                            };
                            format!(
                                "{}{}{}",
                                &original_indent,
                                &indent_char.repeat(relative_indent),
                                line.trim_start()
                            )
                        })
                        .collect();

                    let mut content_lines = content_lines.clone();
                    content_lines.splice(i..i + old_lines.len(), new_lines);
                    modified_content = content_lines.join("\n");
                    match_found = true;
                    break;
                }
            }
            if !match_found {
                return Err(RpcError::internal_error()
                    .with_message(format!(
                        "Could not find exact match for edit:\n{}",
                        edit.old_text
                    ))
                    .into());
            }
        }

        let diff = self.create_unified_diff(
            &content_str,
            &modified_content,
            Some(valid_path.display().to_string()),
        );

        // Format diff with appropriate number of backticks
        let mut num_backticks = 3;
        while diff.contains(&"`".repeat(num_backticks)) {
            num_backticks += 1;
        }
        let formatted_diff = format!(
            "{}diff\n{}{}\n\n",
            "`".repeat(num_backticks),
            diff,
            "`".repeat(num_backticks)
        );

        let is_dry_run = dry_run.unwrap_or(false);

        if !is_dry_run {
            let target = save_to.unwrap_or(valid_path.as_path());
            let modified_content = modified_content.replace("\n", original_line_ending);
            tokio::fs::write(target, modified_content).await?;
        }

        Ok(formatted_diff)
    }

    pub fn escape_regex(&self, text: &str) -> String {
        // Covers special characters in regex engines (RE2, PCRE, JS, Python)
        const SPECIAL_CHARS: &[char] = &[
            '.', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '\\', '|', '/',
        ];

        let mut escaped = String::with_capacity(text.len());

        for ch in text.chars() {
            if SPECIAL_CHARS.contains(&ch) {
                escaped.push('\\');
            }
            escaped.push(ch);
        }

        escaped
    }

    // Searches the content of a file for occurrences of the given query string.
    ///
    /// This method searches the file specified by `file_path` for lines matching the `query`.
    /// The search can be performed as a regular expression or as a literal string,
    /// depending on the `is_regex` flag.
    ///
    /// If matched line is larger than 255 characters, a snippet will be extracted around the matched text.
    ///
    pub fn content_search(
        &self,
        query: &str,
        file_path: impl AsRef<Path>,
        is_regex: Option<bool>,
    ) -> ServiceResult<Option<FileSearchResult>> {
        let query = if is_regex.unwrap_or_default() {
            query.to_string()
        } else {
            self.escape_regex(query)
        };

        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(true)
            .build(query.as_str())?;

        let mut searcher = Searcher::new();
        let mut result = FileSearchResult {
            file_path: file_path.as_ref().to_path_buf(),
            matches: vec![],
        };

        searcher.set_binary_detection(BinaryDetection::quit(b'\x00'));

        searcher.search_path(
            &matcher,
            file_path,
            UTF8(|line_number, line| {
                let actual_match = matcher.find(line.as_bytes())?.unwrap();

                result.matches.push(ContentMatchResult {
                    line_number,
                    start_pos: actual_match.start(),
                    line_text: self.extract_snippet(line, actual_match, None, None),
                });
                Ok(true)
            }),
        )?;

        if result.matches.is_empty() {
            return Ok(None);
        }

        Ok(Some(result))
    }

    /// Extracts a snippet from a given line of text around a match.
    ///
    /// It extracts a substring starting a fixed number of characters (`SNIPPET_BACKWARD_CHARS`)
    /// before the start position of the `match`, and extends up to `max_length` characters
    /// If the snippet does not include the beginning or end of the original line, ellipses (`"..."`) are added
    /// to indicate the truncation.
    pub fn extract_snippet(
        &self,
        line: &str,
        match_result: Match,
        max_length: Option<usize>,
        backward_chars: Option<usize>,
    ) -> String {
        let max_length = max_length.unwrap_or(SNIPPET_MAX_LENGTH);
        let backward_chars = backward_chars.unwrap_or(SNIPPET_BACKWARD_CHARS);

        // Calculate the number of leading whitespace bytes to adjust for trimmed input
        let start_pos = line.len() - line.trim_start().len();
        // Trim leading and trailing whitespace from the input line
        let line = line.trim();

        // Calculate the desired start byte index by adjusting match start for trimming and backward chars
        // match_result.start() is the byte index in the original string
        // Subtract start_pos to account for trimmed whitespace and backward_chars to include context before the match
        let desired_start = (match_result.start() - start_pos).saturating_sub(backward_chars);

        // Find the nearest valid UTF-8 character boundary at or after desired_start
        // Prevents "byte index is not a char boundary" panic by ensuring the slice starts at a valid character (issue #37)
        let snippet_start = line
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= desired_start)
            .unwrap_or(desired_start.min(line.len()));
        // Initialize a counter for tracking characters to respect max_length
        let mut char_count = 0;

        // Calculate the desired end byte index by counting max_length characters from snippet_start
        // Take max_length + 1 to find the boundary after the last desired character
        let desired_end = line[snippet_start..]
            .char_indices()
            .take(max_length + 1)
            .find(|&(_, _)| {
                char_count += 1;
                char_count > max_length
            })
            .map(|(i, _)| snippet_start + i)
            .unwrap_or(line.len());

        // Ensure snippet_end is a valid UTF-8 character boundary at or after desired_end
        // This prevents slicing issues with multi-byte characters
        let snippet_end = line
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= desired_end)
            .unwrap_or(line.len());

        // Cap snippet_end to avoid exceeding the string length
        let snippet_end = snippet_end.min(line.len());

        // Extract the snippet from the trimmed line using the calculated byte indices
        let snippet = &line[snippet_start..snippet_end];

        let mut result = String::new();
        // Add leading ellipsis if the snippet doesn't start at the beginning of the trimmed line
        if snippet_start > 0 {
            result.push_str("...");
        }

        result.push_str(snippet);

        // Add trailing ellipsis if the snippet doesn't reach the end of the trimmed line
        if snippet_end < line.len() {
            result.push_str("...");
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search_files_content(
        &self,
        root_path: impl AsRef<Path>,
        pattern: &str,
        query: &str,
        is_regex: bool,
        exclude_patterns: Option<Vec<String>>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<FileSearchResult>> {
        let files_iter = self
            .search_files_iter(
                root_path.as_ref(),
                pattern.to_string(),
                exclude_patterns.to_owned().unwrap_or_default(),
                min_bytes,
                max_bytes,
            )
            .await?;

        let results: Vec<FileSearchResult> = files_iter
            .filter_map(|entry| {
                self.content_search(query, entry.path(), Some(is_regex))
                    .ok()
                    .and_then(|v| v)
            })
            .collect();
        Ok(results)
    }

    /// Reads the first n lines from a text file, preserving line endings.
    /// Args:
    ///     file_path: Path to the file
    ///     n: Number of lines to read
    /// Returns a String containing the first n lines with original line endings or an error if the path is invalid or file cannot be read.
    pub async fn head_file(&self, file_path: &Path, n: usize) -> ServiceResult<String> {
        // Validate file path against allowed directories
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;

        // Open file asynchronously and create a BufReader
        let file = File::open(&valid_path).await?;
        let mut reader = BufReader::new(file);
        let mut result = String::with_capacity(n * 100); // Estimate capacity (avg 100 bytes/line)
        let mut count = 0;

        // Read lines asynchronously, preserving line endings
        let mut line = Vec::new();
        while count < n {
            line.clear();
            let bytes_read = reader.read_until(b'\n', &mut line).await?;
            if bytes_read == 0 {
                break; // Reached EOF
            }
            result.push_str(&String::from_utf8_lossy(&line));
            count += 1;
        }

        Ok(result)
    }

    /// Reads the last n lines from a text file, preserving line endings.
    /// Args:
    ///     file_path: Path to the file
    ///     n: Number of lines to read
    /// Returns a String containing the last n lines with original line endings or an error if the path is invalid or file cannot be read.
    pub async fn tail_file(&self, file_path: &Path, n: usize) -> ServiceResult<String> {
        // Validate file path against allowed directories
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;

        // Open file asynchronously
        let file = File::open(&valid_path).await?;
        let file_size = file.metadata().await?.len();

        // If file is empty or n is 0, return empty string
        if file_size == 0 || n == 0 {
            return Ok(String::new());
        }

        // Create a BufReader
        let mut reader = BufReader::new(file);
        let mut line_count = 0;
        let mut pos = file_size;
        let chunk_size = 8192; // 8KB chunks
        let mut buffer = vec![0u8; chunk_size];
        let mut newline_positions = Vec::new();

        // Read backwards to collect all newline positions
        while pos > 0 {
            let read_size = chunk_size.min(pos as usize);
            pos -= read_size as u64;
            reader.seek(SeekFrom::Start(pos)).await?;
            let read_bytes = reader.read_exact(&mut buffer[..read_size]).await?;

            // Process chunk in reverse to find newlines
            for (i, byte) in buffer[..read_bytes].iter().enumerate().rev() {
                if *byte == b'\n' {
                    newline_positions.push(pos + i as u64);
                    line_count += 1;
                }
            }
        }

        // Check if file ends with a non-newline character (partial last line)
        if file_size > 0 {
            let mut temp_reader = BufReader::new(File::open(&valid_path).await?);
            temp_reader.seek(SeekFrom::End(-1)).await?;
            let mut last_byte = [0u8; 1];
            temp_reader.read_exact(&mut last_byte).await?;
            if last_byte[0] != b'\n' {
                line_count += 1;
            }
        }

        // Determine start position for reading the last n lines
        let start_pos = if line_count <= n {
            0 // Read from start if fewer than n lines
        } else {
            *newline_positions.get(line_count - n).unwrap_or(&0) + 1
        };

        // Read forward from start_pos
        reader.seek(SeekFrom::Start(start_pos)).await?;
        let mut result = String::with_capacity(n * 100); // Estimate capacity
        let mut line = Vec::new();
        let mut lines_read = 0;

        while lines_read < n {
            line.clear();
            let bytes_read = reader.read_until(b'\n', &mut line).await?;
            if bytes_read == 0 {
                // Handle partial last line at EOF
                if !line.is_empty() {
                    result.push_str(&String::from_utf8_lossy(&line));
                }
                break;
            }
            result.push_str(&String::from_utf8_lossy(&line));
            lines_read += 1;
        }

        Ok(result)
    }

    /// Reads lines from a text file starting at the specified offset (0-based), preserving line endings.
    /// Args:
    ///     path: Path to the file
    ///     offset: Number of lines to skip (0-based)
    ///     limit: Optional maximum number of lines to read
    /// Returns a String containing the selected lines with original line endings or an error if the path is invalid or file cannot be read.
    pub async fn read_file_lines(
        &self,
        path: &Path,
        offset: usize,
        limit: Option<usize>,
    ) -> ServiceResult<String> {
        // Validate file path against allowed directories
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(path, allowed_directories)?;

        // Open file and get metadata before moving into BufReader
        let file = File::open(&valid_path).await?;
        let file_size = file.metadata().await?.len();
        let mut reader = BufReader::new(file);

        // If file is empty or limit is 0, return empty string
        if file_size == 0 || limit == Some(0) {
            return Ok(String::new());
        }

        // Skip offset lines (0-based indexing)
        let mut buffer = Vec::new();
        for _ in 0..offset {
            buffer.clear();
            if reader.read_until(b'\n', &mut buffer).await? == 0 {
                return Ok(String::new()); // EOF before offset
            }
        }

        // Read lines up to limit (or all remaining if limit is None)
        let mut result = String::with_capacity(limit.unwrap_or(100) * 100); // Estimate capacity
        match limit {
            Some(max_lines) => {
                for _ in 0..max_lines {
                    buffer.clear();
                    let bytes_read = reader.read_until(b'\n', &mut buffer).await?;
                    if bytes_read == 0 {
                        break; // Reached EOF
                    }
                    result.push_str(&String::from_utf8_lossy(&buffer));
                }
            }
            None => {
                loop {
                    buffer.clear();
                    let bytes_read = reader.read_until(b'\n', &mut buffer).await?;
                    if bytes_read == 0 {
                        break; // Reached EOF
                    }
                    result.push_str(&String::from_utf8_lossy(&buffer));
                }
            }
        }

        Ok(result)
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
