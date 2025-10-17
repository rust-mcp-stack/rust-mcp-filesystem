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

pub use calculate_directory_size::{CalculateDirectorySize, FileSizeOutputFormat};
pub use create_directory::CreateDirectory;
pub use directory_tree::DirectoryTree;
pub use edit_file::{EditFile, EditOperation, RegexEditOptions};
pub use find_duplicate_files::FindDuplicateFiles;
pub use find_empty_directories::FindEmptyDirectories;
pub use get_file_info::GetFileInfo;
pub use head_file::HeadFile;
pub use list_allowed_directories::ListAllowedDirectories;
pub use list_directory::ListDirectory;
pub use list_directory_with_sizes::ListDirectoryWithSizes;
pub use move_file::MoveFile;
pub use read_file_lines::ReadFileLines;
pub use read_media_file::ReadMediaFile;
pub use read_multiple_media_files::ReadMultipleMediaFiles;
pub use read_multiple_text_files::ReadMultipleTextFiles;
pub use read_text_file::ReadTextFile;
pub use rust_mcp_sdk::tool_box;
pub use search_file::SearchFiles;
pub use search_files_content::SearchFilesContent;
pub use tail_file::TailFile;
pub use write_file::WriteFile;
pub use zip_unzip::{UnzipFile, ZipDirectory, ZipFiles};
//Generate FileSystemTools enum , tools() function, and TryFrom<CallToolRequestParams> trait implementation
tool_box!(
    FileSystemTools,
    [
        ReadTextFile,
        CreateDirectory,
        DirectoryTree,
        EditFile,
        GetFileInfo,
        ListAllowedDirectories,
        ListDirectory,
        MoveFile,
        ReadMultipleTextFiles,
        SearchFiles,
        WriteFile,
        ZipFiles,
        UnzipFile,
        ZipDirectory,
        SearchFilesContent,
        ListDirectoryWithSizes,
        ReadMediaFile,
        ReadMultipleMediaFiles,
        HeadFile,
        TailFile,
        ReadFileLines,
        FindEmptyDirectories,
        CalculateDirectorySize,
        FindDuplicateFiles
    ]
);

impl FileSystemTools {
    // Determines whether the filesystem tool requires write access to the filesystem.
    // Returns `true` for tools that modify files or directories, and `false` otherwise.
    pub fn require_write_access(&self) -> bool {
        match self {
            FileSystemTools::CreateDirectory(_)
            | FileSystemTools::MoveFile(_)
            | FileSystemTools::WriteFile(_)
            | FileSystemTools::EditFile(_)
            | FileSystemTools::ZipFiles(_)
            | FileSystemTools::UnzipFile(_)
            | FileSystemTools::ZipDirectory(_) => true,
            FileSystemTools::ReadTextFile(_)
            | FileSystemTools::DirectoryTree(_)
            | FileSystemTools::GetFileInfo(_)
            | FileSystemTools::ListAllowedDirectories(_)
            | FileSystemTools::ListDirectory(_)
            | FileSystemTools::ReadMultipleTextFiles(_)
            | FileSystemTools::SearchFilesContent(_)
            | FileSystemTools::ListDirectoryWithSizes(_)
            | FileSystemTools::ReadMediaFile(_)
            | FileSystemTools::HeadFile(_)
            | FileSystemTools::ReadMultipleMediaFiles(_)
            | FileSystemTools::TailFile(_)
            | FileSystemTools::ReadFileLines(_)
            | FileSystemTools::FindEmptyDirectories(_)
            | FileSystemTools::CalculateDirectorySize(_)
            | FileSystemTools::FindDuplicateFiles(_)
            | FileSystemTools::SearchFiles(_) => false,
        }
    }
}
