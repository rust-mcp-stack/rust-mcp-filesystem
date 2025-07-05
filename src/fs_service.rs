pub mod file_info;
pub mod utils;
use file_info::FileInfo;
use grep::{
    matcher::{Match, Matcher},
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, BinaryDetection, Searcher},
};
use serde_json::{json, Value};

use std::{
    env,
    fs::{self},
    path::{Path, PathBuf},
};

use async_zip::tokio::{read::seek::ZipFileReader, write::ZipFileWriter};
use glob::Pattern;
use rust_mcp_sdk::schema::RpcError;
use similar::TextDiff;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufReader},
};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use utils::{
    contains_symlink, expand_home, format_bytes, normalize_line_endings, normalize_path,
    write_zip_entry,
};
use walkdir::WalkDir;

use crate::{
    error::{ServiceError, ServiceResult},
    tools::EditOperation,
};

const SNIPPET_MAX_LENGTH: usize = 200;
const SNIPPET_BACKWARD_CHARS: usize = 30;

pub struct FileSystemService {
    allowed_path: Vec<PathBuf>,
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
            allowed_path: normalized_dirs,
        })
    }

    pub fn allowed_directories(&self) -> &Vec<PathBuf> {
        &self.allowed_path
    }
}

impl FileSystemService {
    pub fn validate_path(&self, requested_path: &Path) -> ServiceResult<PathBuf> {
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
        if !self.allowed_path.iter().any(|dir| {
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
                self.allowed_path
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
        let valid_path = self.validate_path(file_path)?;

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
        let valid_dir_path = self.validate_path(Path::new(&input_dir))?;

        let input_dir_str = &valid_dir_path
            .as_os_str()
            .to_str()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid UTF-8 in file name",
            ))?;

        let target_path = self.validate_path(Path::new(&target_zip_file))?;

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

                self.validate_path(full_path).ok().and_then(|path| {
                    if path != valid_dir_path && glob_pattern.matches(&path.display().to_string()) {
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

        let target_path = self.validate_path(Path::new(&target_zip_file))?;

        if target_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_zip_file}' already exists!"),
            )
            .into());
        }

        let source_paths = input_files
            .iter()
            .map(|p| self.validate_path(Path::new(p)))
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
        let zip_file = self.validate_path(Path::new(&zip_file))?;
        let target_dir_path = self.validate_path(Path::new(target_dir))?;
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

    pub async fn read_file(&self, file_path: &Path) -> ServiceResult<String> {
        let valid_path = self.validate_path(file_path)?;
        let content = tokio::fs::read_to_string(valid_path).await?;
        Ok(content)
    }

    pub async fn create_directory(&self, file_path: &Path) -> ServiceResult<()> {
        let valid_path = self.validate_path(file_path)?;
        tokio::fs::create_dir_all(valid_path).await?;
        Ok(())
    }

    pub async fn move_file(&self, src_path: &Path, dest_path: &Path) -> ServiceResult<()> {
        let valid_src_path = self.validate_path(src_path)?;
        let valid_dest_path = self.validate_path(dest_path)?;
        tokio::fs::rename(valid_src_path, valid_dest_path).await?;
        Ok(())
    }

    pub async fn list_directory(&self, dir_path: &Path) -> ServiceResult<Vec<tokio::fs::DirEntry>> {
        let valid_path = self.validate_path(dir_path)?;

        let mut dir = tokio::fs::read_dir(valid_path).await?;

        let mut entries = Vec::new();

        // Use a loop to collect the directory entries
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry);
        }

        Ok(entries)
    }

    pub async fn write_file(&self, file_path: &Path, content: &String) -> ServiceResult<()> {
        let valid_path = self.validate_path(file_path)?;
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
    pub fn search_files(
        &self,
        root_path: &Path,
        pattern: String,
        exclude_patterns: Vec<String>,
    ) -> ServiceResult<Vec<walkdir::DirEntry>> {
        let result = self.search_files_iter(root_path, pattern, exclude_patterns)?;
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
    pub fn search_files_iter<'a>(
        &'a self,
        // root_path: impl Into<PathBuf>,
        root_path: &'a Path,
        pattern: String,
        exclude_patterns: Vec<String>,
    ) -> ServiceResult<impl Iterator<Item = walkdir::DirEntry> + 'a> {
        let valid_path = self.validate_path(root_path)?;

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
                let validated_path = self.validate_path(full_path).ok();

                if validated_path.is_none() {
                    // Skip invalid paths during search
                    return false;
                }

                // Get the relative path from the root_path
                let relative_path = full_path.strip_prefix(root_path).unwrap_or(full_path);

                let should_exclude = exclude_patterns.iter().any(|pattern| {
                    let glob_pattern = if pattern.contains('*') {
                        pattern.clone()
                    } else {
                        format!("*{pattern}*")
                    };

                    Pattern::new(&glob_pattern)
                        .map(|glob| glob.matches(relative_path.to_str().unwrap_or("")))
                        .unwrap_or(false)
                });

                !should_exclude
            })
            .filter_map(|v| v.ok())
            .filter(move |entry| {
                if root_path == entry.path() {
                    return false;
                }

                let is_match = glob_pattern
                    .as_ref()
                    .map(|glob| {
                        glob.matches(&entry.file_name().to_str().unwrap_or("").to_lowercase())
                    })
                    .unwrap_or(false);
                is_match
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
    ) -> ServiceResult<(Value, bool)> {
        let valid_path = self.validate_path(root_path.as_ref())?;

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
                    let (child_children, child_reached_max_depth) =
                        self.directory_tree(child_path, next_depth, max_files, current_count)?;
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
        let valid_path = self.validate_path(file_path)?;

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

        let start_pos = line.len() - line.trim_start().len();

        let line = line.trim();

        // Start SNIPPET_BACKWARD_CHARS characters before match (or at 0)
        let snippet_start = (match_result.start() - start_pos).saturating_sub(backward_chars);

        // Get up to SNIPPET_MAX_LENGTH characters from snippet_start
        let snippet_end = (snippet_start + max_length).min(line.len());

        let snippet = &line[snippet_start..snippet_end];

        // Add ellipses if line was truncated
        let mut result = String::new();
        if snippet_start > 0 {
            result.push_str("...");
        }
        result.push_str(snippet);
        if snippet_end < line.len() {
            result.push_str("...");
        }
        result
    }

    pub fn search_files_content(
        &self,
        root_path: impl AsRef<Path>,
        pattern: &str,
        query: &str,
        is_regex: bool,
        exclude_patterns: Option<Vec<String>>,
    ) -> ServiceResult<Vec<FileSearchResult>> {
        let files_iter = self.search_files_iter(
            root_path.as_ref(),
            pattern.to_string(),
            exclude_patterns.to_owned().unwrap_or_default(),
        )?;

        let results: Vec<FileSearchResult> = files_iter
            .filter_map(|entry| {
                self.content_search(query, entry.path(), Some(is_regex))
                    .ok()
                    .and_then(|v| v)
            })
            .collect();
        Ok(results)
    }
}
