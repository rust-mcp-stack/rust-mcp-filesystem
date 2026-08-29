use crate::{error::ServiceResult, fs_service::FileSystemService};
use std::path::Path;

impl FileSystemService {
    pub async fn write_file(&self, file_path: &Path, content: &String) -> ServiceResult<()> {
        let resolved = self.resolve(file_path).await?;
        resolved.dir.write(&resolved.rel, content.as_bytes())?;
        Ok(())
    }

    pub async fn create_directory(&self, file_path: &Path) -> ServiceResult<()> {
        let resolved = self.resolve(file_path).await?;
        resolved.create_dir_all()?;
        Ok(())
    }

    pub async fn move_file(&self, src_path: &Path, dest_path: &Path) -> ServiceResult<()> {
        let resolved_src = self.resolve(src_path).await?;
        let resolved_dest = self.resolve(dest_path).await?;

        // Rename across the same or different allowed roots. `cap_std` confines
        // both source and destination for local roots; UNC shares fall back to
        // `std::fs::rename` with containment verification.
        resolved_src.rename_to(&resolved_dest)?;
        Ok(())
    }
}
