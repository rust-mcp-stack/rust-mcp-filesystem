use std::path::Path;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{AudioContent, ImageContent};
use rust_mcp_sdk::schema::{CallToolResult, schema_utils::CallToolError};

use crate::error::ServiceError;
use crate::fs_service::FileSystemService;

#[mcp_tool(
    name = "read_media_file",
    title="Read a media (Image/Audio) file",
    description = concat!("Reads an image or audio file and returns its Base64-encoded content along with the corresponding MIME type. ",
        "The max_bytes argument could be used to enforce an upper limit on the size of a file to read ",
        "if the media file exceeds this limit, the operation will return an error instead of reading the media file. ",
    "Access is restricted to files within allowed directories only."),
    destructive_hint = false,
    idempotent_hint = false,
    open_world_hint = false,
    read_only_hint = true,
    icons = [
        (src = "https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_media_file.png",
        mime_type = "image/png",
        sizes = ["128x128"])
    ],
)]
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, JsonSchema)]
pub struct ReadMediaFile {
    /// The path of the file to read.
    pub path: String,
    /// Maximum allowed file size (in bytes) to be read.
    pub max_bytes: Option<u64>,
}

impl ReadMediaFile {
    pub async fn run_tool(
        params: Self,
        context: &FileSystemService,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let (kind, content) = context
            .read_media_file(
                Path::new(&params.path),
                params.max_bytes.map(|v| v as usize),
            )
            .await
            .map_err(CallToolError::new)?;
        let mime_type = kind.mime_type().to_string();
        let call_result = match kind.matcher_type() {
            infer::MatcherType::Image => {
                let image_content: ImageContent = ImageContent::new(content, mime_type, None, None);
                CallToolResult::image_content(vec![image_content])
            }
            infer::MatcherType::Audio => {
                let audio_content: AudioContent = AudioContent::new(content, mime_type, None, None);
                CallToolResult::audio_content(vec![audio_content])
            }
            _ => {
                return Err(CallToolError::from_message(
                    ServiceError::InvalidMediaFile(mime_type).to_string(),
                ));
            }
        };

        Ok(call_result)
    }
}
