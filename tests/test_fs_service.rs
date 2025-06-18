#[path = "common/common.rs"]
pub mod common;

use async_zip::tokio::write::ZipFileWriter;
use common::create_temp_dir;
use common::create_temp_file;
use common::create_temp_file_info;
use common::get_temp_dir;
use common::setup_service;
use dirs::home_dir;
use grep::matcher::Match;
use rust_mcp_filesystem::error::ServiceError;
use rust_mcp_filesystem::fs_service::file_info::FileInfo;
use rust_mcp_filesystem::fs_service::utils::*;
use rust_mcp_filesystem::fs_service::FileSystemService;
use rust_mcp_filesystem::tools::EditOperation;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs as tokio_fs;
use tokio_util::compat::TokioAsyncReadCompatExt;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn test_try_new_success() {
    let temp_dir = get_temp_dir();
    let dir_path = temp_dir.to_str().unwrap().to_string();

    let result = FileSystemService::try_new(&[dir_path]);
    assert!(result.is_ok());
    let service = result.unwrap();
    assert_eq!(service.allowed_directories().as_ref(), vec![temp_dir]);
}

#[test]
#[should_panic(expected = "Error: /does/not/exist is not a directory")]
fn test_try_new_invalid_directory() {
    let _ = FileSystemService::try_new(&["/does/not/exist".to_string()]);
}

#[test]
fn test_allowed_directories() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let allowed = service.allowed_directories();
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0], temp_dir.join("dir1"));
}

#[tokio::test]
async fn test_validate_path_allowed() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = temp_dir.join("dir1").join("test.txt");
    create_temp_file(temp_dir.join("dir1").as_path(), "test.txt", "content");
    let result = service.validate_path(&file_path);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), file_path);
}

#[tokio::test]
async fn test_validate_path_denied() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let outside_path = temp_dir.join("dir2").join("test.txt");
    let result = service.validate_path(&outside_path);
    assert!(matches!(result, Err(ServiceError::FromString(_))));
}

#[test]
fn test_normalize_line_endings() {
    let input = "line1\r\nline2\r\nline3";
    let normalized = normalize_line_endings(input);
    assert_eq!(normalized, "line1\nline2\nline3");
}

#[test]
fn test_contains_symlink_no_symlink() {
    let temp_dir = get_temp_dir();
    let file_path = create_temp_file(&temp_dir, "test.txt", "content");
    let result = contains_symlink(file_path).unwrap();
    assert!(!result);
}

// Symlink test is platform-dependent , it require administrator privileges on some systems
#[cfg(unix)]
#[test]
fn test_contains_symlink_with_symlink() {
    use common::create_temp_file;

    let temp_dir = get_temp_dir();
    let target_path = create_temp_file(&temp_dir, "target.txt", "content");
    let link_path = temp_dir.join("link.txt");
    std::os::unix::fs::symlink(&target_path, &link_path).unwrap();
    let result = contains_symlink(&link_path).unwrap();
    assert!(result);
}

#[tokio::test]
async fn test_get_file_stats() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(temp_dir.join("dir1").as_path(), "test.txt", "content");
    let result = service.get_file_stats(&file_path).await.unwrap();
    assert_eq!(result.size, 7); // "content" is 7 bytes
    assert!(result.is_file);
    assert!(!result.is_directory);
    assert!(result.created.is_some());
    assert!(result.modified.is_some());
    assert!(result.accessed.is_some());
}

#[tokio::test]
async fn test_zip_directory() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);

    let dir_path = temp_dir.join("dir1");
    create_temp_file(&dir_path, "file1.txt", "content1");
    create_temp_file(&dir_path, "file2.txt", "content2");
    let zip_path = dir_path.join("output.zip");
    let result = service
        .zip_directory(
            dir_path.to_str().unwrap().to_string(),
            "*.txt".to_string(),
            zip_path.to_str().unwrap().to_string(),
        )
        .await
        .unwrap();
    assert!(zip_path.exists());
    assert!(result.contains("Successfully compressed"));
    assert!(result.contains("output.zip"));
}

