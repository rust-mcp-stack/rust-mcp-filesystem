use std::path::Path;

use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "move_file",
    description = "Move or rename files and directories. Can move files between directories
and rename them in a single operation. If the destination exists, the
operation will fail. Works across different directories and can be used
for simple renaming within the same directory. Both source and destination must be within allowed directories."
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct MoveFileTool {
    /// The source path of the file to move.
    pub source: String,
    /// The destination path to move the file to.
    pub destination: String,
}

impl MoveFileTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        context
            .move_file(Path::new(&params.source), Path::new(&params.destination))
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(
            format!(
                "Successfully moved {} to {}",
                &params.source, &params.destination
            ),
            None,
        ))
    }
}
