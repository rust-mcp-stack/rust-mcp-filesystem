use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "read_text_file",
    title="Read a text file",
    description = concat!("Read the complete contents of a text file from the file system as text. ",
    "Handles various text encodings and provides detailed error messages if the ",
    "file cannot be read. Use this tool when you need to examine the contents of ",
    "a single file. Optionally include line numbers for precise code targeting. ",
        "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true,
    icons = [
        (src = "https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_text_file.png",
        mime_type = "image/png",
        sizes = ["128x128"])
    ],
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadTextFile {
    /// The path of the file to read.
    pub path: String,
    /// Optional: Include line numbers in output (default: false).
    /// When enabled, each line is prefixed with a right-aligned, 1-based line number
    /// Followed by a space, a vertical bar (`|`), and another space in the format: `   123 | <original line content>`
    #[serde(default)]
    pub with_line_numbers: Option<bool>,
}

impl ReadTextFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let content = context
            .read_text_file(
                Path::new(&params.path),
                params.with_line_numbers.unwrap_or(false),
            )
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            content,
        )]))
    }
}