#[tokio::test]
async fn test_zip_directory_already_exists() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let dir_path = temp_dir.join("dir1");
    let zip_path = create_temp_file(&dir_path, "output.zip", "dummy");
    let result = service
        .zip_directory(
            dir_path.to_str().unwrap().to_string(),
            "*.txt".to_string(),
            zip_path.to_str().unwrap().to_string(),
        )
        .await;
    assert!(matches!(
        result,
        Err(ServiceError::IoError(ref e)) if e.kind() == std::io::ErrorKind::AlreadyExists
    ));
}

#[tokio::test]
async fn test_zip_files() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let dir_path = temp_dir.join("dir1");

    let file1 = create_temp_file(dir_path.as_path(), "file1.txt", "content1");
    let file2 = create_temp_file(dir_path.as_path(), "file2.txt", "content2");
    let zip_path = dir_path.join("output.zip");
    let result = service
        .zip_files(
            vec![
                file1.to_str().unwrap().to_string(),
                file2.to_str().unwrap().to_string(),
            ],
            zip_path.to_str().unwrap().to_string(),
        )
        .await
        .unwrap();
    assert!(zip_path.exists());
    assert!(result.contains("Successfully compressed 2 files"));
    assert!(result.contains("output.zip"));
}

#[tokio::test]
async fn test_zip_files_empty_input() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let zip_path = temp_dir.join("output.zip");
    let result = service
        .zip_files(vec![], zip_path.to_str().unwrap().to_string())
        .await;
    assert!(matches!(
        result,
        Err(ServiceError::IoError(ref e)) if e.kind() == std::io::ErrorKind::InvalidInput
    ));
}

#[tokio::test]
async fn test_unzip_file() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let dir_path = temp_dir.join("dir1");
    let file1 = create_temp_file(&dir_path, "file1.txt", "content1");
    let zip_path = dir_path.join("output.zip");
    service
        .zip_files(
            vec![file1.to_str().unwrap().to_string()],
            zip_path.to_str().unwrap().to_string(),
        )
        .await
        .unwrap();
    let extract_dir = dir_path.join("extracted");
    let result = service
        .unzip_file(zip_path.to_str().unwrap(), extract_dir.to_str().unwrap())
        .await
        .unwrap();
    assert!(extract_dir.join("file1.txt").exists());
    assert!(result.contains("Successfully extracted 1 file"));
}

#[tokio::test]
async fn test_unzip_file_non_existent() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let temp_dir = temp_dir.join("dir1");
    let zip_path = temp_dir.join("non_existent.zip");
    let extract_dir = temp_dir.join("extracted");
    let result = service
        .unzip_file(zip_path.to_str().unwrap(), extract_dir.to_str().unwrap())
        .await;

    assert!(matches!(
        result,
        Err(ServiceError::IoError(ref e)) if e.kind() == std::io::ErrorKind::NotFound
    ));
}

#[tokio::test]
async fn test_read_file() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(temp_dir.join("dir1").as_path(), "test.txt", "content");
    let content = service.read_file(&file_path).await.unwrap();
    assert_eq!(content, "content");
}

#[tokio::test]
async fn test_create_directory() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let new_dir = temp_dir.join("dir1").join("new_dir");
    let result = service.create_directory(&new_dir).await;

    assert!(result.is_ok());
    assert!(new_dir.is_dir());
}

#[tokio::test]
async fn test_move_file() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let src_path = create_temp_file(temp_dir.join("dir1").as_path(), "src.txt", "content");
    let dest_path = temp_dir.join("dir1").join("dest.txt");
    let result = service.move_file(&src_path, &dest_path).await;
    assert!(result.is_ok());
    assert!(!src_path.exists());
    assert!(dest_path.exists());
}

