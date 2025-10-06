use std::path::Path;

use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};

use crate::fs_service::FileSystemService;

// read_file_lines
#[mcp_tool(
    name = "read_file_lines",
    title="Read file lines",
    description = concat!("Reads lines from a text file starting at a specified line offset (0-based) and continues for the specified number of lines if a limit is provided.",
    "This function skips the first 'offset' lines and then reads up to 'limit' lines if specified, or reads until the end of the file otherwise.",
    "It's useful for partial reads, pagination, or previewing sections of large text files.",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadFileLines {
    /// The path of the file to get information for.
    pub path: String,
    /// Number of lines to skip from the start (0-based).
    pub offset: u64,
    ///  Optional maximum number of lines to read after the offset.
    pub limit: Option<u64>,
}

impl ReadFileLines {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = context
            .read_file_lines(
                Path::new(&params.path),
                params.offset as usize,
                params.limit.map(|v| v as usize),
            )
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result,
        )]))
    }
}
