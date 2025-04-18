use clap::Parser;
use rust_mcp_filesystem::{cli, error::ServiceResult, server};

#[tokio::main]
async fn main() -> ServiceResult<()> {
    server::start_server(cli::CommandArguments::parse()).await
}
