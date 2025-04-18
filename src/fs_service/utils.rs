use std::{
    fs::{self},
    path::{Component, Path, PathBuf, Prefix},
    time::SystemTime,
};

use async_zip::{error::ZipError, tokio::write::ZipFileWriter, Compression, ZipEntryBuilder};
use chrono::{DateTime, Local};
use dirs::home_dir;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

pub fn format_system_time(system_time: SystemTime) -> String {
    // Convert SystemTime to DateTime<Local>
    let datetime: DateTime<Local> = system_time.into();
    datetime.format("%a %b %d %Y %H:%M:%S %:z").to_string()
}

pub fn format_permissions(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        format!("0{:o}", mode & 0o777) // Octal representation
    }

    #[cfg(windows)]
    {
        let attributes = metadata.file_attributes();
        let read_only = (attributes & 0x1) != 0; // FILE_ATTRIBUTE_READONLY
        let directory = metadata.is_dir();

        let mut result = String::new();

        if directory {
            result.push('d');
        } else {
            result.push('-');
        }

        if read_only {
            result.push('r');
        } else {
            result.push('w');
        }

        result
    }
}

pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn expand_home(path: PathBuf) -> PathBuf {
    if let Some(home_dir) = home_dir() {
        if path.starts_with("~") {
            let stripped_path = path.strip_prefix("~").unwrap_or(&path);
            return home_dir.join(stripped_path);
        }
    }
    path
}

pub fn format_bytes(bytes: u64) -> String {
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
    format!("{} bytes", bytes)
}

pub async fn write_zip_entry(
    filename: &str,
    input_path: &Path,
    zip_writer: &mut ZipFileWriter<File>,
) -> Result<(), ZipError> {
    let mut input_file = File::open(input_path).await?;
    let input_file_size = input_file.metadata().await?.len() as usize;

    let mut buffer = Vec::with_capacity(input_file_size);
    input_file.read_to_end(&mut buffer).await?;

    let builder = ZipEntryBuilder::new(filename.into(), Compression::Deflate);
    zip_writer.write_entry_whole(builder, &buffer).await?;

    Ok(())
}

pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

// checks if path component is a  Prefix::VerbatimDisk
fn is_verbatim_disk(component: &Component) -> bool {
    match component {
        Component::Prefix(prefix_comp) => matches!(prefix_comp.kind(), Prefix::VerbatimDisk(_)),
        _ => false,
    }
}

/// Check path contains a symlink
pub fn contains_symlink<P: AsRef<Path>>(path: P) -> std::io::Result<bool> {
    let mut current_path = PathBuf::new();

    for component in path.as_ref().components() {
        current_path.push(component);

        // no need to check symlink_metadata for Prefix::VerbatimDisk
        if is_verbatim_disk(&component) {
            continue;
        }

        if !current_path.exists() {
            break;
        }

        if fs::symlink_metadata(&current_path)?
            .file_type()
            .is_symlink()
        {
            return Ok(true);
        }
    }

    Ok(false)
}
