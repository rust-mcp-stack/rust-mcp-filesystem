use crate::cli::CommandArguments;
use crate::error::ServiceError;
use crate::invoke_tools;
use crate::{error::ServiceResult, fs_service::FileSystemService, tools::*};
use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, InitializeRequestParams, NotificationParams, PaginatedRequestParams,
};
use rust_mcp_sdk::schema::{
    CallToolResult, InitializeResult, ListToolsResult, RpcError, schema_utils::CallToolError,
};
use std::cmp::Ordering;
use std::sync::Arc;

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
        // return if roots_support is not enabled
        if !self.mcp_roots_support {
            return;
        }

        let allowed_directories = self.fs_service.allowed_directories().await;
        // if client does NOT support roots
        if !runtime.client_supports_root_list().unwrap_or(false) {
            // use allowed directories from command line
            if !allowed_directories.is_empty() {
                // display message only if mcp_roots_support is enabled, otherwise this message will be redundant
                if self.mcp_roots_support {
                    let _ = runtime.stderr_message("Client does not support MCP Roots. Allowed directories passed from command-line will be used.".to_string()).await;
                }
            } else {
                // root lists not supported AND allowed directories are empty
                let message = "Server cannot operate: No allowed directories available. Server was started without command-line directories and client does not support MCP roots protocol. Please either: 1) Start server with directory arguments, or 2) Use a client that supports MCP roots protocol and provides valid root directories.";
                let _ = runtime.stderr_message(message.to_string()).await;
                std::process::exit(1); // exit the server
            }
        } else {
            // client supports roots
            let fs_service = self.fs_service.clone();
            // retrieve roots from the client and update the allowed directories accordingly
            let roots = match runtime.clone().request_root_list(None).await {
                Ok(roots_result) => roots_result.roots,
                Err(_err) => {
                    vec![]
                }
            };

            let valid_roots = if roots.is_empty() {
                vec![]
            } else {
                let roots: Vec<_> = roots.iter().map(|v| v.uri.as_str()).collect();

                match fs_service.valid_roots(roots) {
                    Ok((roots, skipped)) => {
                        if let Some(message) = skipped {
                            let _ = runtime.stderr_message(message.to_string()).await;
                        }
                        roots
                    }
                    Err(_err) => vec![],
                }
            };

            if valid_roots.is_empty() {
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
        _params: Option<NotificationParams>,
        runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<(), RpcError> {
        if self.mcp_roots_support {
            self.update_allowed_directories(runtime).await;
        } else {
            let message =
                "Skipping ROOTS client updates, server launched without the --enable-roots flag."
                    .to_string();
            let _ = runtime.stderr_message(message).await;
        };
        Ok(())
    }

    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
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
        params: InitializeRequestParams,
        runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<InitializeResult, RpcError> {
        runtime
            .set_client_details(params.clone())
            .await
            .map_err(|err| RpcError::internal_error().with_message(format!("{err}")))?;

        let mut server_info = runtime.server_info().to_owned();
        // Provide compatibility for clients using older MCP protocol versions.
        if server_info.protocol_version.cmp(&params.protocol_version) == Ordering::Greater {
            server_info.protocol_version = params.protocol_version;
        }
        Ok(server_info)
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params: FileSystemTools =
            FileSystemTools::try_from(params).map_err(CallToolError::new)?;

        // Verify write access for tools that modify the file system
        if tool_params.require_write_access() {
            self.assert_write_access()?;
        }

        invoke_tools!(
            tool_params,
            &self.fs_service,
            ReadMediaFile,
            ReadMultipleMediaFiles,
            ReadTextFile,
            ReadMultipleTextFiles,
            WriteFile,
            EditFile,
            CreateDirectory,
            ListDirectory,
            DirectoryTree,
            MoveFile,
            SearchFiles,
            GetFileInfo,
            ListAllowedDirectories,
            ZipFiles,
            UnzipFile,
            ZipDirectory,
            SearchFilesContent,
            ListDirectoryWithSizes,
            HeadFile,
            TailFile,
            ReadFileLines,
            FindEmptyDirectories,
            CalculateDirectorySize,
            FindDuplicateFiles
        )
    }
}
