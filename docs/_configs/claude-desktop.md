Incorporate the following into your `claude_desktop_config.json`, based on your preference for using the installed binary directly or opting for Docker.

## Using the Installed Binary

> Upon installation, binaries are automatically added to the $PATH. However, if you manually downloaded and installed the binary, modify the command to reference the installation path.

**For macOS or Linux:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "rust-mcp-filesystem",
      "args": ["~/Documents", "/path/to/other/allowed/dir"]
    }
  }
}
```

**For Windows:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "rust-mcp-filesystem.exe",
      "args": [
        "C:\\Users\\Username\\Documents",
        "C:\\path\\to\\other\\allowed\\dir"
      ]
    }
  }
}
```

### Disabling Specific Tools

You can disable specific tools using the `-d` or `--disable-tools` flag:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "rust-mcp-filesystem",
      "args": [
        "-d", "write_file,edit_file,move_file",
        "~/Documents"
      ]
    }
  }
}
```

This example disables `write_file`, `edit_file` and `move_file`. See the [CLI Options](../guide/cli-command-options.md) documentation for more details.

## Running via Docker

**Note:** In the example below, all allowed directories are mounted to `/projects`,  and `/projects` is passed as the allowed directory argument to the server CLI. You can modify this as needed to fit your requirements.

`ALLOW_WRITE`, `ENABLE_ROOTS`, and `DISABLE_TOOLS` environments could be used to enable write, MCP Roots support, and disable specific tools.

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "ALLOW_WRITE=false",
        "-e",
        "ENABLE_ROOTS=false",
        "-e",
        "DISABLE_TOOLS=write_file,edit_file",
        "--mount",
        "type=bind,src=/Users/username/Documents,dst=/projects/Documents",
        "--mount",
        "type=bind,src=/other/allowed/dir,dst=/projects/other/allowed/dir",
        "mcp/rust-mcp-filesystem",
        "/projects"
      ]
    }
  }
}
```
