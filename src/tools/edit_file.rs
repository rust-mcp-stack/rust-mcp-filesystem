use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
#[serde(untagged)]
/// Represents a text replacement operation.
/// Supports two modes: exact matching (oldText/newText) or regex matching (pattern/replacement).
pub enum EditOperation {
    /// Exact text matching mode
    Exact {
        /// Text to search for - must match exactly.
        #[serde(rename = "oldText")]
        old_text: String,
        /// Text to replace the matched text with.
        #[serde(rename = "newText")]
        new_text: String,
    },
    /// Regular expression matching mode
    Regex {
        /// Regular expression pattern to find the text to replace.
        pattern: String,
        /// Text to replace the matched text with (can use capture groups $1, $2, etc.).
        replacement: String,
        /// Optional regex options
        #[serde(skip_serializing_if = "Option::is_none")]
        options: Option<RegexEditOptions>,
    },
}

#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
/// Options for regex-based edits
pub struct RegexEditOptions {
    /// If true, the regex is case-insensitive (default: false)
    #[serde(
        rename = "caseInsensitive",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub case_insensitive: Option<bool>,
    /// If true, ^ and $ match line boundaries instead of string boundaries (default: false)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiline: Option<bool>,
    /// If true, the dot (.) matches newlines as well (default: false)
    #[serde(
        rename = "dotAll",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub dot_all: Option<bool>,
    /// Maximum number of replacements (0 = unlimited, default: 0)
    #[serde(
        rename = "maxReplacements",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_replacements: Option<u32>,
}

#[mcp_tool(
    name = "edit_file",
    title="Edit file",
    description = concat!("Make line-based edits to a text file with support for exact matching or regular expressions. ",
    "Each edit can use either exact text matching (oldText/newText) or regex patterns (pattern/replacement). ",
    "Returns a git-style diff showing the changes made. ",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct EditFile {
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
    /// Optional line range to restrict edits (format: "start-end" or "start:end")
    #[serde(
        rename = "lineRange",
        default,
        skip_serializing_if = "std::option::Option::is_none"
    )]
    pub line_range: Option<String>,
}

impl EditFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let diff = context
            .apply_file_edits(
                Path::new(&params.path),
                params.edits,
                params.dry_run,
                None,
                params.line_range,
            )
            .await
            .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(diff)]))
    }
}
