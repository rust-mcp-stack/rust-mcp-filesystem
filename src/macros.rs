/// Generates a `match` expression for dispatching `FileSystemTools` variants to their respective `run_tool` methods.
///
/// This macro reduces boilerplate in matching `FileSystemTools` enum variants by generating a `match` arm
/// for each specified tool. Each arm calls the tool's `run_tool` method with the provided parameters and
/// filesystem service, handling the async dispatch uniformly.
///
/// # Parameters
/// - `$params:expr`: The expression to match against, expected to be a `FileSystemTools` enum value.
/// - `$fs_service:expr`: The filesystem service reference (e.g., `&self.fs_service`) to pass to each tool's `run_tool` method.
/// - `$($tool:ident),*`: A comma-separated list of tool identifiers (e.g., `ReadMediaFileTool`, `WriteFileTool`) corresponding to
///   `FileSystemTools` variants and their associated types.
///
/// # Usage
/// The macro is typically used within a method that dispatches filesystem operations based on a `FileSystemTools` enum.
/// Each tool must have a `run_tool` method with the signature:
/// ```rust
/// async fn run_tool(params: ParamsType, fs_service: &FsService) -> ServiceResult<()>
/// ```
/// where `ParamsType` is the parameter type for the specific tool, and `FsService` is the filesystem service type.
///
/// # Example
/// ```rust
/// match_filesystem_tools!(
///     tool_params,
///     &self.fs_service,
///     ReadMediaFileTool,
///     WriteFileTool
/// )
/// ```
/// This expands to:
/// ```rust
/// match tool_params {
///     FileSystemTools::ReadMediaFileTool(params) => ReadMediaFileTool::run_tool(params, &self.fs_service).await,
///     FileSystemTools::WriteFileTool(params) => WriteFileTool::run_tool(params, &self.fs_service).await,
/// }
/// ```
///
/// # Notes
/// - Ensure each tool identifier matches a variant of the `FileSystemTools` enum and has a corresponding `run_tool` method.
/// - The macro assumes all `run_tool` methods are `async` and return `ServiceResult<()>`.
/// - To add a new tool, include its identifier in the macro invocation list.
/// - If a tool has a different `run_tool` signature, a separate macro or manual `match` arm may be needed.
#[macro_export]
macro_rules! invoke_tools {
    ($params:expr, $fs_service:expr, $($tool:ident),* $(,)?) => {
        match $params {
            $(
                FileSystemTools::$tool(params) => $tool::run_tool(params, $fs_service).await,
            )*
        }
    };
}
