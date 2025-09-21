use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "move_file",
    title="Move File",
    description = concat!("Move or rename files and directories. Can move files between directories ",
"and rename them in a single operation. If the destination exists, the ",
"operation will fail. Works across different directories and can be used ",
"for simple renaming within the same directory. ",
"Both source and destination must be within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct MoveFile {
    /// The source path of the file to move.
    pub source: String,
    /// The destination path to move the file to.
    pub destination: String,
}

impl MoveFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        context
            .move_file(Path::new(&params.source), Path::new(&params.destination))
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!(
                "Successfully moved {} to {}",
                &params.source, &params.destination
            ),
        )]))
    }
}
