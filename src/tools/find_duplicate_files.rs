use crate::fs_service::{FileSystemService, utils::OutputFormat};
use rust_mcp_sdk::{
    macros::{JsonSchema, mcp_tool},
    schema::{CallToolResult, TextContent, schema_utils::CallToolError},
};
use std::path::Path;
use std::{collections::BTreeMap, fmt::Write};

#[mcp_tool(
    name = "find_duplicate_files",
    title="Calculate Directory Size",
    description = concat!("Find duplicate files within a directory and return list of duplicated files as text or json format",
    "Optional `pattern` argument can be used to narrow down the file search to specific glob pattern.",
    "Optional `exclude_patterns` can be used to exclude certain files matching a glob.",
    "`min_bytes` and `max_bytes` are optional arguments that can be used to restrict the search to files with sizes within a specified range.",
    "The output_format argument specifies the format of the output and accepts either `text` or `json` (default: text).",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct FindDuplicateFiles {
    /// The root directory path to start the search.
    pub root_path: String,
    /// Optional glob pattern can be used to match target files.
    pub pattern: Option<String>,
    /// Optional list of glob patterns to exclude from the search. File matching these patterns will be ignored.
    pub exclude_patterns: Option<Vec<String>>,
    /// Minimum file size (in bytes) to include in the search (default to 1).
    #[json_schema(default = "1")]
    pub min_bytes: Option<u64>,
    /// Maximum file size (in bytes) to include in the search (optional).
    pub max_bytes: Option<u64>,
    /// Specify the output format, accepts either `text` or `json` (default: text).
    #[json_schema(default = "text")]
    pub output_format: Option<OutputFormat>,
}

impl FindDuplicateFiles {
    fn format_output(
        duplicate_files: Vec<Vec<String>>,
        output_format: OutputFormat,
    ) -> std::result::Result<String, CallToolError> {
        match output_format {
            OutputFormat::Text => {
                let mut output = String::new();

                let header = if duplicate_files.is_empty() {
                    "No duplicate files were found.".to_string()
                } else {
                    format!("Found {} sets of duplicate files:\n", duplicate_files.len(),)
                };
                output.push_str(&header);

                for (i, group) in duplicate_files.iter().enumerate() {
                    writeln!(output, "\nDuplicated Group {}:", i + 1)
                        .map_err(CallToolError::new)?;
                    for file in group {
                        writeln!(output, "  {file}").map_err(CallToolError::new)?;
                    }
                }
                Ok(output)
            }
            OutputFormat::Json => {
                // Use a map to hold string keys and array values
                let mut map = BTreeMap::new();

                for (i, group) in duplicate_files.into_iter().enumerate() {
                    map.insert(i.to_string(), group);
                }

                // Serialize the map to a pretty JSON string
                Ok(serde_json::to_string_pretty(&map).map_err(CallToolError::new)?)
            }
        }
    }

    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let duplicate_files = context
            .find_duplicate_files(
                Path::new(&params.root_path),
                params.pattern.clone(),
                params.exclude_patterns.clone(),
                params.min_bytes.or(Some(1)),
                params.max_bytes,
            )
            .await
            .map_err(CallToolError::new)?;

        let result_content = Self::format_output(
            duplicate_files,
            params.output_format.unwrap_or(OutputFormat::Text),
        )
        .map_err(CallToolError::new)?;

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_content,
        )]))
    }
}
