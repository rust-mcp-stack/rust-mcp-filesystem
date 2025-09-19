#[path = "common/common.rs"]
pub mod common;

use common::setup_service;
use rust_mcp_filesystem::tools::*;
use rust_mcp_sdk::schema::{ContentBlock, schema_utils::CallToolError};
use std::fs;

#[tokio::test]
async fn test_create_directory_new_directory() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let new_dir = temp_dir.join("dir1").join("new_dir");
    let params = CreateDirectoryTool {
        path: new_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectoryTool::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    let content = call_result.content.first().unwrap();

    match content {
        ContentBlock::TextContent(text_content) => {
            assert_eq!(
                text_content.text,
                format!(
                    "Successfully created directory {}",
                    new_dir.to_str().unwrap()
                )
            );
        }
        _ => panic!("Expected TextContent result"),
    }

    assert!(new_dir.is_dir());
}

#[tokio::test]
async fn test_create_directory_existing_directory() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let existing_dir = temp_dir.join("dir1").join("existing_dir");
    fs::create_dir_all(&existing_dir).unwrap();
    let params = CreateDirectoryTool {
        path: existing_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectoryTool::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    let content = call_result.content.first().unwrap();

    match content {
        ContentBlock::TextContent(text_content) => {
            assert_eq!(
                text_content.text,
                format!(
                    "Successfully created directory {}",
                    existing_dir.to_str().unwrap()
                )
            );
        }
        _ => panic!("Expected TextContent result"),
    }

    assert!(existing_dir.is_dir());
}

#[tokio::test]
async fn test_create_directory_nested() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let nested_dir = temp_dir.join("dir1").join("nested/subdir");
    let params = CreateDirectoryTool {
        path: nested_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectoryTool::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    let content = call_result.content.first().unwrap();

    match content {
        ContentBlock::TextContent(text_content) => {
            assert_eq!(
                text_content.text,
                format!(
                    "Successfully created directory {}",
                    nested_dir.to_str().unwrap()
                )
            );
        }
        _ => panic!("Expected TextContent result"),
    }
}

#[tokio::test]
async fn test_create_directory_outside_allowed() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let outside_dir = temp_dir.join("dir2").join("forbidden");
    let params = CreateDirectoryTool {
        path: outside_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectoryTool::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
    assert!(!outside_dir.exists());
}

#[tokio::test]
async fn test_create_directory_invalid_path() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let invalid_path = temp_dir.join("dir1").join("invalid\0dir");
    let params = CreateDirectoryTool {
        path: invalid_path
            .to_str()
            .map_or("invalid\0dir".to_string(), |s| s.to_string()),
    };

    let result = CreateDirectoryTool::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
}
