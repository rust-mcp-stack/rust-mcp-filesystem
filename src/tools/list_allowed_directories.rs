use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "list_allowed_directories",
    title="List allowed directories",
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
pub struct ListAllowedDirectories {}

impl ListAllowedDirectories {
    pub async fn run_tool(
        _: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let allowed_directories = context.allowed_directories().await;

        let result = if allowed_directories.is_empty() {
            "Allowed directories list is empty!".to_string()
        } else {
            format!(
                "Allowed directories:\n{}",
                allowed_directories
                    .iter()
                    .map(|entry| entry.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result,
        )]))
    }
}
