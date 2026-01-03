use std::path::Path;

use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};

use crate::fs_service::FileSystemService;

// tail_file
#[mcp_tool(
    name = "tail_file",
    title="Tail file",
    description = concat!("Reads and returns the last N lines of a text file.",
    "This is useful for quickly previewing file contents without loading the entire file into memory.",
    "If the file has fewer than N lines, the entire file will be returned.",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true,
    icons = [
        (src = "https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/tail_file.png",
        mime_type = "image/png",
        sizes = ["128x128"])
    ],
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct TailFile {
    /// The path of the file to get information for.
    pub path: String,
    /// The number of lines to read from the ending of the file.
    pub lines: u64,
}

impl TailFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = context
            .tail_file(Path::new(&params.path), params.lines as usize)
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result,
        )]))
    }
}
