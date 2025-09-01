use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};
use serde_json::{json, Map, Value};

use crate::error::ServiceError;
use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "directory_tree",
    title= "Directory Tree",
    description = concat!("Get a recursive tree view of files and directories as a JSON structure. ",
    "Each entry includes 'name', 'type' (file/directory), and 'children' for directories. ",
    "Files have no children array, while directories always have a children array (which may be empty). ",
    "If the 'max_depth' parameter is provided, the traversal will be limited to the specified depth. ",
    "As a result, the returned directory structure may be incomplete or provide a skewed representation of the full directory tree, since deeper-level files and subdirectories beyond the specified depth will be excluded. ",
    "The output is formatted with 2-space indentation for readability. Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct DirectoryTreeTool {
    /// The root path of the directory tree to generate.
    pub path: String,
    /// Limits the depth of directory traversal
    pub max_depth: Option<u64>,
}
impl DirectoryTreeTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let mut entry_counter: usize = 0;

        let allowed_directories = context.allowed_directories().await;

        let (entries, reached_max_depth) = context
            .directory_tree(
                params.path,
                params.max_depth.map(|v| v as usize),
                None,
                &mut entry_counter,
                allowed_directories,
            )
            .map_err(CallToolError::new)?;

        if entry_counter == 0 {
            return Err(CallToolError::new(ServiceError::FromString(
                "Could not find any entries".to_string(),
            )));
        }

        let json_str = serde_json::to_string_pretty(&json!(entries)).map_err(CallToolError::new)?;

        // Include meta flag to denote that max depth was hit; some files and directories might be omitted
        let meta = if reached_max_depth {
            let mut meta = Map::new();
            meta.insert(
                "warning".to_string(),
                Value::String(
                    "Incomplete listing: subdirectories beyond the maximum depth were skipped."
                        .to_string(),
                ),
            );
            Some(meta)
        } else {
            None
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(json_str)]).with_meta(meta))
    }
}
