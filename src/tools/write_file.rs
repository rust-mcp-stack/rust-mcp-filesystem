use std::path::Path;

use rust_mcp_macros::{mcp_tool, JsonSchema};
use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};

use crate::fs_service::FileSystemService;
#[mcp_tool(
    name = "write_file",
    description = "Create a new file or completely overwrite an existing file with new content.
Use with caution as it will overwrite existing files without warning.
Handles text content with proper encoding. Only works within allowed directories."
)]
#[derive(Debug, Clone, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct WriteFileTool {
    /// The path of the file to write to.
    pub path: String,
    /// The content to write to the file.
    pub content: String,
}

impl WriteFileTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        context
            .write_file(Path::new(&params.path), &params.content)
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(
            format!("Successfully wrote to {}", &params.path),
            None,
        ))
    }
}
