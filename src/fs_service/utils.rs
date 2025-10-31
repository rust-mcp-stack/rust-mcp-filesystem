use crate::error::{ServiceError, ServiceResult};
use async_zip::{Compression, ZipEntryBuilder, error::ZipError, tokio::write::ZipFileWriter};
use base64::{engine::general_purpose, write::EncoderWriter};
use chrono::{DateTime, Local};
use dirs::home_dir;
use rust_mcp_sdk::macros::JsonSchema;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{
    ffi::OsStr,
    fs::{self},
    path::{Component, Path, PathBuf, Prefix},
    time::SystemTime,
};
use tokio::io::AsyncReadExt;
use tokio::{
    fs::{File, metadata},
    io::BufReader,
};

#[cfg(windows)]
pub const OS_LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
pub const OS_LINE_ENDING: &str = "\n";

#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub enum OutputFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json")]
    Json,
}

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
    if let Some(home_dir) = home_dir()
        && path.starts_with("~")
    {
        let stripped_path = path.strip_prefix("~").unwrap_or(&path);
        return home_dir.join(stripped_path);
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
    format!("{bytes} bytes")
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
    text.replace("\r\n", "\n").replace('\r', "\n")
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

/// Checks if a given filename is a system metadata file commonly
/// used by operating systems to store folder metadata.
///
/// Specifically detects:
/// - `.DS_Store` (macOS)
/// - `Thumbs.db` (Windows)
///
pub fn is_system_metadata_file(filename: &OsStr) -> bool {
    filename == ".DS_Store" || filename == "Thumbs.db"
}

// reads file as base64 efficiently in a streaming manner
pub async fn read_file_as_base64(file_path: &Path) -> ServiceResult<String> {
    let file = File::open(file_path).await?;
    let mut reader = BufReader::new(file);

    let mut output = Vec::new();
    {
        // Wrap output Vec<u8> in a Base64 encoder writer
        let mut encoder = EncoderWriter::new(&mut output, &general_purpose::STANDARD);

        let mut buffer = [0u8; 8192];
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            // Write raw bytes to the Base64 encoder
            encoder.write_all(&buffer[..n])?;
        }
        // Make sure to flush any remaining bytes
        encoder.flush()?;
    } // drop encoder before consuming output

    // Convert the Base64 bytes to String (safe UTF-8)
    let base64_string =
        String::from_utf8(output).map_err(|err| ServiceError::FromString(format!("{err}")))?;
    Ok(base64_string)
}

pub fn detect_line_ending(text: &str) -> &str {
    if text.contains("\r\n") {
        "\r\n"
    } else if text.contains('\r') {
        "\r"
    } else {
        "\n"
    }
}

pub fn mime_from_path(path: &Path) -> ServiceResult<infer::Type> {
    let is_svg = path
        .extension()
        .is_some_and(|e| e.to_str().is_some_and(|s| s == "svg"));
    // consider it is a svg file as we cannot detect svg from bytes pattern
    if is_svg {
        return Ok(infer::Type::new(
            infer::MatcherType::Image,
            "image/svg+xml",
            "svg",
            |_: &[u8]| true,
        ));

        // infer::Type::new(infer::MatcherType::Image, "", "svg",);
    }
    let kind = infer::get_from_path(path)?.ok_or(ServiceError::FromString(
        "File tyle is unknown!".to_string(),
    ))?;
    Ok(kind)
}

pub fn escape_regex(text: &str) -> String {
    // Covers special characters in regex engines (RE2, PCRE, JS, Python)
    const SPECIAL_CHARS: &[char] = &[
        '.', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '\\', '|', '/',
    ];

    let mut escaped = String::with_capacity(text.len());

    for ch in text.chars() {
        if SPECIAL_CHARS.contains(&ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }

    escaped
}

pub fn filesize_in_range(file_size: u64, min_bytes: Option<u64>, max_bytes: Option<u64>) -> bool {
    if min_bytes.is_none() && max_bytes.is_none() {
        return true;
    }
    match (min_bytes, max_bytes) {
        (_, Some(max)) if file_size > max => false,
        (Some(min), _) if file_size < min => false,
        _ => true,
    }
}

pub async fn validate_file_size<P: AsRef<Path>>(
    path: P,
    min_bytes: Option<usize>,
    max_bytes: Option<usize>,
) -> ServiceResult<()> {
    if min_bytes.is_none() && max_bytes.is_none() {
        return Ok(());
    }

    let file_size = metadata(&path).await?.len() as usize;

    match (min_bytes, max_bytes) {
        (_, Some(max)) if file_size > max => Err(ServiceError::FileTooLarge(max)),
        (Some(min), _) if file_size < min => Err(ServiceError::FileTooSmall(min)),
        _ => Ok(()),
    }
}

/// Converts a string to a `PathBuf`, supporting both raw paths and `file://` URIs.
pub fn parse_file_path(input: &str) -> ServiceResult<PathBuf> {
    Ok(PathBuf::from(
        input.strip_prefix("file://").unwrap_or(input).trim(),
    ))
}
