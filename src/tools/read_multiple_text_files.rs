use crate::fs_service::FileSystemService;
use futures::future::join_all;
use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};
use std::path::Path;

#[mcp_tool(
    name = "read_multiple_text_files",
    title="Read Multiple Text Files",
    description = concat!("Read the contents of multiple text files simultaneously as text. ",
    "This is more efficient than reading files one by one when you need to analyze ",
    "or compare multiple files. Each file's content is returned with its ",
    "path as a reference. Failed reads for individual files won't stop ",
    "the entire operation. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadMultipleTextFiles {
    /// The list of file paths to read.
    pub paths: Vec<String>,
}

impl ReadMultipleTextFiles {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let content_futures: Vec<_> = params
            .paths
            .iter()
            .map(|path| async move {
                {
                    let content = context
                        .read_text_file(Path::new(&path))
                        .await
                        .map_err(CallToolError::new);

                    content.map_or_else(
                        |err| format!("{path}: Error - {err}"),
                        |value| format!("{path}:\n{value}\n"),
                    )
                }
            })
            .collect();

        let contents = join_all(content_futures).await;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            contents.join("\n---\n"),
        )]))
    }
}
