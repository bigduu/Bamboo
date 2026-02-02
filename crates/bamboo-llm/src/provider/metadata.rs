use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatResponse};
use crate::error::Result;
use crate::transformer::LLMStream;

/// LLM Provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider ID
    fn provider_id(&self) -> &str;
    
    /// Get provider metadata
    fn metadata(&self) -> &ProviderMetadata;
    
    /// Send a chat request and get a complete response
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    
    /// Send a chat request and stream the response
    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream>;
    
    /// Validate the provider configuration
    async fn validate(&self) -> Result<()>;
}

/// Provider metadata
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    /// Provider ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Provider capabilities
    pub capabilities: ProviderCapabilities,
}

/// Provider capabilities
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports function/tool calling
    pub tool_calling: bool,
    /// Supports vision/image inputs
    pub vision: bool,
    /// Supports JSON mode
    pub json_mode: bool,
}

impl ProviderCapabilities {
    /// Create default capabilities
    pub fn default_capabilities() -> Self {
        Self {
            streaming: true,
            tool_calling: true,
            vision: false,
            json_mode: false,
        }
    }

    /// Enable all capabilities
    pub fn all() -> Self {
        Self {
            streaming: true,
            tool_calling: true,
            vision: true,
            json_mode: true,
        }
    }
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self::default_capabilities()
    }
}
