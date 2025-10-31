use crate::{error::ServiceResult, fs_service::FileSystemService};
use std::path::Path;

impl FileSystemService {
    pub async fn write_file(&self, file_path: &Path, content: &String) -> ServiceResult<()> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;
        tokio::fs::write(valid_path, content).await?;
        Ok(())
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
}
