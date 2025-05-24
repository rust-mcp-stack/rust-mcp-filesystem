use std::path::Path;

use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use serde_json::json;

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "directory_tree",
    description = concat!("Get a recursive tree view of files and directories as a JSON structure. ",
    "Each entry includes 'name', 'type' (file/directory), and 'children' for directories. ",
    "Files have no children array, while directories always have a children array (which may be empty). ",
    "The output is formatted with 2-space indentation for readability. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct DirectoryTreeTool {
    /// The root path of the directory tree to generate.
    pub path: String,
}
impl DirectoryTreeTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let entries = context
            .list_directory(Path::new(&params.path))
            .await
            .map_err(CallToolError::new)?;

        let json_tree: Vec<serde_json::Value> = entries
            .iter()
            .map(|entry| {
                json!({
                    "name": entry.file_name().to_str().unwrap_or_default(),
                    "type": if entry.path().is_dir(){"directory"}else{"file"}
                })
            })
            .collect();
        let json_str =
            serde_json::to_string_pretty(&json!(json_tree)).map_err(CallToolError::new)?;
        Ok(CallToolResult::text_content(json_str, None))
    }
}
