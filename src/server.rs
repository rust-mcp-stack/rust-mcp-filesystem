use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ServerCapabilities, ServerCapabilitiesTools,
    LATEST_PROTOCOL_VERSION,
};
use rust_mcp_sdk::{mcp_server::server_runtime, McpServer, StdioTransport, TransportOptions};

use crate::handler::FileSystemHandler;
use crate::{cli::CommandArguments, error::ServiceResult};

pub fn server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "rust-mcp-filesystem".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title:Some("Filesystem MCP Server: fast and efficient tools for managing filesystem operations.".to_string())
        },
        capabilities: ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            completions: None,
        },
        instructions: None,
        meta: None,
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    }
}

pub async fn start_server(args: CommandArguments) -> ServiceResult<()> {
    let transport = StdioTransport::new(TransportOptions::default())?;

    let handler = FileSystemHandler::new(&args)?;
    let server = server_runtime::create_server(server_details(), transport, handler);

    server.start().await?;

    Ok(())
}