#[tokio::test]
async fn test_list_directory() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let dir_path = temp_dir.join("dir1");
    create_temp_file(&dir_path, "file1.txt", "content1");
    create_temp_file(&dir_path, "file2.txt", "content2");
    let entries = service.list_directory(&dir_path).await.unwrap();
    let names: Vec<_> = entries
        .into_iter()
        .map(|e| e.file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"file1.txt".to_string()));
    assert!(names.contains(&"file2.txt".to_string()));
}

#[tokio::test]
async fn test_write_file() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = temp_dir.join("dir1").join("test.txt");
    let content = "new content".to_string();
    let result = service.write_file(&file_path, &content).await;
    assert!(result.is_ok());
    assert_eq!(tokio_fs::read_to_string(&file_path).await.unwrap(), content);
}

#[test]
fn test_search_files() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let dir_path = temp_dir.join("dir1");
    create_temp_file(&dir_path, "test1.txt", "content");
    create_temp_file(&dir_path, "test2.doc", "content");
    let result = service
        .search_files(&dir_path, "*.txt".to_string(), vec![])
        .unwrap();
    let names: Vec<_> = result
        .into_iter()
        .map(|e| e.file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["test1.txt"]);
}

#[test]
fn test_search_files_with_exclude() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let dir_path = temp_dir.join("dir1");
    create_temp_file(&dir_path, "test1.txt", "content");
    create_temp_file(&dir_path, "test2.txt", "content");
    let result = service
        .search_files(
            &dir_path,
            "*.txt".to_string(),
            vec!["test2.txt".to_string()],
        )
        .unwrap();
    let names: Vec<_> = result
        .into_iter()
        .map(|e| e.file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["test1.txt"]);
}

#[test]
fn test_create_unified_diff() {
    let (_, service) = setup_service(vec![]);
    let original = "line1\nline2\nline3".to_string();
    let new = "line1\nline4\nline3".to_string();
    let diff = service.create_unified_diff(&original, &new, Some("test.txt".to_string()));
    assert!(diff.contains("Index: test.txt"));
    assert!(diff.contains("--- test.txt\toriginal"));
    assert!(diff.contains("+++ test.txt\tmodified"));
    assert!(diff.contains("-line2"));
    assert!(diff.contains("+line4"));
}

#[tokio::test]
async fn test_apply_file_edits() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3",
    );
    let edits = vec![EditOperation {
        old_text: "line2".to_string(),
        new_text: "line4".to_string(),
    }];
    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None)
        .await
        .unwrap();
    assert!(result.contains("Index:"));
    assert!(result.contains("-line2"));
    assert!(result.contains("+line4"));
    let new_content = tokio_fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(new_content, "line1\nline4\nline3");
}

#[tokio::test]
async fn test_apply_file_edits_dry_run() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3",
    );
    let edits = vec![EditOperation {
        old_text: "line2".to_string(),
        new_text: "line4".to_string(),
    }];
    let result = service
        .apply_file_edits(&file_path, edits, Some(true), None)
        .await
        .unwrap();
    assert!(result.contains("Index:"));
    assert!(result.contains("-line2"));
    assert!(result.contains("+line4"));
    let content = tokio_fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "line1\nline2\nline3"); // Unchanged due to dry run
}

#[tokio::test]
async fn test_apply_file_edits_no_match() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test.txt",
        "line1\nline2\nline3",
    );
    let edits = vec![EditOperation {
        old_text: "non_existent".to_string(),
        new_text: "line4".to_string(),
    }];
    let result = service
        .apply_file_edits(&file_path, edits, Some(false), None)
        .await;
    assert!(matches!(result, Err(ServiceError::RpcError(_))));
}

#[test]
fn test_format_system_time() {
    let now = SystemTime::now();
    let formatted = format_system_time(now);
    // Check that the output matches the expected format (e.g., "Sat Apr 12 2025 14:30:45 +00:00")
    assert!(formatted.contains("202")); // Year should appear
    assert!(formatted.contains(":")); // Time should have colons
    assert!(formatted.contains("+") || formatted.contains("-")); // Timezone offset
}

