#[path = "common/common.rs"]
pub mod common;

use common::create_temp_file;
use common::setup_service;
use rust_mcp_filesystem::tools::{EditOperation, RegexEditOptions};
use std::fs;

/// Test simple regex replacement
#[tokio::test]
async fn test_regex_edit_simple() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.js",
        "function test() {\n  console.log('hello');\n}",
    );

    let edits = vec![EditOperation::Regex {
        pattern: r"function\s+(\w+)".to_string(),
        replacement: "async function $1".to_string(),
        options: None,
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    assert!(result.contains("-function test"));
    assert!(result.contains("+async function test"));

    let new_content = tokio::fs::read_to_string(&file_path).await.unwrap();
    assert!(new_content.contains("async function test"));
}

/// Test regex with case insensitive option
#[tokio::test]
async fn test_regex_edit_case_insensitive() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "Hello World\nHELLO WORLD\nhello world",
    );

    let edits = vec![EditOperation::Regex {
        pattern: "hello".to_string(),
        replacement: "Hi".to_string(),
        options: Some(RegexEditOptions {
            case_insensitive: Some(true),
            multiline: None,
            dot_all: None,
            max_replacements: None,
        }),
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(new_content, "Hi World\nHi WORLD\nHi world");
}

/// Test regex with max replacements
#[tokio::test]
async fn test_regex_edit_max_replacements() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "foo bar foo baz foo",
    );

    let edits = vec![EditOperation::Regex {
        pattern: "foo".to_string(),
        replacement: "FOO".to_string(),
        options: Some(RegexEditOptions {
            case_insensitive: None,
            multiline: None,
            dot_all: None,
            max_replacements: Some(2),
        }),
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    // Should replace only first 2 occurrences
    assert_eq!(new_content, "FOO bar FOO baz foo");
}

/// Test regex with multiline mode
#[tokio::test]
async fn test_regex_edit_multiline() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3",
    );

    let edits = vec![EditOperation::Regex {
        pattern: "^line".to_string(),
        replacement: "LINE".to_string(),
        options: Some(RegexEditOptions {
            case_insensitive: None,
            multiline: Some(true),
            dot_all: None,
            max_replacements: None,
        }),
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(new_content, "LINE1\nLINE2\nLINE3");
}

/// Test regex with dot_all mode
#[tokio::test]
async fn test_regex_edit_dot_all() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.html",
        "<div>hello\nworld</div>",
    );

    let edits = vec![EditOperation::Regex {
        pattern: "<div>(.*?)</div>".to_string(),
        replacement: "<span>$1</span>".to_string(),
        options: Some(RegexEditOptions {
            case_insensitive: None,
            multiline: None,
            dot_all: Some(true),
            max_replacements: None,
        }),
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(new_content, "<span>hello\nworld</span>");
}

/// Test mixing exact and regex edits
#[tokio::test]
async fn test_mixed_exact_and_regex_edits() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.js",
        "const version = '1.0.0';\nfunction test() {}",
    );

    let edits = vec![
        EditOperation::Exact {
            old_text: "const version = '1.0.0';".to_string(),
            new_text: "const version = '2.0.0';".to_string(),
        },
        EditOperation::Regex {
            pattern: r"function\s+(\w+)".to_string(),
            replacement: "async function $1".to_string(),
            options: None,
        },
    ];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert!(new_content.contains("const version = '2.0.0';"));
    assert!(new_content.contains("async function test"));
}

/// Test line range with exact edit
#[tokio::test]
async fn test_line_range_exact_edit() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3\nline4\nline5",
    );

    let edits = vec![EditOperation::Exact {
        old_text: "line3".to_string(),
        new_text: "LINE3".to_string(),
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, Some("2-4".to_string()))
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(new_content, "line1\nline2\nLINE3\nline4\nline5");
}

/// Test line range with regex edit
#[tokio::test]
async fn test_line_range_regex_edit() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3\nline4\nline5",
    );

    let edits = vec![EditOperation::Regex {
        pattern: "line".to_string(),
        replacement: "LINE".to_string(),
        options: None,
    }];

    // Only apply to lines 2-4 (1-based), should affect line2, line3, line4
    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, Some("2-4".to_string()))
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(new_content, "line1\nLINE2\nLINE3\nLINE4\nline5");
}

/// Test invalid regex pattern
#[tokio::test]
async fn test_regex_edit_invalid_pattern() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "hello world",
    );

    let edits = vec![EditOperation::Regex {
        pattern: "[invalid(".to_string(),  // Invalid regex
        replacement: "test".to_string(),
        options: None,
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await;

    assert!(result.is_err());
}

/// Test invalid line range
#[tokio::test]
async fn test_invalid_line_range() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3",
    );

    let edits = vec![EditOperation::Exact {
        old_text: "line2".to_string(),
        new_text: "LINE2".to_string(),
    }];

    // Invalid format
    let result = service
        .apply_file_edits(&file_path, edits.clone(), Some(false), None, Some("invalid".to_string()))
        .await;
    assert!(result.is_err());

    // Start >= end
    let result = service
        .apply_file_edits(&file_path, edits.clone(), Some(false), None, Some("5-2".to_string()))
        .await;
    assert!(result.is_err());

    // Start beyond file
    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, Some("10-20".to_string()))
        .await;
    assert!(result.is_err());
}

/// Test capture groups in replacement
#[tokio::test]
async fn test_regex_capture_groups() {
    let (temp_dir, service, _allowed_dirs) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.js",
        "import React from 'react';",
    );

    let edits = vec![EditOperation::Regex {
        pattern: r"import\s+(\w+)\s+from\s+'([^']+)'".to_string(),
        replacement: "import { $1 } from '$2'".to_string(),
        options: None,
    }];

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None, None)
        .await
        .unwrap();

    let new_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(new_content, "import { React } from 'react';");
}
