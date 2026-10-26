#[path = "common/common.rs"]
pub mod common;

use common::create_temp_file;
use rust_mcp_filesystem::fs_service::FileSystemService;
use std::io::Read;
use tempfile::TempDir;

#[cfg(unix)]
const FILE_COUNT: usize = 1024;

// Re-execute only this test with a small descriptor budget. The parent test
// runner keeps its original limits, including when nextest runs tests in parallel.
#[cfg(unix)]
fn run_with_low_fd_limit(test_name: &str) -> bool {
    const CHILD_ENV: &str = "RUST_MCP_FILESYSTEM_LOW_FD_TEST";
    if std::env::var(CHILD_ENV).as_deref() == Ok(test_name) {
        return true;
    }

    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("ulimit -S -n 128 && exec \"$@\"")
        .arg("low-fd-test")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", test_name, "--nocapture"])
        .env(CHILD_ENV, test_name)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child test {test_name} failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    false
}

#[cfg(unix)]
fn large_directory() -> (TempDir, FileSystemService) {
    let dir = TempDir::new().unwrap();
    for index in 0..FILE_COUNT {
        create_temp_file(dir.path(), &format!("file-{index:04}.txt"), "needle\n");
    }
    let service = FileSystemService::try_new(&[dir.path().to_str().unwrap().to_string()]).unwrap();
    (dir, service)
}

#[cfg(unix)]
#[tokio::test]
async fn test_search_files_with_low_fd_limit() {
    if !run_with_low_fd_limit("test_search_files_with_low_fd_limit") {
        return;
    }
    let (dir, service) = large_directory();
    let nested = dir.path().join("nested");
    create_temp_file(&nested, "nested.txt", "nested");

    for _ in 0..3 {
        let entries = service
            .search_files(dir.path(), "*.txt".to_string(), vec![], None, None)
            .await
            .unwrap();
        assert_eq!(entries.len(), FILE_COUNT + 1);
        assert!(entries.iter().all(|entry| entry.is_file()));
        assert!(
            entries
                .iter()
                .any(|entry| entry.path() == nested.join("nested.txt").canonicalize().unwrap())
        );
        for entry in entries {
            let mut content = String::new();
            entry.open().unwrap().read_to_string(&mut content).unwrap();
            assert!(content == "needle\n" || content == "nested");
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn test_search_files_content_with_low_fd_limit() {
    if !run_with_low_fd_limit("test_search_files_content_with_low_fd_limit") {
        return;
    }
    let (dir, service) = large_directory();
    create_temp_file(&dir.path().join("nested"), "nested.txt", "needle\n");

    for _ in 0..3 {
        let matches = service
            .search_files_content(dir.path(), "*.txt", "needle", false, None, None, None)
            .await
            .unwrap();
        assert_eq!(matches.len(), FILE_COUNT + 1);
        assert!(matches.iter().all(|result| {
            result.matches.len() == 1
                && result.matches[0].line_number == 1
                && result.matches[0].line_text == "needle"
        }));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn test_list_directory_with_low_fd_limit() {
    if !run_with_low_fd_limit("test_list_directory_with_low_fd_limit") {
        return;
    }
    let (dir, service) = large_directory();

    for _ in 0..3 {
        let entries = service.list_directory(dir.path()).await.unwrap();
        assert_eq!(entries.len(), FILE_COUNT);
        for entry in entries {
            assert!(entry.is_file());
            assert_eq!(entry.len(), 7);
            let mut content = String::new();
            entry.open().unwrap().read_to_string(&mut content).unwrap();
            assert_eq!(content, "needle\n");
        }
    }
}

#[tokio::test]
async fn test_search_entries_survive_root_update() {
    let dir = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let file = create_temp_file(&dir.path().join("nested"), "found.txt", "retained");
    let service = FileSystemService::try_new(&[dir.path().to_str().unwrap().to_string()]).unwrap();

    let entries = service
        .search_files(dir.path(), "*.txt".to_string(), vec![], None, None)
        .await
        .unwrap();
    service
        .update_allowed_paths(vec![other.path().to_path_buf()])
        .await;
    assert!(service.resolve(&file).await.is_err());
    drop(service);

    assert_eq!(entries.len(), 1);
    let mut content = String::new();
    entries[0]
        .open()
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "retained");
}

#[tokio::test]
async fn test_list_entries_survive_root_update() {
    let dir = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let file = create_temp_file(dir.path(), "found.txt", "retained");
    let service = FileSystemService::try_new(&[dir.path().to_str().unwrap().to_string()]).unwrap();

    let entries = service.list_directory(dir.path()).await.unwrap();
    service
        .update_allowed_paths(vec![other.path().to_path_buf()])
        .await;
    assert!(service.resolve(&file).await.is_err());
    drop(service);

    assert_eq!(entries.len(), 1);
    let mut content = String::new();
    entries[0]
        .open()
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "retained");
}