#[cfg(unix)]
#[test]
fn test_format_permissions_unix() {
    use rust_mcp_filesystem::fs_service::utils::format_permissions;

    let temp_dir = get_temp_dir();
    let file_path = temp_dir.join("test.txt");
    File::create(&file_path).unwrap();

    // Set specific permissions (e.g., rw-r--r--)
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();
    let metadata = fs::metadata(&file_path).unwrap();
    let formatted = format_permissions(&metadata);
    assert_eq!(formatted, "0644");

    // Test directory permissions
    let dir_metadata = fs::metadata(temp_dir).unwrap();
    let dir_formatted = format_permissions(&dir_metadata);
    assert!(dir_formatted.starts_with("0")); // Should be octal
}

#[cfg(windows)]
#[test]
fn test_format_permissions_windows() {
    let temp_dir = get_temp_dir();
    let file_path = temp_dir.join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"test").unwrap();
    file.flush().unwrap();

    // Set read-only
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file_path, perms).unwrap();

    let metadata = fs::metadata(&file_path).unwrap();
    let formatted = format_permissions(&metadata);
    assert_eq!(formatted, "-r"); // Regular file, read-only

    // Test directory
    let dir_metadata = fs::metadata(temp_dir).unwrap();
    let dir_formatted = format_permissions(&dir_metadata);
    assert_eq!(dir_formatted, "dw"); // Directory, typically writable
}

#[test]
fn test_normalize_path() {
    let temp_dir = get_temp_dir();
    let file_path = temp_dir.join("test.txt");
    File::create(&file_path).unwrap();

    let normalized = normalize_path(&file_path);
    assert_eq!(normalized, file_path);

    // Test non-existent path
    let non_existent = Path::new("/does/not/exist");
    let normalized_non_existent = normalize_path(non_existent);
    assert_eq!(normalized_non_existent, non_existent.to_path_buf());
}

#[test]
fn test_expand_home() {
    // Test with ~ path
    let home_path = PathBuf::from("~/test");
    let expanded = expand_home(home_path.clone());
    if let Some(home) = home_dir() {
        assert_eq!(expanded, home.join("test"));
    } else {
        assert_eq!(expanded, home_path); // No home dir, return original
    }

    // Test non-~ path
    let regular_path = PathBuf::from("/absolute/path");
    let expanded_regular = expand_home(regular_path.clone());
    assert_eq!(expanded_regular, regular_path);
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(500), "500 bytes");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1500), "1.46 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
    assert_eq!(format_bytes(1500 * 1024 * 1024), "1.46 GB");
}

#[tokio::test]
async fn test_write_zip_entry() {
    let temp_dir = get_temp_dir();
    let input_path = temp_dir.join("input.txt");
    let zip_path = temp_dir.join("output.zip");

    // Create a test file
    let content = b"Hello, zip!";
    let mut input_file = File::create(&input_path).unwrap();
    input_file.write_all(content).unwrap();
    input_file.flush().unwrap();

    // Create zip file
    let zip_file = tokio::fs::File::create(&zip_path).await.unwrap();
    let mut zip_writer = ZipFileWriter::new(zip_file.compat());

    // Write zip entry
    let result = write_zip_entry("test.txt", &input_path, &mut zip_writer).await;
    assert!(result.is_ok());

    // Close the zip writer
    zip_writer.close().await.unwrap();

    // Verify the zip file exists and has content
    let zip_metadata = fs::metadata(&zip_path).unwrap();
    assert!(zip_metadata.len() > 0);
}

#[tokio::test]
async fn test_write_zip_entry_non_existent_file() {
    let temp_dir = get_temp_dir();
    let zip_path = temp_dir.join("output.zip");
    let non_existent_path = temp_dir.join("does_not_exist.txt");

    let zip_file = tokio::fs::File::create(&zip_path).await.unwrap();
    let mut zip_writer = ZipFileWriter::new(zip_file.compat());

    let result = write_zip_entry("test.txt", &non_existent_path, &mut zip_writer).await;
    assert!(result.is_err());
}

