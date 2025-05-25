# Capabilities

<!-- mcp-discovery-render -->
## rust-mcp-filesystem 0.1.8
| 🟢 Tools (14) | <span style="opacity:0.6">🔴 Prompts</span> | <span style="opacity:0.6">🔴 Resources</span> | <span style="opacity:0.6">🔴 Logging</span> | <span style="opacity:0.6">🔴 Experimental</span> |
| --- | --- | --- | --- | --- |
## 🛠️ Tools (14)

<table style="text-align: left;">
<thead>
    <tr>
        <th style="width: auto;"></th>
        <th style="width: auto;">Tool Name</th>
        <th style="width: auto;">Description</th>
        <th style="width: auto;">Inputs</th>
    </tr>
</thead>
<tbody style="vertical-align: top;">
        <tr>
            <td>1.</td>
            <td>
                <code><b>create_directory</b></code>
            </td>
            <td>Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. If the directory already exists, this operation will succeed silently. Perfect for setting up directory structures for projects or ensuring required paths exist. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>2.</td>
            <td>
                <code><b>directory_tree</b></code>
            </td>
            <td>Get a recursive tree view of files and directories as a JSON structure. Each entry includes <code>name</code>, <code>type</code> (file/directory), and <code>children</code> for directories. Files have no children array, while directories always have a children array (which may be empty). The output is formatted with 2-space indentation for readability. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>3.</td>
            <td>
                <code><b>edit_file</b></code>
            </td>
            <td>Make line-based edits to a text file. Each edit replaces exact line sequences with new content. Returns a git-style diff showing the changes made. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>dryRun</code> : boolean<br /></li>
                    <li style="white-space: nowrap;"> <code>edits</code> : {newText : string, oldText : string} [ ]<br /></li>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>4.</td>
            <td>
                <code><b>get_file_info</b></code>
            </td>
            <td>Retrieve detailed metadata about a file or directory. Returns comprehensive information including size, creation time, last modified time, permissions, and type. This tool is perfect for understanding file characteristics without reading the actual content. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>5.</td>
            <td>
                <code><b>list_allowed_directories</b></code>
            </td>
            <td>Returns a list of directories that the server has permission to access Subdirectories within these allowed directories are also accessible. Use this to identify which directories and their nested paths are available before attempting to access files.</td>
            <td>
                <ul>
                </ul>
            </td>
        </tr>
        <tr>
            <td>6.</td>
            <td>
                <code><b>list_directory</b></code>
            </td>
            <td>Get a detailed listing of all files and directories in a specified path. Results clearly distinguish between files and directories with <code>FILE</code> and <code>DIR</code> prefixes. This tool is essential for understanding directory structure and finding specific files within a directory. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>7.</td>
            <td>
                <code><b>move_file</b></code>
            </td>
            <td>Move or rename files and directories. Can move files between directories and rename them in a single operation. If the destination exists, the operation will fail. Works across different directories and can be used for simple renaming within the same directory. Both source and destination must be within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>destination</code> : string<br /></li>
                    <li style="white-space: nowrap;"> <code>source</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>8.</td>
            <td>
                <code><b>read_file</b></code>
            </td>
            <td>Read the complete contents of a file from the file system. Handles various text encodings and provides detailed error messages if the file cannot be read. Use this tool when you need to examine the contents of a single file. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>9.</td>
            <td>
                <code><b>read_multiple_files</b></code>
            </td>
            <td>Read the contents of multiple files simultaneously. This is more efficient than reading files one by one when you need to analyze or compare multiple files. Each file's content is returned with its path as a reference. Failed reads for individual files won't stop the entire operation. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>paths</code> : string [ ]<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>10.</td>
            <td>
                <code><b>search_files</b></code>
            </td>
            <td>Recursively search for files and directories matching a pattern. Searches through all subdirectories from the starting path. The search is case-insensitive and matches partial names. Returns full paths to all matching items. Great for finding files when you don't know their exact location. Only searches within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>excludePatterns</code> : string [ ]<br /></li>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                    <li style="white-space: nowrap;"> <code>pattern</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>11.</td>
            <td>
                <code><b>unzip_file</b></code>
            </td>
            <td>Extracts the contents of a ZIP archive to a specified target directory.<br/>It takes a source ZIP file path and a target extraction directory.<br/>The tool decompresses all files and directories stored in the ZIP, recreating their structure in the target location.<br/>Both the source ZIP file and the target directory should reside within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>target_path</code> : string<br /></li>
                    <li style="white-space: nowrap;"> <code>zip_file</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>12.</td>
            <td>
                <code><b>write_file</b></code>
            </td>
            <td>Create a new file or completely overwrite an existing file with new content. Use with caution as it will overwrite existing files without warning. Handles text content with proper encoding. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>content</code> : string<br /></li>
                    <li style="white-space: nowrap;"> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>13.</td>
            <td>
                <code><b>zip_directory</b></code>
            </td>
            <td>Creates a ZIP archive by compressing a directory , including files and subdirectories matching a specified glob pattern.<br/>It takes a path to the folder and a glob pattern to identify files to compress and a target path for the resulting ZIP file.<br/>Both the source directory and the target ZIP file should reside within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>input_directory</code> : string<br /></li>
                    <li style="white-space: nowrap;"> <code>pattern</code> : string<br /></li>
                    <li style="white-space: nowrap;"> <code>target_zip_file</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>14.</td>
            <td>
                <code><b>zip_files</b></code>
            </td>
            <td>Creates a ZIP archive by compressing files. It takes a list of files to compress and a target path for the resulting ZIP file. Both the source files and the target ZIP file should reside within allowed directories.</td>
            <td>
                <ul>
                    <li style="white-space: nowrap;"> <code>input_files</code> : string [ ]<br /></li>
                    <li style="white-space: nowrap;"> <code>target_zip_file</code> : string<br /></li>
                </ul>
            </td>
        </tr>
</tbody>
</table>




<sub>◾ generated by [mcp-discovery](https://github.com/rust-mcp-stack/mcp-discovery)</sub>
<!-- mcp-discovery-render-end -->