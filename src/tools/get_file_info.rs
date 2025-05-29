use std::path::Path;

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "get_file_info",
    description = concat!("Retrieve detailed metadata about a file or directory. ",
    "Returns comprehensive information including size, creation time, ",
    "last modified time, permissions, and type. ",
    "This tool is perfect for understanding file characteristics without ",
    "reading the actual content. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct GetFileInfoTool {
    /// The path of the file to get information for.
    pub path: String,
}

impl GetFileInfoTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let stats = context
            .get_file_stats(Path::new(&params.path))
            .await
            .map_err(CallToolError::new)?;
        Ok(CallToolResult::text_content(stats.to_string(), None))
    }
}
