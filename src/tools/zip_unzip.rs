use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "zip_files",
    title="Zip files",
    description = concat!("Creates a ZIP archive by compressing files. ",
"It takes a list of files to compress and a target path for the resulting ZIP file. ",
"Both the source files and the target ZIP file should reside within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = false
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ZipFiles {
    /// The list of files to include in the ZIP archive.
    pub input_files: Vec<String>,
    /// Path to save the resulting ZIP file, including filename and .zip extension
    pub target_zip_file: String,
}

impl ZipFiles {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result_content = context
            .zip_files(params.input_files, params.target_zip_file)
            .await
            .map_err(CallToolError::new)?;
        //TODO: return resource?
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_content,
        )]))
    }
}

#[mcp_tool(
    name = "unzip_file",
    title = "Unzip Files",
    description = "Extracts the contents of a ZIP archive to a specified target directory.
It takes a source ZIP file path and a target extraction directory.
The tool decompresses all files and directories stored in the ZIP, recreating their structure in the target location.
Both the source ZIP file and the target directory should reside within allowed directories."
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct UnzipFile {
    /// A filesystem path to an existing ZIP file to be extracted.
    pub zip_file: String,
    /// Path to the target directory where the contents of the ZIP file will be extracted.
    pub target_path: String,
}

impl UnzipFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result_content = context
            .unzip_file(&params.zip_file, &params.target_path)
            .await
            .map_err(CallToolError::new)?;
        //TODO: return resource?
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_content,
        )]))
    }
}

#[mcp_tool(
    name = "zip_directory",
    title = "Zip Directory",
    description = "Creates a ZIP archive by compressing a directory , including files and subdirectories matching a specified glob pattern.
It takes a path to the folder and a glob pattern to identify files to compress and a target path for the resulting ZIP file.
Both the source directory and the target ZIP file should reside within allowed directories."
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ZipDirectory {
    /// Path to the directory to zip
    pub input_directory: String,
    /// A optional glob pattern to match files and subdirectories to zip, defaults to **/*"
    pub pattern: Option<String>,
    /// Path to save the resulting ZIP file, including filename and .zip extension
    pub target_zip_file: String,
}

impl ZipDirectory {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let pattern = params.pattern.unwrap_or("**/*".to_string());
        let result_content = context
            .zip_directory(params.input_directory, pattern, params.target_zip_file)
            .await
            .map_err(CallToolError::new)?;
        //TODO: return resource?
        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_content,
        )]))
    }
}
