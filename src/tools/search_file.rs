use std::path::Path;

use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};

use crate::fs_service::FileSystemService;
#[mcp_tool(
    name = "search_files",
    description = concat!("Recursively search for files and directories matching a pattern. ",
  "Searches through all subdirectories from the starting path. The search ",
"is case-insensitive and matches partial names. Returns full paths to all ",
"matching items. Great for finding files when you don't know their exact location. ",
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
            )
            .map_err(CallToolError::new)?;

        let result = if !list.is_empty() {
            list.iter()
                .map(|entry| entry.path().display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "No matches found".to_string()
        };
        Ok(CallToolResult::text_content(result, None))
    }
}