#[test]
fn test_file_info_for_regular_file() {
    let (_dir, file_info) = create_temp_file_info(b"Hello, world!");
    assert_eq!(file_info.size, 13); // "Hello, world!" is 13 bytes
    assert!(file_info.is_file);
    assert!(!file_info.is_directory);
    assert!(file_info.created.is_some());
    assert!(file_info.modified.is_some());
    assert!(file_info.accessed.is_some());
}

#[test]
fn test_file_info_for_directory() {
    let (_dir, file_info) = create_temp_dir();
    assert!(file_info.is_directory);
    assert!(!file_info.is_file);
    assert!(file_info.created.is_some());
    assert!(file_info.modified.is_some());
    assert!(file_info.accessed.is_some());
}

#[test]
fn test_display_format_for_file() {
    let (_dir, file_info) = create_temp_file_info(b"Test content");
    let display_output = file_info.to_string();

    // Since permissions and exact times may vary, we just checking the key parts
    assert!(display_output.contains("size: 12"));
    assert!(display_output.contains("isDirectory: false"));
    assert!(display_output.contains("isFile: true"));
    assert!(display_output.contains("created:"));
    assert!(display_output.contains("modified:"));
    assert!(display_output.contains("accessed:"));
    assert!(display_output.contains("permissions:"));
}

#[test]
fn test_display_format_for_empty_timestamps() {
    // Create a FileInfo with no timestamps
    let metadata = fs::metadata(".").unwrap();
    let file_info = FileInfo {
        size: 123,
        created: None,
        modified: None,
        accessed: None,
        is_directory: false,
        is_file: true,
        metadata: metadata.clone(),
    };

    let display_output = file_info.to_string();

    // Only key parts
    assert!(display_output.contains("size: 123"));
    assert!(display_output.contains("created: \n"));
    assert!(display_output.contains("modified: \n"));
    assert!(display_output.contains("accessed: \n"));
    assert!(display_output.contains("isDirectory: false"));
    assert!(display_output.contains("isFile: true"));
}

#[tokio::test]
async fn test_apply_file_edits_mixed_indentation() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test_indent.txt",
        r#"
            // some descriptions
			const categories = [
				{
					title: 'Подготовка и исследование',
					keywords: ['изуч', 'исследов', 'анализ', 'подготов', 'планиров'],
					tasks: [] as any[]
				},
			];
		// some other descriptions
        "#,
    );
    // different indentation
    let edits = vec![EditOperation {
        old_text: r#"const categories = [
				{
					title: 'Подготовка и исследование',
						keywords: ['изуч', 'исследов', 'анализ', 'подготов', 'планиров'],
					tasks: [] as any[]
				},
			];"#
        .to_string(),
        new_text: r#"const categories = [
				{
					title: 'Подготовка и исследование',
					description: 'Анализ требований и подготовка к разработке',
					keywords: ['изуч', 'исследов', 'анализ', 'подготов', 'планиров'],
					tasks: [] as any[]
				},
			];"#
        .to_string(),
    }];

    let out_file = temp_dir.join("dir1").join("out_indent.txt");

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), Some(out_file.as_path()))
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_apply_file_edits_mixed_indentation_2() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file_path = create_temp_file(
        temp_dir.join("dir1").as_path(),
        "test_indent.txt",
        r#"
            // some descriptions
			const categories = [
				{
					title: 'Подготовка и исследование',
					keywords: ['изуч', 'исследов', 'анализ', 'подготов', 'планиров'],
					tasks: [] as any[]
				},
			];
		// some other descriptions
        "#,
    );
    // different indentation
    let edits = vec![EditOperation {
        old_text: r#"const categories = [
				{
					title: 'Подготовка и исследование',
			keywords: ['изуч', 'исследов', 'анализ', 'подготов', 'планиров'],
					tasks: [] as any[]
				},
			];"#
        .to_string(),
        new_text: r#"const categories = [
				{
					title: 'Подготовка и исследование',
					description: 'Анализ требований и подготовка к разработке',
					keywords: ['изуч', 'исследов', 'анализ', 'подготов', 'планиров'],
					tasks: [] as any[]
				},
			];"#
        .to_string(),
    }];

    let out_file = temp_dir.join("dir1").join("out_indent.txt");

    let result = service
        .apply_file_edits(&file_path, edits, Some(false), Some(out_file.as_path()))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_exact_match() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);

    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "tets_file1.txt",
        "hello world\n",
    );

    let edit = EditOperation {
        old_text: "hello world".to_string(),
        new_text: "hello universe".to_string(),
    };

    let result = service
        .apply_file_edits(file.as_path(), vec![edit], Some(false), None)
        .await
        .unwrap();

    let modified_content = fs::read_to_string(file.as_path()).unwrap();
    assert_eq!(modified_content, "hello universe\n");
    assert!(result.contains("-hello world\n+hello universe"));
}

