# Install

<!-- tabs:start -->

#### **Shell script**

<!-- x-release-please-start-version -->

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-installer.sh | sh
```

#### **PowerShell script**

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-installer.ps1 | iex"
```

<!-- x-release-please-end -->

#### **Homebrew**

```sh
brew install rust-mcp-stack/tap/rust-mcp-filesystem
```

#### **Download Binaries**

<table>
  <thead>
    <tr>
      <th>File</th>
      <th>Platform</th>
      <th>Checksum</th>
    </tr>
  </thead>
  <tbody>
    <tr>      
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-aarch64-apple-darwin.tar.gz">rust-mcp-filesystem-aarch64-apple-darwin.tar.gz</a>
      <!-- x-release-please-end -->
      </td>
      <td>Apple Silicon macOS</td>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-aarch64-apple-darwin.tar.gz.sha256">checksum</a>
      <!-- x-release-please-end -->    
      </td>
    </tr>
    <tr>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-apple-darwin.tar.gz">rust-mcp-filesystem-x86_64-apple-darwin.tar.gz</a>
      <!-- x-release-please-end -->
      </td>
      <td>Intel macOS</td>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-apple-darwin.tar.gz.sha256">checksum</a>
      <!-- x-release-please-end -->
      </td>
    </tr>
    <tr>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-pc-windows-msvc.zip">rust-mcp-filesystem-x86_64-pc-windows-msvc.zip</a>
      <!-- x-release-please-end -->
      </td>
      <td>x64 Windows (zip)</td>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-pc-windows-msvc.zip.sha256">checksum</a>
      <!-- x-release-please-end -->
      </td>
    </tr>
    <tr>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-pc-windows-msvc.msi">rust-mcp-filesystem-x86_64-pc-windows-msvc.msi</a>
      <!-- x-release-please-end -->
      </td>
      <td>x64 Windows (msi)</td>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-pc-windows-msvc.msi.sha256">checksum</a>
      <!-- x-release-please-end -->
      </td>
    </tr>
    <tr>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-aarch64-unknown-linux-gnu.tar.gz">rust-mcp-filesystem-aarch64-unknown-linux-gnu.tar.gz</a>
      <!-- x-release-please-end -->
      </td>
      <td>ARM64 Linux</td>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-aarch64-unknown-linux-gnu.tar.gz.sha256">checksum</a>
      <!-- x-release-please-end -->
      </td>
    </tr>
    <tr>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-unknown-linux-gnu.tar.gz">rust-mcp-filesystem-x86_64-unknown-linux-gnu.tar.gz</a>
      <!-- x-release-please-end -->
      </td>
      <td>x64 Linux</td>
      <td>
      <!-- x-release-please-start-version -->
      <a href="https://github.com/rust-mcp-stack/rust-mcp-filesystem/releases/download/v0.1.9/rust-mcp-filesystem-x86_64-unknown-linux-gnu.tar.gz.sha256">checksum</a>
      <!-- x-release-please-end -->
      </td>
    </tr>
  </tbody>
</table>

<!-- tabs:end -->

### 📝 Important Notice

By default, **rust-mcp-filesystem** operates in **`read-only`** mode unless write access is explicitly enabled. To allow write access, you must include the **`-w`** or **`--write-access`** flag in the list of arguments in configuration.
