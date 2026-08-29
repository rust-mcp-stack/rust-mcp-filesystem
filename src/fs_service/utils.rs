use crate::error::{ServiceError, ServiceResult};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Local};
use dirs::home_dir;
use rust_mcp_sdk::macros::JsonSchema;
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

pub fn normalize_windows_drive_path(path: &Path) -> PathBuf {
    let path_text = path.to_string_lossy();
    let Some((drive, rest)) = split_windows_drive_path(&path_text) else {
        return path.to_path_buf();
    };

    windows_drive_path_buf(drive, &rest)
}

fn split_windows_drive_path(input: &str) -> Option<(char, String)> {
    let normalized = input.trim().replace('\\', "/");
    let without_leading_slash = normalized.strip_prefix('/').unwrap_or(&normalized);
    let bytes = without_leading_slash.as_bytes();

    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'/' {
        return None;
    }

    Some((
        (bytes[0] as char).to_ascii_uppercase(),
        without_leading_slash[3..]
            .trim_start_matches('/')
            .to_string(),
    ))
}

#[cfg(windows)]
fn windows_drive_path_buf(drive: char, rest: &str) -> PathBuf {
    if rest.is_empty() {
        PathBuf::from(format!("{drive}:/"))
    } else {
        PathBuf::from(format!("{drive}:/{rest}"))
    }
}

#[cfg(not(windows))]
fn windows_drive_path_buf(drive: char, rest: &str) -> PathBuf {
    use std::sync::LazyLock;
    static MOUNT_ROOT: LazyLock<String> = LazyLock::new(|| {
        for candidate in ["/mnt", "/cygdrive", ""] {
            if Path::new(&format!("{candidate}/c")).exists() {
                return candidate.to_string();
            }
        }
        "/mnt".to_string()
    });

    let drive = drive.to_ascii_lowercase();
    let root = &*MOUNT_ROOT;
    if rest.is_empty() {
        PathBuf::from(format!("{root}/{drive}"))
    } else {
        PathBuf::from(format!("{root}/{drive}/{rest}"))
    }
}

/// Returns `true` when `path` refers to a Windows UNC share
/// (`\\server\share` or the verbatim `\\?\UNC\server\share` form).
///
/// On macOS/Linux `std::path` never produces `Prefix` components, so this is
/// always `false` there (a `\\server\share` string is just an ordinary file
/// name).
pub fn is_unc_path(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Prefix(p))
            if matches!(p.kind(), Prefix::UNC(..) | Prefix::VerbatimUNC(..))
    )
}

/// Strips the Windows extended-length (`\\?\`) prefix from a canonical path so
/// that it is human-friendly and usable for prefix matching:
///
/// - `\\?\UNC\server\share\dir` → `\\server\share\dir`
/// - `\\?\C:\foo` → `C:\foo`
/// - anything else → unchanged
///
/// This is purely cosmetic/normalization; the verbatim form is still used for
/// the actual `std::fs` system calls (to preserve long-path support).
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    let Some(rest) = text.strip_prefix(r"\\?\") else {
        return path.to_path_buf();
    };
    if let Some(unc) = rest.strip_prefix("UNC\\") {
        PathBuf::from(format!("\\\\{unc}"))
    } else {
        PathBuf::from(rest)
    }
}

/// Removes trailing separators from `path`, without ever dropping a root
/// component (e.g. `C:\` must not become `C:`). Used to deduplicate allowed
/// roots such as `\\server\share` vs `\\server\share\`.
pub fn trim_trailing_separator(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    let trimmed = text.trim_end_matches('\\').trim_end_matches('/');
    if trimmed.is_empty() || trimmed.len() == text.len() {
        return path;
    }
    let candidate = PathBuf::from(trimmed);
    let had_root = path.components().any(|c| matches!(c, Component::RootDir));
    let has_root = candidate
        .components()
        .any(|c| matches!(c, Component::RootDir));
    if had_root && !has_root {
        path
    } else {
        candidate
    }
}

/// Strips `base` from `path` as a prefix, returning the relative remainder.
///
/// On Windows comparison is case-insensitive (UNC server/share names and the
/// filesystem are case-insensitive), on macOS/Linux it is case-sensitive so
/// that mounted `C:` roots under `/mnt/c` keep exact matching.
pub fn strip_prefix_platform(path: &Path, base: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        strip_prefix_ci(path, base)
    }
    #[cfg(not(windows))]
    {
        path.strip_prefix(base).ok().map(Path::to_path_buf)
    }
}

