use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};
use std::fmt::Write;
use std::path::Path;

use crate::fs_service::FileSystemService;
use crate::fs_service::utils::format_bytes;

#[mcp_tool(
    name = "list_directory_with_sizes",
    title="List directory with file sizes",
    description = concat!("Get a detailed listing of all files and directories in a specified path, including sizes. " ,
        "Results clearly distinguish between files and directories with [FILE] and [DIR] prefixes. " ,
        "This tool is useful for understanding directory structure and " ,
        "finding specific files within a directory. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true,
    icons = [
        (src = "https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/list_directory_with_sizes.png",
        mime_type = "image/png",
        sizes = ["128x128"])
    ],
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ListDirectoryWithSizes {
    /// The path of the directory to list.
    pub path: String,
}

impl ListDirectoryWithSizes {
    async fn format_directory_entries(
        &self,
        mut entries: Vec<tokio::fs::DirEntry>,
    ) -> std::result::Result<String, CallToolError> {
        let mut file_count = 0;
        let mut dir_count = 0;
        let mut total_size: u64 = 0;

        // Estimate initial capacity: assume ~50 bytes per entry + summary
        let mut output = String::with_capacity(entries.len() * 50 + 120);

        // Sort entries by file name
        entries.sort_by_key(|a| a.file_name());

        // build the output string
        for entry in &entries {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if entry.path().is_dir() {
                writeln!(output, "[DIR]  {file_name:<30}").map_err(CallToolError::new)?;
                dir_count += 1;
            } else if entry.path().is_file() {
                let metadata = entry.metadata().await.map_err(CallToolError::new)?;

                let file_size = metadata.len();
                writeln!(
                    output,
                    "[FILE] {:<30} {:>10}",
                    file_name,
                    format_bytes(file_size)
                )
                .map_err(CallToolError::new)?;
                file_count += 1;
                total_size += file_size;
            }
        }

        // Append summary
        writeln!(
            output,
            "\nTotal: {file_count} files, {dir_count} directories"
        )
        .map_err(CallToolError::new)?;
        writeln!(output, "Total size: {}", format_bytes(total_size)).map_err(CallToolError::new)?;

        Ok(output)
    }

    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let entries = context
            .list_directory(Path::new(&params.path))
            .await
            .map_err(CallToolError::new)?;

        let output = params
            .format_directory_entries(entries)
            .await
            .map_err(CallToolError::new)?;
        Ok(CallToolResult::text_content(vec![TextContent::from(
            output,
        )]))
    }
}
