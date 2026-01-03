use crate::handler::FileSystemHandler;
use crate::{cli::CommandArguments, error::ServiceResult};
use rust_mcp_sdk::mcp_server::McpServerOptions;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::{McpServer, StdioTransport, TransportOptions, mcp_server::server_runtime};
use rust_mcp_sdk::{ToMcpServerHandler, mcp_icon};

pub fn server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "rust-mcp-filesystem".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("Filesystem MCP Server".to_string()),
            description: Some(
                "A fast and efficient tools for managing filesystem operations.".to_string(),
            ),
            icons: vec![mcp_icon!(
                src = "https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/rust-mcp-filesystem-128.png",
                mime_type = "image/png",
                sizes = ["128x128"]
            )],
            website_url: Some("https://rust-mcp-stack.github.io/rust-mcp-filesystem".into()),
        },
        capabilities: ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            completions: None,
            tasks: None,
        },
        instructions: None,
        meta: None,
        protocol_version: ProtocolVersion::V2025_11_25.to_string(),
    }
}

pub async fn start_server(args: CommandArguments) -> ServiceResult<()> {
    let transport = StdioTransport::new(TransportOptions::default())?;

    let handler = FileSystemHandler::new(&args)?;
    let server = server_runtime::create_server(McpServerOptions {
        server_details: server_details(),
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
        transport,
    });

    server.start().await?;

    Ok(())
}
