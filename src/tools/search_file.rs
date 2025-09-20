use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;
#[mcp_tool(
    name = "search_files",
    title="Search Files",
    description = concat!("Recursively search for files and directories matching a pattern. ",
  "Searches through all subdirectories from the starting path. The search is case-insensitive ",
  "and matches partial names. Returns full paths to all matching items.",
  "Optional 'min_bytes' and 'max_bytes' arguments can be used to filter files by size, ",
  "ensuring that only files within the specified byte range are included in the search. ",
  "This tool is great for finding files when you don't know their exact location or find files by their size.",
  "Only searches within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]

/// A tool for searching files based on a path and pattern.
pub struct SearchFilesTool {
    /// The directory path to search in.
    pub path: String,
    /// The file glob pattern to match (e.g., "*.rs").
    pub pattern: String,
    #[serde(rename = "excludePatterns")]
    /// Optional list of patterns to exclude from the search.
    pub exclude_patterns: Option<Vec<String>>,
    /// Minimum file size (in bytes) to include in the search (optional).
    pub min_bytes: Option<u64>,
    /// Maximum file size (in bytes) to include in the search (optional).
    pub max_bytes: Option<u64>,
}
impl SearchFilesTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let list = context
            .search_files(
                Path::new(&params.path),
                params.pattern,
                params.exclude_patterns.unwrap_or_default(),
                params.min_bytes,
                params.max_bytes,
            )
            .await
            .map_err(CallToolError::new)?;

        let result = if !list.is_empty() {
            list.iter()
                .map(|entry| entry.path().display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "No matches found".to_string()
        };
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result,
        )]))
    }
}
