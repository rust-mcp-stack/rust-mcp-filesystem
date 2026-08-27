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
        resolved.dir.create_dir_all(&resolved.rel)?;
        Ok(())
    }

    pub async fn move_file(&self, src_path: &Path, dest_path: &Path) -> ServiceResult<()> {
        let resolved_src = self.resolve(src_path).await?;
        let resolved_dest = self.resolve(dest_path).await?;

        // Rename across the same or different allowed roots. `cap_std` confines
        // both source and destination, so a symlink cannot redirect either.
        resolved_src
            .dir
            .rename(&resolved_src.rel, &resolved_dest.dir, &resolved_dest.rel)?;
        Ok(())
    }
}
