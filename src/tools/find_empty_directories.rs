use std::path::Path;

use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};

use crate::fs_service::{FileSystemService, OS_LINE_ENDING};

// head_file
#[mcp_tool(
    name = "find_empty_directories",
    title="Find Empty Directories",
    description = concat!("Recursively finds all empty directories within the given root path.",
    "A directory is considered empty if it contains no files or subdirectories.",
    "The optional exclude_patterns argument accepts glob-style patterns to exclude specific paths from the search.",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct FindEmptyDirectoriesTool {
    /// The path of the file to get information for.
    pub path: String,
    /// Optional list of glob patterns to exclude from the search. Directories matching these patterns will be ignored.
    pub exclude_patterns: Option<Vec<String>>,
}

impl FindEmptyDirectoriesTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = context
            .find_empty_directories(&Path::new(&params.path), params.exclude_patterns)
            .await
            .map_err(CallToolError::new)?;
        let content = result.join(OS_LINE_ENDING);
        Ok(CallToolResult::text_content(vec![TextContent::from(
            content,
        )]))
    }
}
