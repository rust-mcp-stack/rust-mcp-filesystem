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
        help = "Enables read/write mode for the app, allowing both reading and writing. Defaults to disabled."
    )]
    pub allow_write: bool,
    #[arg(
        help = "List of directories that are permitted for the operation. It is required when 'enable-roots' is not provided OR client does not support Roots.",
        long_help = concat!("Provide a space-separated list of directories that are permitted for the operation.\nThis list allows multiple directories to be provided.\n\nExample:  ", env!("CARGO_PKG_NAME"), " /path/to/dir1 /path/to/dir2 /path/to/dir3"),
        required = false
    )]
    pub allowed_directories: Vec<String>,

    #[arg(
        short = 't',
        long,
        help = "Enables dynamic directory access control via Roots from the MCP client side. Defaults to disabled.\nWhen enabled, MCP clients that support Roots can dynamically update the allowed directories.\nAny directories provided by the client will completely replace the initially configured allowed directories on the server."
    )]
    pub enable_roots: bool,
}

impl CommandArguments {
    pub fn validate(&self) -> Result<(), String> {
        if !self.enable_roots && self.allowed_directories.is_empty() {
            return Err(
                format!(" <ALLOWED_DIRECTORIES> is required when `--enable-roots` is not provided.\n Run `{} --help` to view the usage instructions.",env!("CARGO_PKG_NAME"))
            );
        }
        Ok(())
    }
}
