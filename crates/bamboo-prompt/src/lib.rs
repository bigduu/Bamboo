mod manager;
mod models;
mod storage;

pub use manager::PromptManager;
pub use models::SystemPrompt;
pub use storage::PromptStorage;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("prompt not found: {0}")]
    NotFound(String),
}

pub type PromptResult<T> = Result<T, PromptError>;
