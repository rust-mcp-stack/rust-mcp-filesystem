#[path = "common/common.rs"]
pub mod common;

use common::parse_args;

#[test]
fn test_parse_with_single_directory() {
    let args = ["mcp-server", "/path/to/dir"];
    let result = parse_args(&args).unwrap();
    assert_eq!(result.allowed_directories, vec!["/path/to/dir"]);
    assert!(!result.allow_write);
}

#[test]
fn test_parse_with_multiple_directories() {
    let args = ["mcp-server", "/dir1", "/dir2", "/dir3"];
    let result = parse_args(&args).unwrap();
    assert_eq!(result.allowed_directories, vec!["/dir1", "/dir2", "/dir3"]);
    assert!(!result.allow_write);
}

#[test]
fn test_parse_with_write_flag_short() {
    let args = ["mcp-server", "-w", "/path/to/dir"];
    let result = parse_args(&args).unwrap();
    assert_eq!(result.allowed_directories, vec!["/path/to/dir"]);
    assert!(result.allow_write);
}

#[test]
fn test_parse_with_write_flag_long() {
    let args = ["mcp-server", "--allow-write", "/path/to/dir"];
    let result = parse_args(&args).unwrap();
    assert_eq!(result.allowed_directories, vec!["/path/to/dir"]);
    assert!(result.allow_write);
}

#[test]
fn test_missing_required_directories() {
    let args = ["mcp-server"];

    // parse should pass
    let result = parse_args(&args);
    assert!(result.is_ok());

    let result = result.unwrap().validate();
    assert!(
        matches!(result, Err(message) if message.contains("is required when `--enable-roots` is not provided"))
    );
}

#[test]
fn test_version_flag() {
    let args = ["mcp-server", "--version"];
    let result = parse_args(&args);
    // Version flag should cause clap to exit early, so we expect an error
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}

#[test]
fn test_help_flag() {
    let args = ["mcp-server", "--help"];
    let result = parse_args(&args);
    // Help flag should cause clap to exit early, so we expect an error
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), clap::error::ErrorKind::DisplayHelp);
    }
}

#[test]
fn test_invalid_flag() {
    let args = ["mcp-server", "--invalid", "/path/to/dir"];
    let result = parse_args(&args);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}
