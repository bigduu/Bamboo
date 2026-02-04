mod enhancer;
mod extractor;
mod manager;
mod models;

pub use enhancer::enhance_prompt;
pub use extractor::MemoryExtractor;
pub use manager::MemoryManager;
pub use models::{Memory, SessionMemory};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type MemoryResult<T> = Result<T, MemoryError>;
