use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "diff_files",
    title="Compare two files",
    description = concat!("Generate a unified diff between two files. ",
    "For text files, produces a standard unified diff format showing additions and deletions. ",
    "For binary files, compares SHA-256 hashes and reports whether files are identical or different. ",
    "Respects file size limits to prevent memory issues. ",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = true,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct DiffFiles {
    /// The path of the first file to compare.
    pub path1: String,
    /// The path of the second file to compare.
    pub path2: String,
    /// Optional: Maximum file size in bytes to process (default: 10485760 = 10MB).
    /// Files exceeding this limit will return an error.
    #[serde(
        rename = "maxFileSizeBytes",
        default,
        skip_serializing_if = "std::option::Option::is_none"
    )]
    pub max_file_size_bytes: Option<u64>,
}

impl DiffFiles {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = context
            .diff_files(
                Path::new(&params.path1),
                Path::new(&params.path2),
                params.max_file_size_bytes,
            )
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result,
        )]))
    }
}
