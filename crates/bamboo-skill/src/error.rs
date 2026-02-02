//! Error types for bamboo-skill

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SkillError>;

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Skill already exists: {0}")]
    AlreadyExists(String),

    #[error("Failed to parse SKILL.md: {0}")]
    ParseError(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Tool error: {0}")]
    Tool(#[from] bamboo_tool::ToolError),

    #[error("Watch error: {0}")]
    Watch(String),

    #[error("Send error: {0}")]
    Send(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<notify::Error> for SkillError {
    fn from(e: notify::Error) -> Self {
        SkillError::Watch(e.to_string())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SkillError {
    fn from(e: tokio::sync::mpsc::error::SendError<T>) -> Self {
        SkillError::Send(e.to_string())
    }
}
