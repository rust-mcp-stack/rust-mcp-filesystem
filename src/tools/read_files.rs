use std::path::Path;

use rust_mcp_macros::{mcp_tool, JsonSchema};
use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "read_file",
    description = "Read the complete contents of a file from the file system. Handles various text encodings and provides detailed error messages if the file cannot be read. Use this tool when you need to examine the contents of a single file. Only works within allowed directories."
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadFileTool {
    /// The path of the file to read.
    pub path: String,
}

impl ReadFileTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let content = context
            .read_file(Path::new(&params.path))
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(content, None))
    }
}
