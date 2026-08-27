use crate::{
    error::{ServiceError, ServiceResult},
    fs_service::{
        FileSystemService,
        utils::{encode_base64, format_permissions, format_system_time, mime_from_bytes},
    },
};
use futures::{StreamExt, stream};
use std::fs::{self};
use std::io::SeekFrom;
use std::time::SystemTime;
use std::path::Path;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader},
};

const MAX_CONCURRENT_FILE_READ: usize = 5;

/// Opens a resolved path for reading as a `tokio` file, confining access to
/// the directory handle that `resolved` was opened from.
fn open_tokio(resolved: &crate::fs_service::Resolved) -> ServiceResult<File> {
    let std_file = resolved.dir.open(&resolved.rel)?.into_std();
    Ok(File::from_std(std_file))
}

/// Returns `std::fs::Metadata` for a resolved path (file or directory).
fn std_metadata(resolved: &crate::fs_service::Resolved) -> ServiceResult<fs::Metadata> {
    match resolved.dir.open(&resolved.rel) {
        Ok(file) => Ok(file.into_std().metadata()?),
        Err(_) => Ok(resolved.dir.open_dir(&resolved.rel)?.into_std_file().metadata()?),
    }
}

impl FileSystemService {
    pub async fn read_text_file(
        &self,
        file_path: &Path,
        with_line_numbers: bool,
    ) -> ServiceResult<String> {
        let resolved = self.resolve(file_path).await?;
        let content = resolved.dir.read_to_string(&resolved.rel)?;

        if with_line_numbers {
            Ok(content
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{:>6} | {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n"))
        } else {
            Ok(content)
        }
    }

    /// Reads the first n lines from a text file, preserving line endings.
    /// Args:
    ///     file_path: Path to the file
    ///     n: Number of lines to read
    /// Returns a String containing the first n lines with original line endings or an error if the path is invalid or file cannot be read.
    pub async fn head_file(&self, file_path: &Path, n: usize) -> ServiceResult<String> {
        let resolved = self.resolve(file_path).await?;

        // Open file asynchronously and create a BufReader
        let file = open_tokio(&resolved)?;
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
        let resolved = self.resolve(file_path).await?;

        // Open file asynchronously
        let file = open_tokio(&resolved)?;
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
            let mut temp_reader = BufReader::new(open_tokio(&resolved)?);
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
            *newline_positions.get(n - 1).unwrap_or(&0) + 1
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
        let resolved = self.resolve(path).await?;

        // Open file and get metadata before moving into BufReader
        let file = open_tokio(&resolved)?;
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
        let resolved = self.resolve(file_path).await?;
        let bytes = resolved.dir.read(&resolved.rel)?;

        if let Some(max) = max_bytes
            && bytes.len() > max
        {
            return Err(ServiceError::FileTooLarge(max));
        }

        let kind = mime_from_bytes(&bytes, &resolved.rel)?;
        let content = encode_base64(&bytes);
        Ok((kind, content))
    }

    // Get file stats
    pub async fn get_file_stats(&self, file_path: &Path) -> ServiceResult<FileInfo> {
        let resolved = self.resolve(file_path).await?;

        let metadata = std_metadata(&resolved)?;

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
}

#[derive(Debug)]
pub struct FileInfo {
    pub size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub is_directory: bool,
    pub is_file: bool,
    pub metadata: fs::Metadata,
}

impl std::fmt::Display for FileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"size: {}
created: {}
modified: {}
accessed: {}
isDirectory: {}
isFile: {}
permissions: {}
"#,
            self.size,
            self.created.map_or("".to_string(), format_system_time),
            self.modified.map_or("".to_string(), format_system_time),
            self.accessed.map_or("".to_string(), format_system_time),
            self.is_directory,
            self.is_file,
            format_permissions(&self.metadata)
        )
    }
}
