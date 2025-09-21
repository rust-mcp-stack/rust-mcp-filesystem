use crate::fs_service::{FileSystemService, utils::format_bytes};
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use std::path::Path;

#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub enum FileSizeOutputFormat {
    #[serde(rename = "human-readable")]
    HumanReadable,
    #[serde(rename = "bytes")]
    Bytes,
}

#[mcp_tool(
    name = "calculate_directory_size",
    title="Calculate Directory Size",
    description = concat!("Calculates the total size of a directory specified by `root_path`.",
    "It recursively searches for files and sums their sizes. ",
    "The result can be returned in either a `human-readable` format or as `bytes`, depending on the specified `output_format` argument.",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct CalculateDirectorySize {
    /// The root directory path to start the size calculation.
    pub root_path: String,
    /// Defines the output format, which can be either `human-readable` or `bytes`.
    #[json_schema(default = "human-readable")]
    pub output_format: Option<FileSizeOutputFormat>,
}

impl CalculateDirectorySize {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let total_bytes = context
            .calculate_directory_size(Path::new(&params.root_path))
            .await
            .map_err(CallToolError::new)?;

        let output_content = match params
            .output_format
            .unwrap_or(FileSizeOutputFormat::HumanReadable)
        {
            FileSizeOutputFormat::HumanReadable => format_bytes(total_bytes),
            FileSizeOutputFormat::Bytes => format!("{total_bytes}"),
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(
            output_content,
        )]))
    }
}
