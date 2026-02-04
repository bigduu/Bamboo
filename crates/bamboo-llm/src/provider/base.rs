use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatResponse};
use futures::{StreamExt, TryStreamExt};
use reqwest::{Client, header};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use std::sync::Arc;
use std::time::Duration;

use crate::error::{LLMError, Result};
use crate::provider::{ProviderConfig, ProviderMetadata, LLMProvider, AuthConfig};
use crate::transformer::{SchemaTransformer, LLMStream};
use crate::auth::{Authenticator, ApiKeyAuth, BearerAuth, DeviceCodeAuth};

/// Base provider implementation
/// Handles common HTTP functionality and delegates schema transformation
pub struct BaseProvider<T: SchemaTransformer> {
    config: ProviderConfig,
    http_client: reqwest_middleware::ClientWithMiddleware,
    transformer: Arc<T>,
    pub metadata: ProviderMetadata,
    authenticator: Arc<dyn Authenticator>,
}

impl<T: SchemaTransformer + 'static> BaseProvider<T> {
    /// Create a new base provider
    pub async fn new(
        config: ProviderConfig,
        transformer: T,
        metadata: ProviderMetadata,
    ) -> Result<Self> {
        // Create retry policy with exponential backoff
        let retry_policy = ExponentialBackoff::builder()
            .base(2)
            .build_with_max_retries(3);
        
        // Create client with retry middleware
        let http_client = reqwest_middleware::ClientBuilder::new(
            Client::builder()
                .timeout(config.timeout)
                .build()
                .map_err(|e| LLMError::Config(e.to_string()))?
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
        
        // Create authenticator based on auth config
        let authenticator: Arc<dyn Authenticator> = match &config.auth {
            AuthConfig::ApiKey { key } => Arc::new(ApiKeyAuth::new(key.clone())),
            AuthConfig::Bearer { token } => Arc::new(BearerAuth::new(token.clone())),
            AuthConfig::DeviceCode { .. } => {
                let auth = DeviceCodeAuth::new();
                auth.init().await.map_err(|e| LLMError::Auth(e.to_string()))?;
                Arc::new(auth)
            }
            AuthConfig::None => Arc::new(crate::auth::NoAuth),
        };

        Ok(Self {
            config,
            http_client,
            transformer: Arc::new(transformer),
            metadata,
            authenticator,
        })
    }

    /// Create with a custom authenticator
    pub fn with_authenticator(
        config: ProviderConfig,
        transformer: T,
        metadata: ProviderMetadata,
        authenticator: Arc<dyn Authenticator>,
    ) -> Result<Self> {
        // Create retry policy with exponential backoff
        let retry_policy = ExponentialBackoff::builder()
            .base(2)
            .build_with_max_retries(3);
        
        let http_client = reqwest_middleware::ClientBuilder::new(
            Client::builder()
                .timeout(config.timeout)
                .build()
                .map_err(|e| LLMError::Config(e.to_string()))?
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        Ok(Self {
            config,
            http_client,
            transformer: Arc::new(transformer),
            metadata,
            authenticator,
        })
    }

    /// Create HTTP client with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self> {
        // Create retry policy with exponential backoff
        let retry_policy = ExponentialBackoff::builder()
            .base(2)
            .build_with_max_retries(3);
        
        self.http_client = reqwest_middleware::ClientBuilder::new(
            Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| LLMError::Config(e.to_string()))?
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
        Ok(self)
    }

    /// Get the provider ID
    pub fn provider_id(&self) -> &str {
        self.transformer.provider_id()
    }

    /// Get the config
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Get the authenticator
    pub fn authenticator(&self) -> &Arc<dyn Authenticator> {
        &self.authenticator
    }

    /// Build request headers
    async fn build_headers(&self) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));

        // Add authentication header
        if let Some((header_name, header_value)) = self.authenticator.get_auth_header().await? {
            let name = header::HeaderName::from_bytes(header_name.as_bytes())
                .map_err(|e| LLMError::Config(format!("Invalid auth header name: {}", e)))?;
            let value = header::HeaderValue::from_str(&header_value)
                .map_err(|e| LLMError::Config(format!("Invalid auth header value: {}", e)))?;
            headers.insert(name, value);
        }

        // Add custom headers from config
        for (key, value) in &self.config.headers {
            let header_name = header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| LLMError::Config(format!("Invalid header name: {}", e)))?;
            let header_value = header::HeaderValue::from_str(value)
                .map_err(|e| LLMError::Config(format!("Invalid header value: {}", e)))?;
            headers.insert(header_name, header_value);
        }

        Ok(headers)
    }

    /// Check and refresh authentication if needed
    async fn ensure_auth(&self) -> Result<()> {
        if self.authenticator.needs_refresh().await {
            self.authenticator.refresh().await?;
        }
        Ok(())
    }

    /// Send a non-streaming request
    pub async fn send_request(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Ensure authentication is valid
        self.ensure_auth().await?;
        
        let body = self.transformer.transform_request(&request)?;
        let headers = self.build_headers().await?;
        
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let response = self.http_client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            return Err(match status.as_u16() {
                401 | 403 => LLMError::Auth(error_text),
                429 => {
                    // For rate limit, we can't get retry-after header since we already consumed response
                    LLMError::RateLimited { retry_after: 60 }
                }
                _ => LLMError::Api {
                    status: status.as_u16(),
                    message: error_text,
                },
            });
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        self.transformer.parse_response(&response_data)
            .map_err(|e| LLMError::Transform(e))
    }

    /// Send a streaming request
    pub async fn send_stream_request(&self, request: ChatRequest) -> Result<LLMStream> {
        // Ensure authentication is valid
        self.ensure_auth().await?;
        
        let mut request = request;
        request.options.stream = true;
        
        let body = self.transformer.transform_request(&request)?;
        let headers = self.build_headers().await?;
        
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let response = self.http_client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let transformer = self.transformer.clone();
        let stream = response
            .bytes_stream()
            .map_err(|e| LLMError::Network(e.to_string()))
            .filter_map(move |result| {
                let transformer = transformer.clone();
                async move {
                    match result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            // Handle SSE format (data: {...})
                            for line in text.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    match transformer.parse_stream_chunk(data.trim()) {
                                        Ok(Some(chunk)) => return Some(Ok(chunk)),
                                        Ok(None) => continue,
                                        Err(e) => return Some(Err(LLMError::Transform(e))),
                                    }
                                }
                            }
                            None
                        }
                        Err(e) => Some(Err(e)),
                    }
                }
            });

        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl<T: SchemaTransformer + 'static> LLMProvider for BaseProvider<T> {
    fn provider_id(&self) -> &str {
        self.provider_id()
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.send_request(request).await
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream> {
        self.send_stream_request(request).await
    }

    async fn validate(&self) -> Result<()> {
        // Simple validation: check if we can build headers
        let _ = self.build_headers().await?;
        Ok(())
    }
}
