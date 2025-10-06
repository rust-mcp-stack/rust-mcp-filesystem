use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::TextContent,
};
use std::path::Path;

use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;
#[mcp_tool(
    name = "write_file",
    title="Write file",
    description = concat!("Create a new file or completely overwrite an existing file with new content. ",
"Use with caution as it will overwrite existing files without warning. ",
"Handles text content with proper encoding. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(Debug, Clone, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct WriteFile {
    /// The path of the file to write to.
    pub path: String,
    /// The content to write to the file.
    pub content: String,
}

impl WriteFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        context
            .write_file(Path::new(&params.path), &params.content)
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            format!("Successfully wrote to {}", &params.path),
        )]))
    }
}
