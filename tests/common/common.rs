use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use rust_mcp_filesystem::{
    cli::CommandArguments,
    fs_service::{file_info::FileInfo, FileSystemService},
};
use tempfile::TempDir;

pub fn get_temp_dir() -> PathBuf {
    let temp_dir = TempDir::new().unwrap().path().canonicalize().unwrap();
    fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

// Helper to create a FileSystemService with temporary directories
pub fn setup_service(dirs: Vec<String>) -> (PathBuf, FileSystemService, Arc<Vec<PathBuf>>) {
    let temp_dir = get_temp_dir();
    let allowed_dirs = dirs
        .into_iter()
        .map(|d| {
            let dir_path = temp_dir.join(&d);
            // Create the directory if it doesn't exist
            fs::create_dir_all(&dir_path).unwrap();
            dir_path.to_str().unwrap().to_string()
        })
        .collect::<Vec<String>>();
    let service = FileSystemService::try_new(&allowed_dirs).unwrap();
    let allowed_dirs: Vec<PathBuf> = allowed_dirs.iter().map(|i| i.into()).collect();
    (temp_dir, service, Arc::new(allowed_dirs))
}

// Helper to create a temporary file
pub fn create_temp_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let file_path = dir.join(name);
    File::create(&file_path)
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    file_path
}

// Helper to create a temporary file and get its FileInfo
pub fn create_temp_file_info(content: &[u8]) -> (PathBuf, FileInfo) {
    let dir = get_temp_dir();
    let file_path = dir.join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(content).unwrap();
    file.flush().unwrap();

    let metadata = fs::metadata(&file_path).unwrap();
    let file_info = FileInfo {
        size: metadata.len(),
        created: metadata.created().ok(),
        modified: metadata.modified().ok(),
        accessed: metadata.accessed().ok(),
        is_directory: metadata.is_dir(),
        is_file: metadata.is_file(),
        metadata,
    };
    (dir, file_info)
}

// Helper to create a temporary directory and get its FileInfo
pub fn create_temp_dir() -> (TempDir, FileInfo) {
    let dir = TempDir::new().unwrap();
    let metadata = fs::metadata(dir.path()).unwrap();
    let file_info = FileInfo {
        size: metadata.len(),
        created: metadata.created().ok(),
        modified: metadata.modified().ok(),
        accessed: metadata.accessed().ok(),
        is_directory: metadata.is_dir(),
        is_file: metadata.is_file(),
        metadata,
    };
    (dir, file_info)
}

// Helper function to try to parse arguments and return the result
pub fn parse_args(args: &[&str]) -> Result<CommandArguments, clap::Error> {
    CommandArguments::try_parse_from(args)
}
