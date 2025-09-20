mod calculate_directory_size;
mod create_directory;
mod directory_tree;
mod edit_file;
mod find_duplicate_files;
mod find_empty_directories;
mod get_file_info;
mod head_file;
mod list_allowed_directories;
mod list_directory;
mod list_directory_with_sizes;
mod move_file;
mod read_file_lines;
mod read_media_file;
mod read_multiple_media_files;
mod read_multiple_text_files;
mod read_text_file;
mod search_file;
mod search_files_content;
mod tail_file;
mod write_file;
mod zip_unzip;

pub use create_directory::CreateDirectoryTool;
pub use directory_tree::DirectoryTreeTool;
pub use edit_file::{EditFileTool, EditOperation};
pub use get_file_info::GetFileInfoTool;
pub use head_file::HeadFileTool;
pub use list_allowed_directories::ListAllowedDirectoriesTool;
pub use list_directory::ListDirectoryTool;
pub use list_directory_with_sizes::ListDirectoryWithSizesTool;
pub use move_file::MoveFileTool;
pub use read_media_file::ReadMediaFileTool;
pub use read_multiple_media_files::ReadMultipleMediaFilesTool;
pub use read_multiple_text_files::ReadMultipleTextFilesTool;
pub use read_text_file::ReadTextFileTool;
pub use rust_mcp_sdk::tool_box;
pub use search_file::SearchFilesTool;
pub use search_files_content::SearchFilesContentTool;
pub use write_file::WriteFileTool;
pub use zip_unzip::{UnzipFileTool, ZipDirectoryTool, ZipFilesTool};

//Generate FileSystemTools enum , tools() function, and TryFrom<CallToolRequestParams> trait implementation
tool_box!(
    FileSystemTools,
    [
        ReadTextFileTool,
        CreateDirectoryTool,
        DirectoryTreeTool,
        EditFileTool,
        GetFileInfoTool,
        ListAllowedDirectoriesTool,
        ListDirectoryTool,
        MoveFileTool,
        ReadMultipleTextFilesTool,
        SearchFilesTool,
        WriteFileTool,
        ZipFilesTool,
        UnzipFileTool,
        ZipDirectoryTool,
        SearchFilesContentTool,
        ListDirectoryWithSizesTool,
        ReadMediaFileTool,
        ReadMultipleMediaFilesTool,
        HeadFileTool
    ]
);

impl FileSystemTools {
    // Determines whether the filesystem tool requires write access to the filesystem.
    // Returns `true` for tools that modify files or directories, and `false` otherwise.
    pub fn require_write_access(&self) -> bool {
        match self {
            FileSystemTools::CreateDirectoryTool(_)
            | FileSystemTools::MoveFileTool(_)
            | FileSystemTools::WriteFileTool(_)
            | FileSystemTools::EditFileTool(_)
            | FileSystemTools::ZipFilesTool(_)
            | FileSystemTools::UnzipFileTool(_)
            | FileSystemTools::ZipDirectoryTool(_) => true,
            FileSystemTools::ReadTextFileTool(_)
            | FileSystemTools::DirectoryTreeTool(_)
            | FileSystemTools::GetFileInfoTool(_)
            | FileSystemTools::ListAllowedDirectoriesTool(_)
            | FileSystemTools::ListDirectoryTool(_)
            | FileSystemTools::ReadMultipleTextFilesTool(_)
            | FileSystemTools::SearchFilesContentTool(_)
            | FileSystemTools::ListDirectoryWithSizesTool(_)
            | FileSystemTools::ReadMediaFileTool(_)
            | FileSystemTools::HeadFileTool(_)
            | FileSystemTools::ReadMultipleMediaFilesTool(_)
            | FileSystemTools::SearchFilesTool(_) => false,
        }
    }
}
