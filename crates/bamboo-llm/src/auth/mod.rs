use async_trait::async_trait;
use crate::error::Result;

/// Authenticator trait for different authentication methods
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Get the authentication header (header_name, header_value)
    /// Returns None if no authentication is needed
    async fn get_auth_header(&self) -> Result<Option<(String, String)>>;
    
    /// Check if the authentication needs refresh
    async fn needs_refresh(&self) -> bool;
    
    /// Refresh the authentication token
    /// Returns Ok(()) if refresh was successful
    async fn refresh(&self) -> Result<()>;
}

/// API Key authenticator
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

impl ApiKeyAuth {
    /// Create a new API key authenticator
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl Authenticator for ApiKeyAuth {
    async fn get_auth_header(&self) -> Result<Option<(String, String)>> {
        Ok(Some((
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        )))
    }
    
    async fn needs_refresh(&self) -> bool {
        false
    }
    
    async fn refresh(&self) -> Result<()> {
        Ok(())
    }
}

/// Bearer token authenticator
#[derive(Debug, Clone)]
pub struct BearerAuth {
    token: String,
}

impl BearerAuth {
    /// Create a new bearer authenticator
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl Authenticator for BearerAuth {
    async fn get_auth_header(&self) -> Result<Option<(String, String)>> {
        Ok(Some((
            "Authorization".to_string(),
            format!("Bearer {}", self.token),
        )))
    }
    
    async fn needs_refresh(&self) -> bool {
        false
    }
    
    async fn refresh(&self) -> Result<()> {
        Ok(())
    }
}

/// No authentication
#[derive(Debug, Clone)]
pub struct NoAuth;

#[async_trait]
impl Authenticator for NoAuth {
    async fn get_auth_header(&self) -> Result<Option<(String, String)>> {
        Ok(None)
    }
    
    async fn needs_refresh(&self) -> bool {
        false
    }
    
    async fn refresh(&self) -> Result<()> {
        Ok(())
    }
}

pub mod device_code;
pub mod token;
pub mod cache;

pub use device_code::DeviceCodeAuth;
pub use token::{poll_access_token, get_copilot_token, CopilotToken};
pub use cache::TokenCache;
