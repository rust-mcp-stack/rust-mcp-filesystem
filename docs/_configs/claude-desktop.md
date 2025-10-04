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

## Running via Docker

**Note:** In the example below, all allowed directories are mounted to `/projects`,  and `/projects` is passed as the allowed directory argument to the server CLI. You can modify this as needed to fit your requirements.

`ALLOW_WRITE` and `ENABLE_ROOTS` environments could be used to enable write and MCP Roots support.

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
