use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "list_allowed_directories",
    description = concat!("Returns a list of directories that the server has permission ",
    "to access Subdirectories within these allowed directories are also accessible. ",
    "Use this to identify which directories and their nested paths are available ",
    "before attempting to access files.")
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ListAllowedDirectoriesTool {}

impl ListAllowedDirectoriesTool {
    pub async fn run_tool(
        _: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = format!(
            "Allowed directories:\n{}",
            context
                .allowed_directories()
                .iter()
                .map(|entry| entry.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        );
        Ok(CallToolResult::text_content(result, None))
    }
}
