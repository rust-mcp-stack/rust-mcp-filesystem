use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use std::fmt::Write;
use std::path::Path;

use crate::fs_service::{FileSystemService, utils::OutputFormat};

// find_empty_directories
#[mcp_tool(
    name = "find_empty_directories",
    title="Find empty directories",
    description = concat!("Recursively finds all empty directories within the given root path.",
    "A directory is considered empty if it contains no files in itself or any of its subdirectories.",
    "Operating system metadata files `.DS_Store` (macOS) and `Thumbs.db` (Windows) will be ignored.",
    "The optional exclude_patterns argument accepts glob-style patterns to exclude specific paths from the search.",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true,
    icons = [
        (src = "https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/find_empty_directories.png",
        mime_type = "image/png",
        sizes = ["128x128"])
    ],
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct FindEmptyDirectories {
    /// The path of the file to get information for.
    pub path: String,
    /// Optional list of glob patterns to exclude from the search. Directories matching these patterns will be ignored.
    pub exclude_patterns: Option<Vec<String>>,
    /// Specify the output format, accepts either `text` or `json` (default: text).
    pub output_format: Option<OutputFormat>,
}

impl FindEmptyDirectories {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = context
            .find_empty_directories(Path::new(&params.path), params.exclude_patterns)
            .await
            .map_err(CallToolError::new)?;

        let content =
            Self::format_output(result, params.output_format.unwrap_or(OutputFormat::Text))
                .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            content,
        )]))
    }

    fn format_output(
        empty_dirs: Vec<String>,
        output_format: OutputFormat,
    ) -> std::result::Result<String, CallToolError> {
        let output = match output_format {
            OutputFormat::Text => {
                let mut output = String::new();

                let header = if empty_dirs.is_empty() {
                    "No empty directories were found.".to_string()
                } else {
                    format!(
                        "Found {} empty {}:\n",
                        empty_dirs.len(),
                        (if empty_dirs.len() == 1 {
                            "directory"
                        } else {
                            "directories"
                        }),
                    )
                };
                output.push_str(&header);

                for dir in empty_dirs {
                    writeln!(output, "  {dir}").map_err(CallToolError::new)?;
                }
                output
            }
            OutputFormat::Json => {
                serde_json::to_string_pretty(&empty_dirs).map_err(CallToolError::new)?
            }
        };

        Ok(output)
    }
}
