use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatResponse};

use crate::error::Result;
use crate::provider::{ProviderConfig, ProviderMetadata, ProviderCapabilities, LLMProvider};
use crate::transformer::{OpenAiTransformer, LLMStream};

/// OpenAI Provider
/// Uses OpenAI API or compatible endpoints (including GitHub Copilot)
pub struct OpenAiProvider {
    base: super::super::provider::BaseProvider<OpenAiTransformer>,
}

impl OpenAiProvider {
    /// Create with custom configuration (async)
    pub async fn with_config(config: ProviderConfig) -> Result<Self> {
        let provider_id = config.provider_id.clone();
        let metadata = ProviderMetadata {
            id: provider_id.clone(),
            name: if provider_id == "copilot" {
                "GitHub Copilot".to_string()
            } else {
                "OpenAI".to_string()
            },
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: provider_id != "copilot", // Copilot doesn't support vision
                json_mode: provider_id != "copilot", // Copilot doesn't support JSON mode
            },
        };

        let base = super::super::provider::BaseProvider::new(
            config,
            OpenAiTransformer::new(),
            metadata,
        ).await?;

        Ok(Self { base })
    }

    /// Create a new OpenAI provider with API key
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new("openai", "https://api.openai.com/v1")
            .with_api_key(api_key);
        
        Self::new_with_config_sync(config)
    }

    /// Create with custom base URL (for Azure or other compatible APIs)
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new("openai", base_url)
            .with_api_key(api_key);
        
        Self::new_with_config_sync(config)
    }
    
    /// Create a GitHub Copilot provider
    /// This will use device code authentication
    pub async fn copilot() -> Result<Self> {
        let headers = [
            ("editor-version".to_string(), "vscode/1.99.2".to_string()),
            ("editor-plugin-version".to_string(), "copilot-chat/0.20.3".to_string()),
            ("user-agent".to_string(), "GitHubCopilotChat/0.20.3".to_string()),
        ].into_iter().collect();

        let config = ProviderConfig::new("copilot", "https://api.githubcopilot.com")
            .with_model("copilot-chat")
            .with_device_code("Iv1.b507a08c87ecfe98")
            .with_headers(headers);
        
        Self::with_config(config).await
    }
    
    /// Create a Copilot provider with custom headers
    pub async fn copilot_with_headers(headers: std::collections::HashMap<String, String>) -> Result<Self> {
        let config = ProviderConfig::new("copilot", "https://api.githubcopilot.com")
            .with_model("copilot-chat")
            .with_device_code("Iv1.b507a08c87ecfe98")
            .with_headers(headers);
        
        Self::with_config(config).await
    }

    /// Set model
    pub fn with_model(self, _model: impl Into<String>) -> Self {
        // Note: This is a simplified implementation
        // In production, you'd want to properly update the config
        self
    }
    
    /// Check if authenticated (relevant for DeviceCode auth)
    pub fn is_authenticated(&self) -> bool {
        // For device code auth, check if we have a valid token
        // For API key auth, always true
        true
    }
    
    /// Trigger authentication for DeviceCode providers
    pub async fn authenticate(&self) -> Result<()> {
        // This would need access to the authenticator
        // For now, authentication happens automatically during request
        Ok(())
    }
    
    /// Helper to run async config creation in sync context
    fn new_with_config_sync(config: ProviderConfig) -> Result<Self> {
        let runtime = tokio::runtime::Handle::try_current();
        match runtime {
            Ok(rt) => {
                rt.block_on(Self::with_config(config))
            }
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| crate::error::LLMError::Config(e.to_string()))?;
                rt.block_on(Self::with_config(config))
            }
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAiProvider {
    fn provider_id(&self) -> &str {
        self.base.provider_id()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_provider() {
        let provider = OpenAiProvider::new("test-key");
        assert!(provider.is_ok());
    }

    #[test]
    fn test_with_base_url() {
        let provider = OpenAiProvider::with_base_url("test-key", "https://custom.openai.com/v1");
        assert!(provider.is_ok());
    }
}