#[tokio::test]
async fn test_exact_match_edit2() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file1.txt",
        "hello world\n",
    );

    let edits = vec![EditOperation {
        old_text: "hello world\n".into(),
        new_text: "hello Rust\n".into(),
    }];

    let result = service
        .apply_file_edits(&file, edits, Some(false), None)
        .await;

    assert!(result.is_ok());
    let updated_content = fs::read_to_string(&file).unwrap();
    assert_eq!(updated_content, "hello Rust\n");
}

#[tokio::test]
async fn test_line_by_line_match_with_indent() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file2.rs",
        "    let x = 42;\n    println!(\"{}\");\n",
    );

    let edits = vec![EditOperation {
        old_text: "let x = 42;\nprintln!(\"{}\");\n".into(),
        new_text: "let x = 43;\nprintln!(\"x = {}\", x)".into(),
    }];

    let result = service
        .apply_file_edits(&file, edits, Some(false), None)
        .await;

    assert!(result.is_ok());

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("let x = 43;"));
    assert!(content.contains("println!(\"x = {}\", x)"));
}

#[tokio::test]
async fn test_dry_run_mode() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file4.sh",
        "echo hello\n",
    );

    let edits = vec![EditOperation {
        old_text: "echo hello\n".into(),
        new_text: "echo world\n".into(),
    }];

    let result = service
        .apply_file_edits(&file, edits, Some(true), None)
        .await;
    assert!(result.is_ok());

    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content, "echo hello\n"); // Should not be modified
}

#[tokio::test]
async fn test_save_to_different_path() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let orig_file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file5.txt",
        "foo = 1\n",
    );

    let save_to = temp_dir.as_path().join("dir1").join("saved_output.txt");

    let edits = vec![EditOperation {
        old_text: "foo = 1\n".into(),
        new_text: "foo = 2\n".into(),
    }];

    let result = service
        .apply_file_edits(&orig_file, edits, Some(false), Some(&save_to))
        .await;

    assert!(result.is_ok());

    let original_content = fs::read_to_string(&orig_file).unwrap();
    let saved_content = fs::read_to_string(&save_to).unwrap();
    assert_eq!(original_content, "foo = 1\n");
    assert_eq!(saved_content, "foo = 2\n");
}

#[tokio::test]
async fn test_diff_backtick_formatting() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file6.md",
        "```\nhello\n```\n",
    );

    let edits = vec![EditOperation {
        old_text: "```\nhello\n```".into(),
        new_text: "```\nworld\n```".into(),
    }];

    let result = service
        .apply_file_edits(&file, edits, Some(true), None)
        .await;
    assert!(result.is_ok());

    let diff = result.unwrap();
    assert!(diff.contains("diff"));
    assert!(diff.starts_with("```")); // Should start with fenced backticks
}

