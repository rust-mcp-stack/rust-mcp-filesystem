## CLI Command Options

```sh
Usage: rust-mcp-filesystem [OPTIONS] [ALLOWED_DIRECTORIES]...

Arguments:
  [ALLOWED_DIRECTORIES]...
          Provide a space-separated list of directories that are permitted for the operation.
          This list allows multiple directories to be provided.

          Example:  rust-mcp-filesystem /path/to/dir1 /path/to/dir2 /path/to/dir3

Options:
  -d, --disable-tools <DISABLE_TOOLS>
          Comma-separated list of tools to disable. By default, all tools are enabled.
          Visit https://rust-mcp-stack.github.io/rust-mcp-filesystem/#/capabilities to view the full list of available tools.

          [env: DISABLE_TOOLS=]

  -w, --allow-write [<ALLOW_WRITE>]
          Enables write mode for the app, allowing both reading and writing. Defaults to disabled.

          [env: ALLOW_WRITE=]
          [default: false]
          [possible values: true, false]

  -t, --enable-roots [<ENABLE_ROOTS>]
          Enables dynamic directory access control via Roots from the MCP client side. Defaults to disabled.
          When enabled, MCP clients that support Roots can dynamically update the allowed directories.
          Any directories provided by the client will completely replace the initially configured allowed directories on the server.

          [env: ENABLE_ROOTS=true]
          [default: false]
          [possible values: true, false]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Disabling Tools

You can disable specific tools to limit the functionality of the server. This is useful for:

- Security hardening by disabling write operations
- Reducing the number of available tools to save tokens
- Customizing available functionality for specific use cases

### Examples

```sh
# Disable write-related tools
rust-mcp-filesystem -d write_file,edit_file,move_file /path/to/dir

# Disable multiple tools
rust-mcp-filesystem -d read_text_file,search_files /path/to/dir

# Using environment variable
DISABLE_TOOLS=write_file,edit_file rust-mcp-filesystem /path/to/dir

# Disable all write tools at once
rust-mcp-filesystem -d write_file,edit_file,move_file,create_directory,delete_file,zip_files,unzip_file /path/to/dir
```

### Available Tool Names

For a complete list of available tools and their names, see the [Capabilities](https://rust-mcp-stack.github.io/rust-mcp-filesystem/#/capabilities) page. Tool names are case-insensitive (e.g., `read_text_file` and `Read_Text_File` are equivalent).
