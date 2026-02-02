use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatResponse};

use crate::error::Result;
use crate::provider::{ProviderConfig, ProviderMetadata, ProviderCapabilities, LLMProvider};
use crate::transformer::OpenAiTransformer;
use crate::transformer::LLMStream;

/// Forward Provider
/// Forwards requests to another endpoint (e.g., local proxy or forward service)
pub struct ForwardProvider {
    base: super::super::provider::BaseProvider<OpenAiTransformer>,
}

impl ForwardProvider {
    /// Create a new forward provider (async)
    pub async fn new(base_url: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new("forward", base_url);
        Self::with_config(config).await
    }

    /// Create with custom configuration (async)
    pub async fn with_config(config: ProviderConfig) -> Result<Self> {
        let metadata = ProviderMetadata {
            id: "forward".to_string(),
            name: "Forward".to_string(),
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: true,
                json_mode: true,
            },
        };

        let base = super::super::provider::BaseProvider::new(
            config,
            OpenAiTransformer::new(),
            metadata,
        ).await?;

        Ok(Self { base })
    }

    /// Create with API key (async)
    pub async fn with_api_key(base_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new("forward", base_url)
            .with_api_key(api_key);
        
        Self::with_config(config).await
    }
}

#[async_trait]
impl LLMProvider for ForwardProvider {
    fn provider_id(&self) -> &str {
        "forward"
    }

    fn metadata(&self) -> &ProviderMetadata {
        self.base.metadata()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.base.chat(request).await
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream> {
        self.base.chat_stream(request).await
    }

    async fn validate(&self) -> Result<()> {
        self.base.validate().await
    }
}

/// Direct Provider
/// Uses OpenAI API directly (simple wrapper around OpenAiProvider)
pub struct DirectProvider {
    inner: super::openai::OpenAiProvider,
}

impl DirectProvider {
    /// Create a new direct provider
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Ok(Self {
            inner: super::openai::OpenAiProvider::new(api_key)?,
        })
    }

    /// Create with custom base URL
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            inner: super::openai::OpenAiProvider::with_base_url(api_key, base_url)?,
        })
    }

    /// Set model
    pub fn with_model(self, _model: impl Into<String>) -> Self {
        // Note: This is a simplified implementation
        self
    }
}

#[async_trait]
impl LLMProvider for DirectProvider {
    fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }

    fn metadata(&self) -> &ProviderMetadata {
        self.inner.metadata()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.inner.chat(request).await
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream> {
        self.inner.chat_stream(request).await
    }

    async fn validate(&self) -> Result<()> {
        self.inner.validate().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_forward_provider() {
        let provider = ForwardProvider::new("http://localhost:8080").await;
        assert!(provider.is_ok());
    }

    #[test]
    fn test_direct_provider() {
        let provider = DirectProvider::new("test-key");
        assert!(provider.is_ok());
    }
}
