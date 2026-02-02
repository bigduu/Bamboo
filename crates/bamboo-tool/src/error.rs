//! Error types for bamboo-tool

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ToolError>;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool execution timed out after {0}ms")]
    Timeout(u64),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    #[error("Argument type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Script not executable: {0}")]
    NotExecutable(String),

    #[error("Command not allowed: {0}")]
    CommandNotAllowed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(String),
}
