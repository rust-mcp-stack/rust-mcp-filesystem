use std::path::Path;

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "create_directory",
    title="Create Directory",
    description = concat!("Create a new directory or ensure a directory exists. ",
    "Can create multiple nested directories in one operation. ",
    "If the directory already exists, this operation will succeed silently. ",
    "Perfect for setting up directory structures for projects or ensuring required paths exist. ",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct CreateDirectoryTool {
    /// The path where the directory will be created.
    pub path: String,
}

impl CreateDirectoryTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        context
            .create_directory(Path::new(&params.path))
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!("Successfully created directory {}", &params.path),
        )]))
    }
}