#[tokio::test]
async fn test_no_edits_provided() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file7.toml",
        "enabled = true\n",
    );

    let result = service
        .apply_file_edits(&file, vec![], Some(false), None)
        .await;
    assert!(result.is_ok());

    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content, "enabled = true\n");
}

#[tokio::test]
async fn test_preserve_windows_line_endings() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_file.txt",
        "line1\r\nline2\r\n",
    );

    let edits = vec![EditOperation {
        old_text: "line1\nline2".into(), // normalized format
        new_text: "updated1\nupdated2".into(),
    }];

    let result = service
        .apply_file_edits(&file, edits, Some(false), None)
        .await;
    assert!(result.is_ok());

    let output = std::fs::read_to_string(&file).unwrap();
    assert_eq!(output, "updated1\r\nupdated2\r\n"); // Line endings preserved!
}

#[tokio::test]
async fn test_preserve_unix_line_endings() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "unix_line_file.txt",
        "line1\nline2\n",
    );

    let edits = vec![EditOperation {
        old_text: "line1\nline2".into(),
        new_text: "updated1\nupdated2".into(),
    }];

    let result = service
        .apply_file_edits(&file, edits, Some(false), None)
        .await;

    assert!(result.is_ok());

    let updated = std::fs::read_to_string(&file).unwrap();
    assert_eq!(updated, "updated1\nupdated2\n"); // Still uses \n endings
}

#[tokio::test]
// Issue #19: https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/19
async fn test_panic_on_out_of_bounds_edit() {
    let (temp_dir, service) = setup_service(vec!["dir1".to_string()]);

    // Set up an edit that expects to match 5 lines
    let edit = EditOperation {
        old_text: "line e\n".repeat(41).to_string(),
        new_text: "replaced content".to_string(),
    };

    // Set up your file content with only 2 lines
    let file_content = "line A\nline B\n";
    let test_path = create_temp_file(
        &temp_dir.as_path().join("dir1"),
        "test_input.txt",
        file_content,
    );

    let result = service
        .apply_file_edits(&test_path, vec![edit], Some(true), None)
        .await;

    // It should panic without the fix, or return an error after applying the fix
    assert!(result.is_err());
}

#[tokio::test]
async fn test_content_search() {
    let (temp_dir, service) = setup_service(vec!["dir_search".to_string()]);
    let file = create_temp_file(
        &temp_dir.as_path().join("dir_search"),
        "file_to_search.txt",
        r#"For the Doctor Watsons of this world, as opposed to the Sherlock
        Holmeses, success in the province of detective work must always
        be, to a very large extent, the result of luck. Sherlock Holmes
        can extract a clew from a wisp of straw or a flake of cigar ash;
        but Doctor Watso2n has to have it taken out for him and dusted,
        and exhibited clearly, with Watso\d*n a label attached."#,
    );

    let query = r#"Watso\d*n"#;

    // search as regex
    let result = service.content_search(query, &file, Some(true)).unwrap();

    assert!(result.is_some());
    let result = result.unwrap();

    assert_eq!(result.file_path, file);
    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.matches[0].line_number, 1);
    assert_eq!(result.matches[1].line_number, 5);
    assert_eq!(
        result.matches[0].line_text.trim(),
        "For the Doctor Watsons of this world, as opposed to the Sherlock"
    );
    assert_eq!(
        result.matches[1].line_text.trim(),
        "but Doctor Watso2n has to have it taken out for him and dusted,"
    );

    // search as literal
    let result = service.content_search(query, &file, Some(false)).unwrap();
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].line_number, 6);
    assert_eq!(
        result.matches[0].line_text.trim(),
        "and exhibited clearly, with Watso\\d*n a label attached."
    );
}

