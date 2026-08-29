use crate::{error::ServiceResult, fs_service::FileSystemService};
use rc_zip_tokio::ReadZip;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

impl FileSystemService {
    pub async fn unzip_file(&self, zip_file: &str, target_dir: &str) -> ServiceResult<String> {
        let resolved_zip = self.resolve(Path::new(zip_file)).await?;
        let resolved_target = self.resolve(Path::new(target_dir)).await?;

        if !resolved_zip.dir.exists(&resolved_zip.rel) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Zip file does not exists.",
            )
            .into());
        }

        if resolved_target.dir.exists(&resolved_target.rel) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_dir}' directory already exists!"),
            )
            .into());
        }

        let zip_data = resolved_zip.dir.read(&resolved_zip.rel)?;

        let archive = zip_data.read_zip().await?;

        let entries: Vec<_> = archive.entries().collect();
        let file_count = entries.len();

        for entry in entries {
            let name = entry.sanitized_name().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid entry name")
            })?;
            let entry_rel = resolved_target.rel.join(PathBuf::from(name));
            if let Some(parent) = entry_rel.parent() {
                resolved_target.create_dir_all_rel(parent)?;
            }

            let mut reader = entry.reader();
            let mut output_file = resolved_target.dir.create(&entry_rel)?;

            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;
            output_file.write_all(&buffer)?;
            output_file.flush()?;
        }

        let result_message = format!(
            "Successfully extracted {} {} into '{}'.",
            file_count,
            if file_count == 1 { "file" } else { "files" },
            resolved_target.display.display()
        );

        Ok(result_message)
    }
}
