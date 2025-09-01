use clap::Parser;
use rust_mcp_filesystem::{cli, server};

#[tokio::main]
async fn main() {
    let arguments = cli::CommandArguments::parse();
    if let Err(err) = arguments.validate() {
        eprintln!("Error: {err}");
        return;
    };

    if let Err(error) = server::start_server(arguments).await {
        eprintln!("{error}");
    }
    println!(">>> 90 {:?} ", 90);
}
