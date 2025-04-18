use clap::{arg, command, Parser};

#[derive(Parser, Debug)]
#[command(name =  env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A lightning-fast, asynchronous, and lightweight MCP server designed for efficient handling of various filesystem operations", 
long_about = None)]
pub struct CommandArguments {
    #[arg(
        short = 'w',
        long,
        help = "Enables read/write mode for the app, allowing both reading and writing."
    )]
    pub allow_write: bool,
    #[arg(
        help = "List of directories that are permitted for the operation.",
        long_help = concat!("Provide a space-separated list of directories that are permitted for the operation.\nThis list allows multiple directories to be provided.\n\nExample:  ", env!("CARGO_PKG_NAME"), " /path/to/dir1 /path/to/dir2 /path/to/dir3"),
        required = true
    )]
    pub allowed_directories: Vec<String>,
}
