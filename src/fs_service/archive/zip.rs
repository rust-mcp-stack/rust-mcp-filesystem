use crate::{error::ServiceResult, fs_service::FileSystemService};
use glob_match::glob_match;
use std::fs::File as StdFile;
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::write::ZipWriter;

fn format_bytes_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    let units = [(TB, "TB"), (GB, "GB"), (MB, "MB"), (KB, "KB")];

    for (threshold, unit) in units {
        if bytes >= threshold {
            return format!("{:.2} {}", bytes as f64 / threshold as f64, unit);
        }
    }
    format!("{bytes} bytes")
}

impl FileSystemService {
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

        let glob_pattern = &updated_pattern;

        let entries: Vec<_> = WalkDir::new(&valid_dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let full_path = entry.path();

                self.validate_path(full_path, allowed_directories.clone())
                    .ok()
                    .and_then(|path| {
                        if path != valid_dir_path {
                            let relative_path = path
                                .strip_prefix(input_dir_str)
                                .ok()
                                .map(|p| p.display().to_string());
                            let matches = relative_path
                                .map(|rel| glob_match(glob_pattern, rel.as_ref()))
                                .unwrap_or(false);
                            if matches {
                                return Some(path);
                            }
                        }
                        None
                    })
            })
            .collect();

        let target_path_clone = target_path.clone();
        let entries_clone: Vec<_> = entries.to_vec();
        let input_dir_str_clone = input_dir_str.to_string();

        let zip_file_size = tokio::task::spawn_blocking(move || {
            let file = StdFile::create(&target_path_clone)?;
            let mut zip_writer = ZipWriter::new(file);
            let options: zip::write::FileOptions<()> =
                zip::write::FileOptions::default().compression_method(CompressionMethod::Deflated);

            for entry_path_buf in &entries_clone {
                if entry_path_buf.is_dir() {
                    continue;
                }
                let entry_path = entry_path_buf.as_path();
                let entry_str = entry_path.as_os_str().to_str().ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid UTF-8 in file name",
                ))?;

                if !entry_str.starts_with(&input_dir_str_clone) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Entry file path does not start with base input directory path.",
                    ));
                }

                let entry_str = &entry_str[input_dir_str_clone.len() + 1..];

                let mut input_file = StdFile::open(entry_path)?;
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut input_file, &mut buffer)?;

                zip_writer.start_file(entry_str, options)?;
                zip_writer.write_all(&buffer)?;
                zip_writer.flush()?;
            }

            zip_writer.finish()?;
            let metadata = std::fs::metadata(&target_path_clone)?;
            Ok::<u64, std::io::Error>(metadata.len())
        })
        .await
        .map_err(std::io::Error::other)??;

        let result_message = format!(
            "Successfully compressed '{}' directory into '{}' ({}).",
            input_dir,
            target_path.display(),
            format_bytes_size(zip_file_size)
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

        let target_path_clone = target_path.clone();
        let source_paths_clone: Vec<_> = source_paths.to_vec();

        let zip_file_size = tokio::task::spawn_blocking(move || {
            let file = StdFile::create(&target_path_clone)?;
            let mut zip_writer = ZipWriter::new(file);
            let options: zip::write::FileOptions<()> =
                zip::write::FileOptions::default().compression_method(CompressionMethod::Deflated);

            for path in &source_paths_clone {
                let filename = path.file_name().ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid path!",
                ))?;

                let filename = filename.to_str().ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid UTF-8 in file name",
                ))?;

                let mut input_file = StdFile::open(path)?;
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut input_file, &mut buffer)?;

                zip_writer.start_file(filename, options)?;
                zip_writer.write_all(&buffer)?;
                zip_writer.flush()?;
            }

            zip_writer.finish()?;
            let metadata = std::fs::metadata(&target_path_clone)?;
            Ok::<u64, std::io::Error>(metadata.len())
        })
        .await
        .map_err(std::io::Error::other)??;

        let result_message = format!(
            "Successfully compressed {} {} into '{}' ({}).",
            file_count,
            if file_count == 1 { "file" } else { "files" },
            target_path.display(),
            format_bytes_size(zip_file_size)
        );
        Ok(result_message)
    }
}
