use crate::fs_service::FileSystemService;
use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{AudioContent, ContentBlock, ImageContent};
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

#[mcp_tool(
    name = "read_multiple_media_files",
    title="Read multiple media (Image/Audio) files",
    description = concat!("Reads multiple image or audio files and returns their Base64-encoded contents along with corresponding MIME types. ",
    "This method is more efficient than reading files individually. ",
    "The max_bytes argument could be used to enforce an upper limit on the size of a file to read ",
    "Failed reads for specific files are skipped without interrupting the entire operation. ",
    "Only works within allowed directories."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadMultipleMediaFiles {
    /// The list of media file paths to read.
    pub paths: Vec<String>,
    /// Maximum allowed file size (in bytes) to be read.
    pub max_bytes: Option<u64>,
}

impl ReadMultipleMediaFiles {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let result = context
            .read_media_files(params.paths, params.max_bytes.map(|v| v as usize))
            .await
            .map_err(CallToolError::new)?;

        let content: Vec<_> = result
            .into_iter()
            .filter_map(|(kind, content)| {
                let mime_type = kind.mime_type().to_string();

                match kind.matcher_type() {
                    infer::MatcherType::Image => Some(ContentBlock::ImageContent(
                        ImageContent::new(content, mime_type, None, None),
                    )),
                    infer::MatcherType::Audio => Some(ContentBlock::AudioContent(
                        AudioContent::new(content, mime_type, None, None),
                    )),
                    _ => None,
                }
            })
            .collect();

        Ok(CallToolResult {
            content,
            is_error: None,
            meta: None,
            structured_content: None,
        })
    }
}
