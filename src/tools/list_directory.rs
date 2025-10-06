use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "list_directory",
    title="List directory",
    description = concat!("Get a detailed listing of all files and directories in a specified path. ",
"Results clearly distinguish between files and directories with [FILE] and [DIR] ",
"prefixes. This tool is essential for understanding directory structure and ",
"finding specific files within a directory. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ListDirectory {
    /// The path of the directory to list.
    pub path: String,
}

impl ListDirectory {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let entries = context
            .list_directory(Path::new(&params.path))
            .await
            .map_err(CallToolError::new)?;

        let formatted: Vec<_> = entries
            .iter()
            .map(|entry| {
                format!(
                    "{} {}",
                    if entry.path().is_dir() {
                        "[DIR]"
                    } else {
                        "[FILE]"
                    },
                    entry.file_name().to_str().unwrap_or_default()
                )
            })
            .collect();

        Ok(CallToolResult::text_content(vec![TextContent::from(
            formatted.join("\n"),
        )]))
    }
}
