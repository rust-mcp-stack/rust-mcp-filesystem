#[path = "common/common.rs"]
pub mod common;

use common::{create_temp_file, setup_service};
use rust_mcp_filesystem::tools::*;
use rust_mcp_sdk::schema::{ContentBlock, schema_utils::CallToolError};
use std::{collections::HashSet, fs};

#[tokio::test]
async fn test_create_directory_new_directory() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let new_dir = temp_dir.join("dir1").join("new_dir");
    let params = CreateDirectory {
        path: new_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectory::run_tool(params, &service).await;
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
    let params = CreateDirectory {
        path: existing_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectory::run_tool(params, &service).await;
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
    let params = CreateDirectory {
        path: nested_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectory::run_tool(params, &service).await;
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
    let params = CreateDirectory {
        path: outside_dir.to_str().unwrap().to_string(),
    };

    let result = CreateDirectory::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
    assert!(!outside_dir.exists());
}

#[tokio::test]
async fn test_create_directory_invalid_path() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let invalid_path = temp_dir.join("dir1").join("invalid\0dir");
    let params = CreateDirectory {
        path: invalid_path
            .to_str()
            .map_or("invalid\0dir".to_string(), |s| s.to_string()),
    };

    let result = CreateDirectory::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
}

// Github Issue #54
// https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/54
#[tokio::test]
async fn ensure_tools_duplication() {
    let mut names = HashSet::new();
    let mut duplicate_names = vec![];

    let mut titles = HashSet::new();
    let mut duplicate_titles = vec![];

    let mut descriptions = HashSet::new();
    let mut duplicate_descriptions = vec![];

    for t in FileSystemTools::tools() {
        if !names.insert(t.name.to_string()) {
            duplicate_names.push(t.name.to_string());
        }

        if let Some(title) = t.title {
            if !titles.insert(title.to_string()) {
                duplicate_titles.push(title.to_string());
            }
        }

        if let Some(description) = t.description {
            if !descriptions.insert(description.to_string()) {
                duplicate_descriptions.push(description.to_string());
            }
        }
    }

    assert_eq!(duplicate_names.join(","), "");
    assert_eq!(duplicate_titles.join(","), "");
    assert_eq!(duplicate_descriptions.join(","), "");
}

#[tokio::test]
async fn test_diff_files_identical_text_files() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file1 = create_temp_file(&temp_dir.join("dir1"), "file1.txt", "Hello\nWorld\n");
    let file2 = create_temp_file(&temp_dir.join("dir1"), "file2.txt", "Hello\nWorld\n");

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    match call_result.content.first().unwrap() {
        ContentBlock::TextContent(text_content) => {
            assert!(text_content.text.contains("Files are identical"));
        }
        _ => panic!("Expected TextContent result"),
    }
}

#[tokio::test]
async fn test_diff_files_different_text_files() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file1 = create_temp_file(&temp_dir.join("dir1"), "file1.txt", "Hello\nWorld\n");
    let file2 = create_temp_file(&temp_dir.join("dir1"), "file2.txt", "Hello\nRust\n");

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    match call_result.content.first().unwrap() {
        ContentBlock::TextContent(text_content) => {
            assert!(text_content.text.contains("diff"));
            assert!(text_content.text.contains("-World") || text_content.text.contains("+Rust"));
        }
        _ => panic!("Expected TextContent result"),
    }
}

#[tokio::test]
async fn test_diff_files_binary_identical() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);

    // Create two identical binary files
    let binary_data = vec![0u8, 1, 2, 3, 255, 254];
    let file1 = temp_dir.join("dir1").join("file1.bin");
    let file2 = temp_dir.join("dir1").join("file2.bin");

    fs::write(&file1, &binary_data).unwrap();
    fs::write(&file2, &binary_data).unwrap();

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    match call_result.content.first().unwrap() {
        ContentBlock::TextContent(text_content) => {
            assert!(text_content.text.contains("Binary files are identical"));
            assert!(text_content.text.contains("SHA-256"));
        }
        _ => panic!("Expected TextContent result"),
    }
}

#[tokio::test]
async fn test_diff_files_binary_different() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);

    // Create two different binary files
    let binary_data1 = vec![0u8, 1, 2, 3, 255, 254];
    let binary_data2 = vec![0u8, 1, 2, 3, 255, 253]; // Last byte different
    let file1 = temp_dir.join("dir1").join("file1.bin");
    let file2 = temp_dir.join("dir1").join("file2.bin");

    fs::write(&file1, &binary_data1).unwrap();
    fs::write(&file2, &binary_data2).unwrap();

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    match call_result.content.first().unwrap() {
        ContentBlock::TextContent(text_content) => {
            assert!(text_content.text.contains("Binary files differ"));
            assert!(text_content.text.contains("SHA-256") || text_content.text.contains("File 1"));
        }
        _ => panic!("Expected TextContent result"),
    }
}

#[tokio::test]
async fn test_diff_files_outside_allowed_directory() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);

    // Create files: one in allowed dir, one outside
    let file1 = create_temp_file(&temp_dir.join("dir1"), "file1.txt", "Hello\n");

    // Create dir2 which is not allowed
    fs::create_dir_all(temp_dir.join("dir2")).unwrap();
    let file2 = create_temp_file(&temp_dir.join("dir2"), "file2.txt", "World\n");

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
}

#[tokio::test]
async fn test_diff_files_nonexistent_file() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file1 = create_temp_file(&temp_dir.join("dir1"), "file1.txt", "Hello\n");
    let file2 = temp_dir.join("dir1").join("nonexistent.txt");

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
}

#[tokio::test]
async fn test_diff_files_exceeds_size_limit() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);

    // Create a file larger than the limit
    let large_content = "x".repeat(1000);
    let file1 = create_temp_file(&temp_dir.join("dir1"), "file1.txt", &large_content);
    let file2 = create_temp_file(&temp_dir.join("dir1"), "file2.txt", &large_content);

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: Some(500), // Set limit to 500 bytes
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, CallToolError { .. }));
}

#[tokio::test]
async fn test_diff_files_multiline_changes() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let content1 = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";
    let content2 = "Line 1\nLine 2 modified\nLine 3\nLine 4 changed\nLine 5\n";
    let file1 = create_temp_file(&temp_dir.join("dir1"), "file1.txt", content1);
    let file2 = create_temp_file(&temp_dir.join("dir1"), "file2.txt", content2);

    let params = DiffFiles {
        path1: file1.to_str().unwrap().to_string(),
        path2: file2.to_str().unwrap().to_string(),
        max_file_size_bytes: None,
    };

    let result = DiffFiles::run_tool(params, &service).await;
    assert!(result.is_ok());
    let call_result = result.unwrap();

    assert_eq!(call_result.content.len(), 1);
    match call_result.content.first().unwrap() {
        ContentBlock::TextContent(text_content) => {
            assert!(text_content.text.contains("diff"));
            // Should show both changes
            assert!(text_content.text.contains("Line 2") || text_content.text.contains("Line 4"));
        }
        _ => panic!("Expected TextContent result"),
    }
}

#[tokio::test]
async fn adhoc() {}
