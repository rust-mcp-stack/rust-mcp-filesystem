use std::cmp::Ordering;
use std::sync::Arc;

use crate::cli::CommandArguments;
use crate::error::ServiceError;
use crate::{error::ServiceResult, fs_service::FileSystemService, tools::*};
use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::RootsListChangedNotification;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, InitializeRequest, InitializeResult, ListToolsRequest,
    ListToolsResult, RpcError, schema_utils::CallToolError,
};

pub struct FileSystemHandler {
    readonly: bool,
    mcp_roots_support: bool,
    fs_service: Arc<FileSystemService>,
}

impl FileSystemHandler {
    pub fn new(args: &CommandArguments) -> ServiceResult<Self> {
        let fs_service = FileSystemService::try_new(&args.allowed_directories)?;
        Ok(Self {
            fs_service: Arc::new(fs_service),
            readonly: !args.allow_write,
            mcp_roots_support: args.enable_roots,
        })
    }

    pub fn assert_write_access(&self) -> std::result::Result<(), CallToolError> {
        if self.readonly {
            Err(CallToolError::new(ServiceError::NoWriteAccess))
        } else {
            Ok(())
        }
    }

    pub async fn startup_message(&self) -> String {
        let common_message = format!(
            "Secure MCP Filesystem Server running in \"{}\" mode {} \"MCP Roots\" support.",
            if !self.readonly {
                "read/write"
            } else {
                "readonly"
            },
            if self.mcp_roots_support {
                "with"
            } else {
                "without"
            },
        );

        let allowed_directories = self.fs_service.allowed_directories().await;
        let sub_message: String = if allowed_directories.is_empty() && self.mcp_roots_support {
            "No allowed directories is set - waiting for client to provide roots via MCP protocol...".to_string()
        } else {
            format!(
                "Allowed directories:\n{}",
                allowed_directories
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<String>>()
                    .join(",\n")
            )
        };

        format!("{common_message}\n{sub_message}")
    }

    pub(crate) async fn update_allowed_directories(&self, runtime: Arc<dyn McpServer>) {
        // if client does not support roots
        let allowed_directories = self.fs_service.allowed_directories().await;
        if !runtime.client_supports_root_list().unwrap_or(false) {
            if !allowed_directories.is_empty() {
                let _ = runtime.stderr_message(format!("Client does not support MCP Roots, using allowed directories set from server args:\n{}", allowed_directories
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<String>>()
                    .join(",\n"))).await;
            } else {
                // let message = "Server cannot operate: No allowed directories available. Server was started without command-line directories and client either does not support MCP roots protocol or provided empty roots. Please either: 1) Start server with directory arguments, or 2) Use a client that supports MCP roots protocol and provides valid root directories.";
                let message = "Server cannot operate: No allowed directories available. Server was started without command-line directories and client does not support MCP roots protocol. Please either: 1) Start server with directory arguments, or 2) Use a client that supports MCP roots protocol and provides valid root directories.";
                let _ = runtime.stderr_message(message.to_string()).await;
            }
        } else {
            let fs_service = self.fs_service.clone();
            let mcp_roots_support = self.mcp_roots_support;
            // retrieve roots from the client and update the allowed directories accordingly
            tokio::spawn(async move {
                let roots = match runtime.clone().list_roots(None).await {
                    Ok(roots_result) => roots_result.roots,
                    Err(_err) => {
                        vec![]
                    }
                };

                let valid_roots = if roots.is_empty() {
                    vec![]
                } else {
                    let roots: Vec<_> = roots.iter().map(|v| v.uri.as_str()).collect();
                    let valid_roots = match fs_service.valid_roots(roots) {
                        Ok((roots, skipped)) => {
                            if let Some(message) = skipped {
                                let _ = runtime.stderr_message(message.to_string()).await;
                            }
                            roots
                        }
                        Err(_err) => vec![],
                    };
                    valid_roots
                };

                if valid_roots.is_empty() && !mcp_roots_support {
                    let message = if allowed_directories.is_empty() {
                        "Server cannot operate: No allowed directories available. Server was started without command-line directories and client provided empty roots. Please either: 1) Start server with directory arguments, or 2) Use a client that supports MCP roots protocol and provides valid root directories."
                    } else {
                        "Client provided empty roots. Allowed directories passed from command-line will be used."
                    };
                    let _ = runtime.stderr_message(message.to_string()).await;
                } else {
                    let num_valid_roots = valid_roots.len();

                    fs_service.update_allowed_paths(valid_roots).await;
                    let message = format!(
                        "Updated allowed directories from MCP roots: {num_valid_roots} valid directories",
                    );
                    let _ = runtime.stderr_message(message.to_string()).await;
                }
            });
        }
    }
}
#[async_trait]
impl ServerHandler for FileSystemHandler {
    async fn on_initialized(&self, runtime: Arc<dyn McpServer>) {
        let _ = runtime.stderr_message(self.startup_message().await).await;
        self.update_allowed_directories(runtime).await;
    }

    async fn handle_roots_list_changed_notification(
        &self,
        _notification: RootsListChangedNotification,
        runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<(), RpcError> {
        self.update_allowed_directories(runtime).await;
        Ok(())
    }

    async fn handle_list_tools_request(
        &self,
        _: ListToolsRequest,
        _: Arc<dyn McpServer>,
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
        runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<InitializeResult, RpcError> {
        runtime
            .set_client_details(initialize_request.params.clone())
            .await
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
        _: Arc<dyn McpServer>,
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
            FileSystemTools::ListDirectoryWithSizesTool(params) => {
                ListDirectoryWithSizesTool::run_tool(params, &self.fs_service).await
            }
        }
    }
}
