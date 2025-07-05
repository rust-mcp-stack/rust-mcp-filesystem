use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "list_allowed_directories",
    title="List Allowed Directories",
    description = concat!("Returns a list of directories that the server has permission ",
    "to access Subdirectories within these allowed directories are also accessible. ",
    "Use this to identify which directories and their nested paths are available ",
    "before attempting to access files."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
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
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result,
        )]))
    }
}
