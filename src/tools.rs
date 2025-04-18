mod create_directory;
mod directory_tree;
mod edit_file;
mod get_file_info;
mod list_allowed_directories;
mod list_directory;
mod move_file;
mod read_files;
mod read_multiple_files;
mod search_file;
mod write_file;
mod zip_unzip;

pub use create_directory::CreateDirectoryTool;
pub use directory_tree::DirectoryTreeTool;
pub use edit_file::{EditFileTool, EditOperation};
pub use get_file_info::GetFileInfoTool;
pub use list_allowed_directories::ListAllowedDirectoriesTool;
pub use list_directory::ListDirectoryTool;
pub use move_file::MoveFileTool;
pub use read_files::ReadFileTool;
pub use read_multiple_files::ReadMultipleFilesTool;
pub use rust_mcp_sdk::tool_box;
pub use search_file::SearchFilesTool;
pub use write_file::WriteFileTool;
pub use zip_unzip::{UnzipFileTool, ZipDirectoryTool, ZipFilesTool};

//Generate FileSystemTools enum , tools() function, and TryFrom<CallToolRequestParams> trait implementation
tool_box!(
    FileSystemTools,
    [
        ReadFileTool,
        CreateDirectoryTool,
        DirectoryTreeTool,
        EditFileTool,
        GetFileInfoTool,
        ListAllowedDirectoriesTool,
        ListDirectoryTool,
        MoveFileTool,
        ReadMultipleFilesTool,
        SearchFilesTool,
        WriteFileTool,
        ZipFilesTool,
        UnzipFileTool,
        ZipDirectoryTool
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

            FileSystemTools::ReadFileTool(_)
            | FileSystemTools::DirectoryTreeTool(_)
            | FileSystemTools::GetFileInfoTool(_)
            | FileSystemTools::ListAllowedDirectoriesTool(_)
            | FileSystemTools::ListDirectoryTool(_)
            | FileSystemTools::ReadMultipleFilesTool(_)
            | FileSystemTools::SearchFilesTool(_) => false,
        }
    }
}
