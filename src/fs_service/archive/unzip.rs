use crate::{error::ServiceResult, fs_service::FileSystemService};
use rc_zip_tokio::ReadZip;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl FileSystemService {
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

        let mut file = File::open(&zip_file).await?;
        let mut zip_data = Vec::new();
        file.read_to_end(&mut zip_data).await?;

        let archive = zip_data.read_zip().await?;

        let entries: Vec<_> = archive.entries().collect();
        let file_count = entries.len();

        for entry in entries {
            let name = entry.sanitized_name().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid entry name")
            })?;
            let entry_path = target_dir_path.join(name);
            if let Some(parent) = entry_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let mut reader = entry.reader();
            let mut output_file = File::create(&entry_path).await?;

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;
            output_file.write_all(&buffer).await?;
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
}