#[test]
fn test_match_near_start_short_line() {
    let (_, service) = setup_service(vec!["dir_search".to_string()]);

    let line = "match this text";
    let m = Match::new(0, 5);
    let result = service.extract_snippet(line, m, Some(20), Some(5));

    // Start at 0, should not prepend ...
    // Full line is shorter than SNIPPET_MAX_LENGTH
    assert_eq!(result, "match this text");
}

#[tokio::test]
async fn test_snippet_back_chars() {
    let (_, service) = setup_service(vec!["dir_search".to_string()]);
    let line = "this is a long enough line for testing match in middle";
    let m = Match::new(40, 45);
    let result = service.extract_snippet(line, m, Some(20), Some(5));

    assert!(result.starts_with("..."));
    assert!(!result.ends_with("..."));
    assert!(result.contains("match"));

    // larger text, truncates at the end
    let line = "this is a long enough line for testing match in middles .";
    let m = Match::new(40, 45);
    let result = service.extract_snippet(line, m, Some(20), Some(5));
    assert!(result.starts_with("..."));
    assert!(result.ends_with("..."));
    assert!(result.contains("match"));
}

#[test]
fn test_match_triggers_only_end_ellipsis() {
    let (_, service) = setup_service(vec!["dir_search".to_string()]);

    let line = "match is at start but line is long";
    let m = Match::new(0, 5);

    let result = service.extract_snippet(line, m, Some(10), Some(5));

    // Only ends in ellipsis
    assert!(!result.starts_with("..."));
    assert!(result.ends_with("..."));
}

#[test]
fn test_match_triggers_only_start_ellipsis() {
    let (_, service) = setup_service(vec!["dir_search".to_string()]);

    let line = "line is long and match is near end";
    let m = Match::new(31, 36);
    let result = service.extract_snippet(line, m, Some(10), Some(5));
    // Only starts with ellipsis
    assert!(result.starts_with("..."));
    assert!(!result.ends_with("..."));
}

#[test]
fn test_trim_applied() {
    let (_, service) = setup_service(vec!["dir_search".to_string()]);

    let line = "     match here with spaces    ";
    let m = Match::new(5, 10);

    let result = service.extract_snippet(line, m, Some(10), Some(5));

    // Ensure whitespace is trimmed before slicing
    assert!(!result.contains("     "));
    assert!(result.contains("match"));
}

#[test]
fn test_exact_snippet_end() {
    let (_, service) = setup_service(vec!["dir_search".to_string()]);
    let line = "some content with match inside";
    let m = Match::new(18, 23);
    let result = service.extract_snippet(line, m, Some(line.len()), Some(18));
    // Full trimmed line, no ellipses
    assert_eq!(result, "some content with match inside");
}

#[test]
fn search_files_content() {
    let (temp_dir, service) = setup_service(vec!["dir_search".to_string()]);

    create_temp_file(
        &temp_dir.as_path().join("dir_search"),
        "file1.txt",
        r#"For the Doctor Watsons of this world, as opposed to the Sherlock
        Holmeses, success in the province of detective work must always
        be, to a very large extent, the result of luck. Sherlock Holmes
        can extract a clew from a wisp of straw or a flake of cigar ash;
        but Doctor Watso2n has to have it taken out for him and dusted,
        and exhibited clearly, with Watso\d*n a label attached."#,
    );
    create_temp_file(
        &temp_dir.as_path().join("dir_search"),
        "file2.txt",
        r#"For the Doctor Watsons of this world, as opposed to the Sherlock
        Holmeses, success in the province of detective work must always
        be, to a very large extent, the result of luck. Sherlock Holmes
        can extract a clew from a wisp of straw or a flake of cigar ash;
        but Doctor Watso2n has to have it taken out for him and dusted,
        and exhibited clearly, with Watso\d*n a label attached."#,
    );

    let query = r#"Watso\d*n"#;

    let results = service
        .search_files_content(
            temp_dir.as_path().join("dir_search"),
            "*.txt",
            query,
            true,
            None,
        )
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].matches.len(), 2);
    assert_eq!(results[1].matches.len(), 2);
}
