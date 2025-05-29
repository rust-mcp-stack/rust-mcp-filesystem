use async_zip::error::ZipError;
use glob::PatternError;
use rust_mcp_sdk::schema::{schema_utils::SdkError, RpcError};
use rust_mcp_sdk::{error::McpSdkError, TransportError};

use thiserror::Error;
use tokio::io;

pub type ServiceResult<T> = core::result::Result<T, ServiceError>;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service is running in read-only mode. To enable write access, please run with the --allow-write flag.")]
    NoWriteAccess,
    #[error("{0}")]
    FromString(String),
    #[error("{0}")]
    TransportError(#[from] TransportError),
    #[error("{0}")]
    SdkError(#[from] SdkError),
    #[error("{0}")]
    RpcError(#[from] RpcError),
    #[error("{0}")]
    IoError(#[from] io::Error),
    #[error("{0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("{0}")]
    McpSdkError(#[from] McpSdkError),
    #[error("{0}")]
    ZipError(#[from] ZipError),
    #[error("{0}")]
    GlobPatternError(#[from] PatternError),
}