/// Case-insensitive, component-wise `strip_prefix` for Windows.
#[cfg(windows)]
fn strip_prefix_ci(path: &Path, base: &Path) -> Option<PathBuf> {
    let base_components: Vec<Component<'_>> = base.components().collect();
    let path_components: Vec<Component<'_>> = path.components().collect();
    if path_components.len() < base_components.len() {
        return None;
    }

    for (base_component, path_component) in base_components.iter().zip(path_components.iter()) {
        let equal = match (base_component, path_component) {
            (Component::Prefix(base_prefix), Component::Prefix(path_prefix)) => {
                match (base_prefix.kind(), path_prefix.kind()) {
                    (Prefix::UNC(a, b), Prefix::UNC(c, d))
                    | (Prefix::VerbatimUNC(a, b), Prefix::VerbatimUNC(c, d)) => {
                        a.eq_ignore_ascii_case(c) && b.eq_ignore_ascii_case(d)
                    }
                    (Prefix::Disk(a), Prefix::Disk(c))
                    | (Prefix::VerbatimDisk(a), Prefix::VerbatimDisk(c)) => {
                        a.eq_ignore_ascii_case(&c)
                    }
                    _ => false,
                }
            }
            (Component::RootDir, Component::RootDir) => true,
            (Component::Normal(a), Component::Normal(c)) => a.eq_ignore_ascii_case(c),
            (Component::CurDir, Component::CurDir) => true,
            _ => false,
        };
        if !equal {
            return None;
        }
    }

    let mut rel = PathBuf::new();
    for component in &path_components[base_components.len()..] {
        rel.push(component.as_os_str());
    }
    Some(rel)
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

pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
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

pub fn detect_line_ending(text: &str) -> &str {
    if text.contains("\r\n") {
        "\r\n"
    } else if text.contains('\r') {
        "\r"
    } else {
        "\n"
    }
}

pub fn mime_from_bytes(bytes: &[u8], path: &Path) -> ServiceResult<infer::Type> {
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
    }
    infer::get(bytes).ok_or(ServiceError::FromString(
        "File type is unknown!".to_string(),
    ))
}

pub fn encode_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
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

