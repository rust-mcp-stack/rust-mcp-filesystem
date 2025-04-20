use std::path::Path;

use rust_mcp_schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};

use crate::fs_service::FileSystemService;

#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
/// Represents a text replacement operation.
pub struct EditOperation {
    /// Text to search for - must match exactly.
    #[serde(rename = "oldText")]
    pub old_text: String,
    #[serde(rename = "newText")]
    /// Text to replace the matched text with.
    pub new_text: String,
}

#[mcp_tool(
    name = "edit_file",
    description = "Make line-based edits to a text file. Each edit replaces exact line sequences
with new content. Returns a git-style diff showing the changes made.
Only works within allowed directories."
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct EditFileTool {
    /// The path of the file to edit.
    pub path: String,

    /// The list of edit operations to apply.
    pub edits: Vec<EditOperation>,
    /// Preview changes using git-style diff format without applying them.
    #[serde(
        rename = "dryRun",
        default,
        skip_serializing_if = "std::option::Option::is_none"
    )]
    pub dry_run: Option<bool>,
}

impl EditFileTool {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let diff = context
            .apply_file_edits(Path::new(&params.path), params.edits, params.dry_run, None)
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(diff, None))
    }
}
