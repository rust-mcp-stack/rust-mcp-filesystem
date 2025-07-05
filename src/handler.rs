use std::cmp::Ordering;

use crate::cli::CommandArguments;
use crate::error::ServiceError;
use crate::{error::ServiceResult, fs_service::FileSystemService, tools::*};
use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequest, CallToolResult, InitializeRequest,
    InitializeResult, ListToolsRequest, ListToolsResult, RpcError,
};
use rust_mcp_sdk::McpServer;

pub struct MyServerHandler {
    readonly: bool,
    fs_service: FileSystemService,
}

impl MyServerHandler {
    pub fn new(args: &CommandArguments) -> ServiceResult<Self> {
        let fs_service = FileSystemService::try_new(&args.allowed_directories)?;
        Ok(Self {
            fs_service,
            readonly: !&args.allow_write,
        })
    }

    pub fn assert_write_access(&self) -> std::result::Result<(), CallToolError> {
        if self.readonly {
            Err(CallToolError::new(ServiceError::NoWriteAccess))
        } else {
            Ok(())
        }
    }

    pub fn startup_message(&self) -> String {
        format!(
            "Secure MCP Filesystem Server running in \"{}\" mode.\nAllowed directories:\n{}",
            if !self.readonly {
                "read/write"
            } else {
                "readonly"
            },
            self.fs_service
                .allowed_directories()
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<String>>()
                .join(",\n")
        )
    }
}
#[async_trait]
impl ServerHandler for MyServerHandler {
    async fn on_server_started(&self, runtime: &dyn McpServer) {
        let _ = runtime.stderr_message(self.startup_message()).await;
    }

    async fn on_initialized(&self, _: &dyn McpServer) {}

    async fn handle_list_tools_request(
        &self,
        _: ListToolsRequest,
        _: &dyn McpServer,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: FileSystemTools::tools(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_initialize_request(
        &self,
        initialize_request: InitializeRequest,
        runtime: &dyn McpServer,
    ) -> std::result::Result<InitializeResult, RpcError> {
        runtime
            .set_client_details(initialize_request.params.clone())
            .map_err(|err| RpcError::internal_error().with_message(format!("{err}")))?;

        let mut server_info = runtime.server_info().to_owned();
        // Provide compatibility for clients using older MCP protocol versions.
        if server_info
            .protocol_version
            .cmp(&initialize_request.params.protocol_version)
            == Ordering::Greater
        {
            server_info.protocol_version = initialize_request.params.protocol_version;
        }
        Ok(server_info)
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _: &dyn McpServer,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params: FileSystemTools =
            FileSystemTools::try_from(request.params).map_err(CallToolError::new)?;

        // Verify write access for tools that modify the file system
        if tool_params.require_write_access() {
            self.assert_write_access()?;
        }

        match tool_params {
            FileSystemTools::ReadFileTool(params) => {
                ReadFileTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::ReadMultipleFilesTool(params) => {
                ReadMultipleFilesTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::WriteFileTool(params) => {
                WriteFileTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::EditFileTool(params) => {
                EditFileTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::CreateDirectoryTool(params) => {
                CreateDirectoryTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::ListDirectoryTool(params) => {
                ListDirectoryTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::DirectoryTreeTool(params) => {
                DirectoryTreeTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::MoveFileTool(params) => {
                MoveFileTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::SearchFilesTool(params) => {
                SearchFilesTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::GetFileInfoTool(params) => {
                GetFileInfoTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::ListAllowedDirectoriesTool(params) => {
                ListAllowedDirectoriesTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::ZipFilesTool(params) => {
                ZipFilesTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::UnzipFileTool(params) => {
                UnzipFileTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::ZipDirectoryTool(params) => {
                ZipDirectoryTool::run_tool(params, &self.fs_service).await
            }
            FileSystemTools::SearchFilesContentTool(params) => {
                SearchFilesContentTool::run_tool(params, &self.fs_service).await
            }
        }
    }
}
