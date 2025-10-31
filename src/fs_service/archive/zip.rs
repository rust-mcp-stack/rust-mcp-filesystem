use crate::{
    error::ServiceResult,
    fs_service::{
        FileSystemService,
        utils::{format_bytes, write_zip_entry},
    },
};
use async_zip::tokio::write::ZipFileWriter;
use glob_match::glob_match;
use std::path::Path;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;
use walkdir::WalkDir;

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
                        if path != valid_dir_path
                            && glob_match(glob_pattern, path.display().to_string().as_ref())
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
}
