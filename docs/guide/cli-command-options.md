## CLI Command Options

```sh
Usage: rust-mcp-filesystem [OPTIONS] [ALLOWED_DIRECTORIES]...

Arguments:
  [ALLOWED_DIRECTORIES]...
          Provide a space-separated list of directories that are permitted for the operation.
          This list allows multiple directories to be provided.

          Example:  rust-mcp-filesystem /path/to/dir1 /path/to/dir2 /path/to/dir3

Options:
  -w, --allow-write
          Enables read/write mode for the app, allowing both reading and writing. Defaults to disabled.

  -t, --enable-roots
          Enables dynamic directory access control via Roots from the MCP client side. Defaults to disabled.
          When enabled, MCP clients that support Roots can dynamically update the allowed directories.
          Any directories provided by the client will completely replace the initially configured allowed directories on the server.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
