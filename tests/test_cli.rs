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

#[test]
fn test_disable_tools_single_tool() {
    let args = ["mcp-server", "-d", "read_text_file", "/path/to/dir"];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    assert_eq!(
        result.disabled_tool_names,
        Some(vec!["read_text_file".to_string()])
    );
}

#[test]
fn test_disable_tools_multiple_tools() {
    let args = [
        "mcp-server",
        "-d",
        "read_text_file,write_file,edit_file",
        "/path/to/dir",
    ];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    let mut expected = result.disabled_tool_names.unwrap();
    expected.sort();
    assert_eq!(expected, vec!["edit_file", "read_text_file", "write_file"]);
}

#[test]
fn test_disable_tools_case_insensitive() {
    let args = ["mcp-server", "-d", "Read_Text_File", "/path/to/dir"];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    assert_eq!(
        result.disabled_tool_names,
        Some(vec!["read_text_file".to_string()])
    );
}

#[test]
fn test_disable_tools_with_spaces() {
    let args = [
        "mcp-server",
        "-d",
        "read_text_file, write_file ",
        "/path/to/dir",
    ];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    let mut expected = result.disabled_tool_names.unwrap();
    expected.sort();
    assert_eq!(expected, vec!["read_text_file", "write_file"]);
}

#[test]
fn test_disable_tools_invalid_tool() {
    let args = ["mcp-server", "-d", "invalidtool", "/path/to/dir"];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_err());
    assert!(
        validated
            .unwrap_err()
            .contains("Invalid entry detected in the disable-tools list : 'invalidtool'")
    );
}

#[test]
fn test_disable_tools_empty_value() {
    let args = ["mcp-server", "-d", "", "/path/to/dir"];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    assert_eq!(result.disabled_tool_names, Some(vec![]));
}

#[test]
fn test_disable_tools_whitespace_only() {
    let args = ["mcp-server", "-d", "   ", "/path/to/dir"];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    assert_eq!(result.disabled_tool_names, Some(vec![]));
}

#[test]
fn test_disable_tools_long_flag() {
    let args = [
        "mcp-server",
        "--disable-tools",
        "read_text_file",
        "/path/to/dir",
    ];
    let mut result = parse_args(&args).unwrap();
    let validated = result.validate();
    assert!(validated.is_ok());
    assert_eq!(
        result.disabled_tool_names,
        Some(vec!["read_text_file".to_string()])
    );
}