/// Converts a string to a `PathBuf`, supporting both raw paths and `file://` URIs.
pub fn parse_file_path(input: &str) -> ServiceResult<PathBuf> {
    let raw = input.strip_prefix("file://").unwrap_or(input).trim();
    Ok(normalize_windows_drive_path(Path::new(raw)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_windows_drive_edge_cases() {
        // None cases
        assert!(split_windows_drive_path("").is_none());
        assert!(split_windows_drive_path("C:foo").is_none());
        assert!(split_windows_drive_path("1:/foo").is_none());
        assert!(split_windows_drive_path("CC:/foo").is_none());
        assert!(split_windows_drive_path("/foo/bar").is_none());
        assert!(split_windows_drive_path("/C:").is_none());
        assert!(split_windows_drive_path(r"\\?\C:\foo").is_none());

        // Success cases
        assert_eq!(
            split_windows_drive_path("C:/Users/Peter"),
            Some(('C', "Users/Peter".to_string()))
        );
        assert_eq!(
            split_windows_drive_path("/C:/Users/Peter"),
            Some(('C', "Users/Peter".to_string()))
        );
        assert_eq!(
            split_windows_drive_path(r"c:\Users\Peter"),
            Some(('C', "Users/Peter".to_string()))
        );
        assert_eq!(
            split_windows_drive_path("/c:/"),
            Some(('C', "".to_string()))
        );
        assert_eq!(split_windows_drive_path("Z:/"), Some(('Z', "".to_string())));
    }

    #[test]
    fn test_is_unc_path() {
        #[cfg(windows)]
        {
            assert!(is_unc_path(Path::new(r"\\server\share")));
            assert!(is_unc_path(Path::new(r"\\?\UNC\server\share")));
            assert!(is_unc_path(Path::new(r"\\server\share\folder")));
            assert!(!is_unc_path(Path::new(r"C:\foo")));
            assert!(!is_unc_path(Path::new(r"\\?\C:\foo")));
        }
        #[cfg(not(windows))]
        {
            assert!(!is_unc_path(Path::new(r"\\server\share")));
            assert!(!is_unc_path(Path::new(r"\\?\UNC\server\share")));
        }
        assert!(!is_unc_path(Path::new("/foo/bar")));
        assert!(!is_unc_path(Path::new("")));
    }

    #[test]
    fn test_strip_verbatim_prefix() {
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\MEDIASERVER\Movies")),
            PathBuf::from(r"\\MEDIASERVER\Movies")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\sub\file.txt")),
            PathBuf::from(r"\\server\share\sub\file.txt")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:\foo")),
            PathBuf::from(r"C:\foo")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\server\share")),
            PathBuf::from(r"\\server\share")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"C:\foo")),
            PathBuf::from(r"C:\foo")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new("/mnt/c/foo")),
            PathBuf::from("/mnt/c/foo")
        );
    }

    #[test]
    fn test_trim_trailing_separator() {
        #[cfg(windows)]
        {
            assert_eq!(
                trim_trailing_separator(PathBuf::from(r"C:\Users\Peter\")),
                PathBuf::from(r"C:\Users\Peter")
            );
            assert_eq!(
                trim_trailing_separator(PathBuf::from(r"\\server\share\")),
                PathBuf::from(r"\\server\share")
            );
            // Production pipeline: de-verbatimize, then trim.
            assert_eq!(
                trim_trailing_separator(strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\"))),
                PathBuf::from(r"\\server\share")
            );
            assert_eq!(
                trim_trailing_separator(PathBuf::from(r"C:\")),
                PathBuf::from(r"C:\")
            );
            assert_eq!(
                trim_trailing_separator(PathBuf::from(r"C:\Users\Peter")),
                PathBuf::from(r"C:\Users\Peter")
            );
        }
        assert_eq!(
            trim_trailing_separator(PathBuf::from("/usr/local/")),
            PathBuf::from("/usr/local")
        );
        assert_eq!(
            trim_trailing_separator(PathBuf::from("/")),
            PathBuf::from("/")
        );
        assert_eq!(
            trim_trailing_separator(PathBuf::from("/usr/local")),
            PathBuf::from("/usr/local")
        );
    }

    #[test]
    fn test_strip_prefix_platform() {
        #[cfg(windows)]
        {
            // UNC, exact case.
            assert_eq!(
                strip_prefix_platform(
                    Path::new(r"\\MEDIASERVER\Movies\sub\file.txt"),
                    Path::new(r"\\MEDIASERVER\Movies")
                ),
                Some(PathBuf::from(r"sub\file.txt"))
            );
            // UNC, case-insensitive server + share + leaf.
            assert_eq!(
                strip_prefix_platform(
                    Path::new(r"\\mediaserver\movies\FILE.txt"),
                    Path::new(r"\\MEDIASERVER\Movies")
                ),
                Some(PathBuf::from("FILE.txt"))
            );
            // UNC, outside the share.
            assert_eq!(
                strip_prefix_platform(
                    Path::new(r"\\other\share\file"),
                    Path::new(r"\\MEDIASERVER\Movies")
                ),
                None
            );
            // Verbatim UNC input is de-verbatimized before matching.
            let verbatim_path = strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\x"));
            let verbatim_base = strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share"));
            assert_eq!(
                strip_prefix_platform(&verbatim_path, &verbatim_base),
                Some(PathBuf::from("x"))
            );
            // Drive letter + component case-insensitive.
            assert_eq!(
                strip_prefix_platform(
                    Path::new(r"c:\users\peter\file.txt"),
                    Path::new(r"C:\Users\Peter")
                ),
                Some(PathBuf::from(r"file.txt"))
            );
            // Root drive match.
            assert_eq!(
                strip_prefix_platform(Path::new(r"C:\Users\Peter"), Path::new(r"C:\")),
                Some(PathBuf::from(r"Users\Peter"))
            );
        }
        #[cfg(not(windows))]
        {
            assert_eq!(
                strip_prefix_platform(
                    Path::new("/mnt/c/Users/file.txt"),
                    Path::new("/mnt/c/Users")
                ),
                Some(PathBuf::from("file.txt"))
            );
            // Case-sensitive on macOS/Linux.
            assert_eq!(
                strip_prefix_platform(
                    Path::new("/mnt/C/Users/file.txt"),
                    Path::new("/mnt/c/Users")
                ),
                None
            );
            assert_eq!(
                strip_prefix_platform(Path::new("/etc/passwd"), Path::new("/mnt/c/Users")),
                None
            );
        }
    }
}
