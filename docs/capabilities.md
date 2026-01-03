# Capabilities

<!-- mcp-discovery-render -->
## rust-mcp-filesystem 0.3.9

A fast and efficient tools for managing filesystem operations.

Website: https://rust-mcp-stack.github.io/rust-mcp-filesystem

| ✔ Tools (24) | ~~<span style="opacity:0.6" class="error">✘ Prompts</span>~~ | ~~<span style="opacity:0.6" class="error">✘ Resources</span>~~ | ~~<span style="opacity:0.6" class="error">✘ Logging</span>~~ | ~~<span style="opacity:0.6" class="error">✘ Completions</span>~~ | ~~<span style="opacity:0.6" class="error">✘ Tasks</span>~~ |
| --- | --- | --- | --- | --- | --- |

## 🛠️ Tools (24)

<table style="text-align: left;">
<thead>
    <tr>
        <th style="width: auto;"></th>
        <th style="width: auto;">Icon</th>
        <th style="width: auto;">Tool Name</th>
        <th style="width: auto;">Description</th>
        <th style="width: auto;">Inputs</th>
    </tr>
</thead>
<tbody style="vertical-align: top;">
        <tr>
            <td>1.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/calculate_directory_size.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>calculate_directory_size</b></code>
            </td>
            <td>Calculates the total size of a directory specified by <code>root_path</code>.It recursively searches for files and sums their sizes. The result can be returned in either a <code>human-readable</code> format or as <code>bytes</code>, depending on the specified <code>output_format</code> argument.Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>output_format</code> : human-readable | bytes<br /></li>
                    <li> <code>root_path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>2.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/create_directory.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>create_directory</b></code>
            </td>
            <td>Create a new directory or ensure a directory exists. Can create multiple nested directories in one operation. If the directory already exists, this operation will succeed silently. Perfect for setting up directory structures for projects or ensuring required paths exist. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>3.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/directory_tree.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>directory_tree</b></code>
            </td>
            <td>Get a recursive tree view of files and directories as a JSON structure. Each entry includes <code>name</code>, <code>type</code> (file/directory), and <code>children</code> for directories. Files have no children array, while directories always have a children array (which may be empty). If the <code>max_depth</code> parameter is provided, the traversal will be limited to the specified depth. As a result, the returned directory structure may be incomplete or provide a skewed representation of the full directory tree, since deeper-level files and subdirectories beyond the specified depth will be excluded. The output is formatted with 2-space indentation for readability. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>max_depth</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>4.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/edit_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>edit_file</b></code>
            </td>
            <td>Make line-based edits to a text file. Each edit replaces exact line sequences with new content. Returns a git-style diff showing the changes made. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>dryRun</code> : boolean<br /></li>
                    <li> <code>edits</code> : {newText : string, oldText : string} [ ]<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>5.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/find_duplicate_files.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>find_duplicate_files</b></code>
            </td>
            <td>Find duplicate files within a directory and return list of duplicated files as text or json formatOptional <code>pattern</code> argument can be used to narrow down the file search to specific glob pattern.Optional <code>exclude_patterns</code> can be used to exclude certain files matching a glob.<code>min_bytes</code> and <code>max_bytes</code> are optional arguments that can be used to restrict the search to files with sizes within a specified range.The output_format argument specifies the format of the output and accepts either <code>text</code> or <code>json</code> (default: text).Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>exclude_patterns</code> : string [ ]<br /></li>
                    <li> <code>max_bytes</code> : integer<br /></li>
                    <li> <code>min_bytes</code> : integer<br /></li>
                    <li> <code>output_format</code> : text | json<br /></li>
                    <li> <code>pattern</code> : string<br /></li>
                    <li> <code>root_path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>6.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/find_empty_directories.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>find_empty_directories</b></code>
            </td>
            <td>Recursively finds all empty directories within the given root path.A directory is considered empty if it contains no files in itself or any of its subdirectories.Operating system metadata files `.DS_Store` (macOS) and `Thumbs.db` (Windows) will be ignored.The optional exclude_patterns argument accepts glob-style patterns to exclude specific paths from the search.Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>exclude_patterns</code> : string [ ]<br /></li>
                    <li> <code>output_format</code> : text | json<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>7.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/get_file_info.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>get_file_info</b></code>
            </td>
            <td>Retrieve detailed metadata about a file or directory. Returns comprehensive information including size, creation time, last modified time, permissions, and type. This tool is perfect for understanding file characteristics without reading the actual content. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>8.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/head_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>head_file</b></code>
            </td>
            <td>Reads and returns the first N lines of a text file.This is useful for quickly previewing file contents without loading the entire file into memory.If the file has fewer than N lines, the entire file will be returned.Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>lines</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>9.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/list_allowed_directories.png" width="32" height="32"/>
            </td>
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
            <td>10.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/list_directory.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>list_directory</b></code>
            </td>
            <td>Get a detailed listing of all files and directories in a specified path. Results clearly distinguish between files and directories with <code>FILE</code> and <code>DIR</code> prefixes. This tool is essential for understanding directory structure and finding specific files within a directory. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>11.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/list_directory_with_sizes.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>list_directory_with_sizes</b></code>
            </td>
            <td>Get a detailed listing of all files and directories in a specified path, including sizes. Results clearly distinguish between files and directories with <code>FILE</code> and <code>DIR</code> prefixes. This tool is useful for understanding directory structure and finding specific files within a directory. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>12.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/move_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>move_file</b></code>
            </td>
            <td>Move or rename files and directories. Can move files between directories and rename them in a single operation. If the destination exists, the operation will fail. Works across different directories and can be used for simple renaming within the same directory. Both source and destination must be within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>destination</code> : string<br /></li>
                    <li> <code>source</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>13.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_file_lines.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>read_file_lines</b></code>
            </td>
            <td>Reads lines from a text file starting at a specified line offset (0-based) and continues for the specified number of lines if a limit is provided.This function skips the first <code>offset</code> lines and then reads up to <code>limit</code> lines if specified, or reads until the end of the file otherwise.It's useful for partial reads, pagination, or previewing sections of large text files.Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>limit</code> : integer<br /></li>
                    <li> <code>offset</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>14.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_media_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>read_media_file</b></code>
            </td>
            <td>Reads an image or audio file and returns its Base64-encoded content along with the corresponding MIME type. The max_bytes argument could be used to enforce an upper limit on the size of a file to read if the media file exceeds this limit, the operation will return an error instead of reading the media file. Access is restricted to files within allowed directories only.</td>
            <td>
                <ul>
                    <li> <code>max_bytes</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>15.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_multiple_media_files.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>read_multiple_media_files</b></code>
            </td>
            <td>Reads multiple image or audio files and returns their Base64-encoded contents along with corresponding MIME types. This method is more efficient than reading files individually. The max_bytes argument could be used to enforce an upper limit on the size of a file to read Failed reads for specific files are skipped without interrupting the entire operation. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>max_bytes</code> : integer<br /></li>
                    <li> <code>paths</code> : string [ ]<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>16.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_multiple_text_files.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>read_multiple_text_files</b></code>
            </td>
            <td>Read the contents of multiple text files simultaneously as text. This is more efficient than reading files one by one when you need to analyze or compare multiple files. Each file's content is returned with its path as a reference. Failed reads for individual files won't stop the entire operation. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>paths</code> : string [ ]<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>17.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/read_text_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>read_text_file</b></code>
            </td>
            <td>Read the complete contents of a text file from the file system as text. Handles various text encodings and provides detailed error messages if the file cannot be read. Use this tool when you need to examine the contents of a single file. Optionally include line numbers for precise code targeting. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>path</code> : string<br /></li>
                    <li> <code>with_line_numbers</code> : boolean<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>18.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/search_files.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>search_files</b></code>
            </td>
            <td>Recursively search for files and directories matching a pattern. Searches through all subdirectories from the starting path. The search is case-insensitive and matches partial names. Returns full paths to all matching items.Optional <code>min_bytes</code> and <code>max_bytes</code> arguments can be used to filter files by size, ensuring that only files within the specified byte range are included in the search. This tool is great for finding files when you don't know their exact location or find files by their size.Only searches within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>excludePatterns</code> : string [ ]<br /></li>
                    <li> <code>max_bytes</code> : integer<br /></li>
                    <li> <code>min_bytes</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                    <li> <code>pattern</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>19.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/search_files_content.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>search_files_content</b></code>
            </td>
            <td>Searches for text or regex patterns in the content of files matching matching a GLOB pattern.Returns detailed matches with file path, line number, column number and a preview of matched text.By default, it performs a literal text search; if the <code>is_regex</code> parameter is set to true, it performs a regular expression (regex) search instead.Optional <code>min_bytes</code> and <code>max_bytes</code> arguments can be used to filter files by size, ensuring that only files within the specified byte range are included in the search. Ideal for finding specific code, comments, or text when you don’t know their exact location.</td>
            <td>
                <ul>
                    <li> <code>excludePatterns</code> : string [ ]<br /></li>
                    <li> <code>is_regex</code> : boolean<br /></li>
                    <li> <code>max_bytes</code> : integer<br /></li>
                    <li> <code>min_bytes</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                    <li> <code>pattern</code> : string<br /></li>
                    <li> <code>query</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>20.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/tail_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>tail_file</b></code>
            </td>
            <td>Reads and returns the last N lines of a text file.This is useful for quickly previewing file contents without loading the entire file into memory.If the file has fewer than N lines, the entire file will be returned.Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>lines</code> : integer<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>21.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/unzip_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>unzip_file</b></code>
            </td>
            <td>Extracts the contents of a ZIP archive to a specified target directory.<br/>It takes a source ZIP file path and a target extraction directory.<br/>The tool decompresses all files and directories stored in the ZIP, recreating their structure in the target location.<br/>Both the source ZIP file and the target directory should reside within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>target_path</code> : string<br /></li>
                    <li> <code>zip_file</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>22.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/write_file.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>write_file</b></code>
            </td>
            <td>Create a new file or completely overwrite an existing file with new content. Use with caution as it will overwrite existing files without warning. Handles text content with proper encoding. Only works within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>content</code> : string<br /></li>
                    <li> <code>path</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>23.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/zip_directory.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>zip_directory</b></code>
            </td>
            <td>Creates a ZIP archive by compressing a directory , including files and subdirectories matching a specified glob pattern.<br/>It takes a path to the folder and a glob pattern to identify files to compress and a target path for the resulting ZIP file.<br/>Both the source directory and the target ZIP file should reside within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>input_directory</code> : string<br /></li>
                    <li> <code>pattern</code> : string<br /></li>
                    <li> <code>target_zip_file</code> : string<br /></li>
                </ul>
            </td>
        </tr>
        <tr>
            <td>24.</td>
            <td>
                <img src="https://rust-mcp-stack.github.io/rust-mcp-filesystem/_media/tool_icons/zip_files.png" width="32" height="32"/>
            </td>
            <td>
                <code><b>zip_files</b></code>
            </td>
            <td>Creates a ZIP archive by compressing files. It takes a list of files to compress and a target path for the resulting ZIP file. Both the source files and the target ZIP file should reside within allowed directories.</td>
            <td>
                <ul>
                    <li> <code>input_files</code> : string [ ]<br /></li>
                    <li> <code>target_zip_file</code> : string<br /></li>
                </ul>
            </td>
        </tr>
</tbody>
</table>




<sup>◾ generated by [mcp-discovery](https://github.com/rust-mcp-stack/mcp-discovery)</sup>
<!-- mcp-discovery-render-end -->