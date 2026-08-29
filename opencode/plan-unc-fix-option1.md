# Fix Windows UNC (`\\server\share`) support without regressing symlink-escape hardening

**Status:** Implemented (Option 1 — hybrid)
**Target:** `rust-mcp-filesystem` 0.4.4
**Regression introduced by:** `5128138` (PR #94, cap-std sandboxing) — also touched by `1356567` (PR #91) and `418f837` (PR #88).
**Advisory:** [GHSA-58gp-g6r5-8p6w](https://github.com/rust-mcp-stack/rust-mcp-filesystem/security/advisories/GHSA-58gp-g6r5-8p6w) must NOT be re-introduced.

## Problem

On Windows, configuring an allowed directory such as `\\MEDIASERVER\Movies` stops working after the cap-std migration:

- `Path::canonicalize()` returns the **verbatim** form `\\?\UNC\MEDIASERVER\Movies`.
- cap-std's path-based operations on Windows build their path via `GetFinalPathNameByHandleW` and naively strip `\\?\` (`cap-primitives-4.0.3/src/windows/fs/get_path.rs:11-24`). For a UNC share this yields a *relative* `UNC\server\share` path, so `read_dir`/`entries`/`create_dir_all`/`rename` fail.
- `Path::strip_prefix` is case-sensitive, but UNC server/share names are case-insensitive → lower-case client requests are denied.
- The verbatim prefix leaks into display/log output (`Allowed directories: \\?\UNC\...`), confusing users.

## Root cause summary (verified against cap-std 4.0.3 source)

cap-std splits into two families on Windows:

| Operation | Mechanism | Works on UNC? |
|---|---|---|
| `open`, `read`, `read_to_string`, `write`, `create`(file), `metadata`, `dir_metadata`, `exists`, `open_dir`, `try_clone` | `open_at` → `CreateFileAtW` **relative to the dir handle** | ✅ yes |
| `read_dir`/`entries`, `create_dir`/`create_dir_all`, `rename`, `remove_file`, `remove_dir` | `get_path` → strip `\\?\` → `fs::*` (path-based) | ❌ broken |

So cap-std's **handle-relative** ops keep their hard symlink guarantee on UNC, and only the **already-path-based** ops are broken. The fix therefore keeps cap-std for content ops and falls back to `std::fs` only for the path-based ops — with pre/post verification.

## Design (Option 1 — hybrid)

1. **De-verbatimize canonical paths consistently** (`strip_verbatim_prefix`): `\\?\UNC\s\sh` → `\\s\sh`, `\\?\C:\foo` → `C:\foo`. Used for matching, display, and error/log output. Verbatim canonical is still kept for the actual `std::fs` calls (avoids MAX_PATH 260 limit).
2. **Case-insensitive prefix matching on Windows** (`strip_prefix_platform`): component-wise, `OsStr::eq_ignore_ascii_case` for Prefix (server/drive) and Normal components; plain `Path::strip_prefix` on macOS/Linux (case-sensitive, protects `/mnt/c` mount matching).
3. **`AllowedDir` gains `is_unc` + `unc_root: Option<PathBuf>`** (verbatim canonical root). `Resolved` gains `unc_root`. `FsEntry` unchanged (its `dir.open(&rel)` is handle-relative and works on UNC).
4. **Fallback (UNC-only) operations** on `Resolved`, each preserving the advisory's defenses:
   - `read_dir_names(base)` — `std::fs::read_dir` on the canonical/verbatim path for UNC; cap-std `entries()`/`read_dir` for local.
   - `entry_metadata(rel)` — `std::fs::symlink_metadata` (no-follow) for UNC so symlinked dirs can never be traversed out; cap-std `open`/`open_dir` (follows, as today) for local.
   - `create_dir_all()` / `create_dir_all_rel(rel)` — `std::fs::create_dir_all` then post-verify canonical result stays within root (case-insensitive); best-effort remove + error if escaped.
   - `rename_to(&dest)` — cap-std `Dir::rename` for local↔local; `std::fs::rename` with parent pre-verify + destination post-verify when UNC is involved.
5. **`resolve()`** keeps: canonicalize-deepest-ancestor + rejoin suffix, de-verbatimize, `strip_prefix_platform` match, `ParentDir` rejection (GHSA-58gp-g6r5-8p6w defense).

## Security guarantees after fix

| Threat | Local | UNC |
|---|---|---|
| `..` traversal (write/create/move/edit/`save_to`) | ✅ | ✅ (same resolve path) |
| Static symlink → outside (read/write) | ✅ | ✅ (canonicalize → strip fails) |
| Dangling symlink write target | ✅ | ✅ (handle open refuses / symlink_metadata) |
| TOCTOU symlink swap on read/write/edit/open/metadata | ✅ cap-std handle | ✅ cap-std handle (works on UNC) |
| create_dir/move/enumerate path-based residual | ⚠️ same as today (cap-std is path-based here) | ⚠️ same class, + pre/post verify |
| Enumeration follows symlink out | ✅ cap-std | ✅ `symlink_metadata` no-follow |

No regression vs 0.4.4 for local drives; UNC gains hard protection on content ops and path-verified protection on mkdir/move/enumerate.

## Files changed

- `src/fs_service/utils.rs` — helpers + unit tests
- `src/fs_service/core.rs` — types, resolve, walk_dir, fallback methods
- `src/fs_service/io/write.rs` — create_directory / move_file
- `src/fs_service/archive/unzip.rs` — create_dir_all_rel
- `src/fs_service/search/tree.rs` — build_tree / list_directory / find_empty_directories
- `src/fs_service/search/files.rs`, `src/fs_service/archive/zip.rs` — walk_dir call sites
- `tests/common/common.rs` — `get_temp_dir` de-verbatimized on Windows
- `tests/test_fs_service.rs` — Windows regression guards
- `tests/test_unc.rs` — real-share integration (skips if no share available)

## Test coverage

- **Unit (all OSes):** `is_unc_path`, `strip_verbatim_prefix`, `strip_prefix_platform` (case-insensitive Windows / case-sensitive Unix / UNC & drive / outside → None), `trim_trailing_separator`.
- **Windows guards (no share):** allowed dirs / resolve display never contain `\\?\`.
- **Windows integration (`tests/test_unc.rs`):** real UNC root via `MCP_TEST_UNC_ROOT`, else a temp SMB share via `net share`, else `\\localhost\C$`; exercises try_new, list, read, write, search, create_dir, move, case-insensitive request, denied-outside. Skips gracefully when no share is reachable.

## Verification

- `cargo build` and `cargo test` on Windows/macOS/Linux.
- Manual: `rust-mcp-filesystem \\MEDIASERVER\Movies --allow-write` → startup shows `Allowed directories: \\MEDIASERVER\Movies`; list/read/write/search work.

## Residual risk (documented, acceptable)

UNC `create_dir_all`/`rename`/enumeration remain path-based (Windows has no openat-mkdir/rename) with a narrow TOCTOU window — identical to cap-std's own behavior on local Windows drives today. No read/write of file content can land outside the share.
