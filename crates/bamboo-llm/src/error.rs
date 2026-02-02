use thiserror::Error;

/// Unified error type for LLM operations
#[derive(Error, Debug)]
pub enum LLMError {
    #[error("network error: {0}")]
    Network(String),
    
    #[error("api error: {status} - {message}")]
    Api { status: u16, message: String },
    
    #[error("authentication error: {0}")]
    Auth(String),
    
    #[error("transform error: {0}")]
    Transform(#[from] ConversionError),
    
    #[error("stream error: {0}")]
    Stream(String),
    
    #[error("config error: {0}")]
    Config(String),
    
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },
}

/// Error during schema transformation
#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("missing field: {0}")]
    MissingField(String),
    
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

/// Auth error types
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    Failed(String),
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Device code expired")]
    DeviceCodeExpired,
    
    #[error("Access denied")]
    AccessDenied,
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<AuthError> for LLMError {
    fn from(e: AuthError) -> Self {
        LLMError::Auth(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LLMError>;
