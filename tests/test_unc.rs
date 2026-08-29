//! Windows-only integration tests for UNC (`\\server\share`) allowed
//! directories.
//!
//! These require an actual reachable SMB share. The test locates one via:
//!   1. `MCP_TEST_UNC_ROOT` environment variable (e.g.
//!      `\\MEDIASERVER\Movies`), or
//!   2. creating a temporary SMB share with `net share` (requires elevation,
//!      available on CI runners), or
//!   3. the localhost admin share `\\localhost\C$` (requires elevation).
//!
//! If no share is reachable the test prints a skip notice and passes without
//! asserting, so CI on machines without a share does not fail spuriously.

#[cfg(windows)]
mod unc {
    use rust_mcp_filesystem::fs_service::FileSystemService;
    use std::path::PathBuf;

    /// Locates an accessible UNC root and an optional share name to delete on
    /// cleanup. Returns `None` when no share is reachable (test is skipped).
    fn make_unc_root() -> Option<(PathBuf, Option<String>)> {
        if let Ok(root) = std::env::var("MCP_TEST_UNC_ROOT") {
            return Some((PathBuf::from(root), None));
        }

        // Try creating a temporary SMB share; this keeps the canonical UNC form
        // (`\\?\UNC\localhost\...`) so the UNC fallback path is exercised.
        let base = std::env::temp_dir().join(format!("mcp_unc_base_{}", std::process::id()));
        if std::fs::create_dir_all(&base).is_ok() {
            let share_name = format!("mcpunc{}", std::process::id());
            let share_arg = format!("{share_name}={}", base.display());
            let ok = std::process::Command::new("net")
                .args(["share", &share_arg, "/grant:everyone,FULL"])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            if ok {
                return Some((
                    PathBuf::from(format!(r"\\localhost\{share_name}")),
                    Some(share_name),
                ));
            }
        }

        // Fall back to the admin share (resolves to a drive path, so this only
        // exercises UNC-form input handling, not the UNC fallback branch).
        let admin = PathBuf::from(r"\\localhost\C$");
        if std::fs::metadata(&admin).is_ok() {
            return Some((admin, None));
        }

        None
    }

    fn cleanup_share(share_name: Option<String>) {
        if let Some(name) = share_name {
            let _ = std::process::Command::new("net")
                .args(["share", &name, "/delete"])
                .status();
        }
    }

    #[tokio::test]
    async fn test_unc_end_to_end() {
        let Some((root, share_name)) = make_unc_root() else {
            eprintln!("SKIP: no UNC share available (set MCP_TEST_UNC_ROOT to run)");
            return;
        };

        // A unique subdirectory under the share is the allowed root for this test.
        let allowed_root = root.join(format!("mcp_unc_test_{}", std::process::id()));
        if std::fs::remove_dir_all(&allowed_root).is_err() {
            let _ = std::fs::create_dir_all(&allowed_root);
        }
        std::fs::create_dir_all(&allowed_root).unwrap();

        let run = async {
            let root_str = allowed_root.to_str().ok_or("non-utf8 path")?.to_string();
            let service = FileSystemService::try_new(&[root_str])?;

            // Allowed dirs are de-verbatimized for display/matching.
            let allowed = service.allowed_directories().await;
            assert!(!allowed[0].to_string_lossy().starts_with(r"\\?\"));
            assert_eq!(allowed[0], allowed_root);

            // Fresh directory starts empty.
            let entries = service.list_directory(&allowed_root).await?;
            assert!(entries.is_empty());

            // write_file + read_text_file.
            let file = allowed_root.join("a.txt");
            service.write_file(&file, &"hello".to_string()).await?;
            assert_eq!(service.read_text_file(&file, false).await?, "hello");

            // create_directory + move_file.
            let sub = allowed_root.join("sub");
            service.create_directory(&sub).await?;
            let moved = sub.join("b.txt");
            service.move_file(&file, &moved).await?;
            assert!(std::fs::metadata(&moved).is_ok());
            assert!(std::fs::metadata(&file).is_err());

            // search_files walks the share.
            let found = service
                .search_files(&allowed_root, "*.txt".to_string(), vec![], None, None)
                .await?;
            assert_eq!(found.len(), 1);

            // Case-insensitive request (NTFS/SMB semantics).
            let case_variant = allowed_root.join("SUB").join("B.TXT");
            assert_eq!(service.read_text_file(&case_variant, false).await?, "hello");

            // Outside the allowed root is denied.
            let outside = allowed_root.parent().unwrap().join("outside.txt");
            assert!(service.resolve(&outside).await.is_err());

            Ok::<(), Box<dyn std::error::Error>>(())
        };

        let result = run.await;
        let _ = std::fs::remove_dir_all(&allowed_root);
        cleanup_share(share_name);
        result.unwrap();
    }
}
