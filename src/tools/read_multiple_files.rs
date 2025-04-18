use std::path::Path;

use futures::future::join_all;
use rust_mcp_macros::{mcp_tool, JsonSchema};
use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "read_multiple_files",
    description = "Read the contents of multiple files simultaneously. This is more 
efficient than reading files one by one when you need to analyze 
or compare multiple files. Each file's content is returned with its 
path as a reference. Failed reads for individual files won't stop 
the entire operation. Only works within allowed directories."
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadMultipleFilesTool {
    /// The list of file paths to read.
    pub paths: Vec<String>,
}

impl ReadMultipleFilesTool {
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
                        .read_file(Path::new(&path))
                        .await
                        .map_err(CallToolError::new);

                    content.map_or_else(
                        |err| format!("{}: Error - {}", path, err),
                        |value| format!("{}:\n{}\n", path, value),
                    )
                }
            })
            .collect();

        let contents = join_all(content_futures).await;

        Ok(CallToolResult::text_content(contents.join("\n---\n"), None))
    }
}
