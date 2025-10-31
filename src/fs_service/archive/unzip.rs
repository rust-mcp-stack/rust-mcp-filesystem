use crate::{error::ServiceResult, fs_service::FileSystemService};
use async_zip::tokio::read::seek::ZipFileReader;
use std::path::Path;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufReader},
};
use tokio_util::compat::FuturesAsyncReadCompatExt;

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
}
