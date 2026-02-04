use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatResponse};
use bamboo_core::chat::ChatChunk;
use futures::{StreamExt, TryStreamExt};
use reqwest::header;

use crate::error::{LLMError, Result};
use crate::provider::{ProviderConfig, ProviderMetadata, ProviderCapabilities, LLMProvider};
use crate::transformer::LLMStream;
use crate::adapters::{
    openai_to_anthropic_request,
    anthropic_to_openai_response,
    anthropic_stream_to_openai,
    StreamState,
};
use crate::adapters::anthropic::MessagesStreamEvent;

/// Anthropic Provider
/// Converts OpenAI format requests to Anthropic API format
pub struct AnthropicProvider {
    config: ProviderConfig,
    http_client: reqwest::Client,
    metadata: ProviderMetadata,
}

impl AnthropicProvider {
    /// Create with custom configuration (async)
    pub async fn with_config(config: ProviderConfig) -> Result<Self> {
        let metadata = ProviderMetadata {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: false,
                json_mode: false,
            },
        };

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| LLMError::Config(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
            metadata,
        })
    }

    /// Create a new Anthropic provider with API key
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new("anthropic", "https://api.anthropic.com/v1")
            .with_api_key(api_key);
        
        Self::new_with_config_sync(config)
    }

    /// Create with custom base URL
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new("anthropic", base_url)
            .with_api_key(api_key);
        
        Self::new_with_config_sync(config)
    }
    
    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
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
                    .map_err(|e| LLMError::Config(e.to_string()))?;
                rt.block_on(Self::with_config(config))
            }
        }
    }

    /// Build request headers
    fn build_headers(&self,
    ) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", header::HeaderValue::from_static("2023-06-01"));

        match &self.config.auth {
            crate::provider::AuthConfig::ApiKey { key } => {
                headers.insert(
                    "x-api-key",
                    header::HeaderValue::from_str(key)
                        .map_err(|e| LLMError::Config(format!("Invalid API key: {}", e)))?,
                );
            }
            crate::provider::AuthConfig::Bearer { token } => {
                headers.insert(
                    header::AUTHORIZATION,
                    header::HeaderValue::from_str(&format!("Bearer {}", token))
                        .map_err(|e| LLMError::Config(format!("Invalid bearer token: {}", e)))?,
                );
            }
            _ => {
                return Err(LLMError::Auth("Anthropic requires API key or Bearer token".to_string()));
            }
        }

        for (key, value) in &self.config.headers {
            let header_name = header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| LLMError::Config(format!("Invalid header name: {}", e)))?;
            let header_value = header::HeaderValue::from_str(value)
                .map_err(|e| LLMError::Config(format!("Invalid header value: {}", e)))?;
            headers.insert(header_name, header_value);
        }

        Ok(headers)
    }

    /// Convert internal ChatRequest to OpenAI format
    fn convert_internal_to_openai(&self,
        request: &ChatRequest,
    ) -> Result<crate::adapters::openai::ChatRequest> {
        use crate::adapters::openai::{
            ChatRequest as OpenAIChatRequest, Message as OpenAIMessage, Role, Content,
        };
        use bamboo_core::types::Role as CoreRole;

        let messages: Vec<OpenAIMessage> = request
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    CoreRole::System => Role::System,
                    CoreRole::User => Role::User,
                    CoreRole::Assistant => Role::Assistant,
                    CoreRole::Tool => Role::Tool,
                };

                let content = match &m.content {
                    bamboo_core::types::Content::Text { text } => Some(Content::Text(text.clone())),
                    bamboo_core::types::Content::Parts { parts } => {
                        let content_parts: Vec<crate::adapters::openai::ContentPart> = parts
                            .iter()
                            .map(|p| match p {
                                bamboo_core::types::ContentPart::Text { text } => {
                                    crate::adapters::openai::ContentPart::Text {
                                        text: text.clone(),
                                    }
                                }
                                bamboo_core::types::ContentPart::Image { source } => {
                                    let url = match source {
                                        bamboo_core::types::ImageSource::Base64 { data, mime_type: _ } => {
                                            format!("data:image/jpeg;base64,{}", data)
                                        }
                                        bamboo_core::types::ImageSource::Url { url } => url.clone(),
                                    };
                                    crate::adapters::openai::ContentPart::ImageUrl {
                                        image_url: crate::adapters::openai::ImageUrl { url },
                                    }
                                }
                            })
                            .collect();
                        Some(Content::Parts(content_parts))
                    }
                };

                let tool_calls = m.tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| crate::adapters::openai::ToolCall {
                            id: tc.id.clone(),
                            tool_type: crate::adapters::openai::ToolType::Function,
                            function: crate::adapters::openai::FunctionCall {
                                name: tc.name.clone(),
                                arguments: tc.arguments.to_string(),
                            },
                        })
                        .collect()
                });

                OpenAIMessage {
                    role,
                    content,
                    tool_calls,
                    tool_call_id: m.tool_call_id.clone(),
                    extra: std::collections::HashMap::new(),
                }
            })
            .collect();

        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| crate::adapters::openai::ToolDefinition {
                        tool_type: crate::adapters::openai::ToolType::Function,
                        function: crate::adapters::openai::FunctionDefinition {
                            name: t.name.clone(),
                            description: Some(t.description.clone()),
                            parameters: Some(t.parameters.clone()),
                        },
                    })
                    .collect(),
            )
        };

        Ok(OpenAIChatRequest {
            model: request.model.clone(),
            messages,
            stream: request.options.stream,
            temperature: request.options.temperature,
            max_tokens: request.options.max_tokens,
            top_p: request.options.top_p,
            tools,
            tool_choice: None,
            response_format: None,
            extra: std::collections::HashMap::new(),
        })
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn provider_id(&self) -> &str {
        &self.metadata.id
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn chat(&self,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let openai_req = self.convert_internal_to_openai(&request)?;
        let anthropic_req = openai_to_anthropic_request(&openai_req)
            .map_err(|e| LLMError::Transform(e))?;

        let headers = self.build_headers()?;
        let url = format!("{}/messages", self.config.base_url);

        let response = self.http_client
            .post(&url)
            .headers(headers)
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => LLMError::Auth(error_text),
                429 => LLMError::RateLimited { retry_after: 60 },
                _ => LLMError::Api {
                    status: status.as_u16(),
                    message: error_text,
                },
            });
        }

        let anthropic_resp: crate::adapters::anthropic::MessagesResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        let openai_resp = anthropic_to_openai_response(&anthropic_resp, &request.model)
            .map_err(|e| LLMError::Transform(e))?;

        let message = bamboo_core::types::Message {
            id: format!("msg_{}", openai_resp.id),
            role: bamboo_core::types::Role::Assistant,
            content: bamboo_core::types::Content::Text {
                text: match &openai_resp.choices[0].message.content {
                    Some(crate::adapters::openai::Content::Text(t)) => t.clone(),
                    _ => String::new(),
                },
            },
            tool_calls: openai_resp.choices[0].message.tool_calls.as_ref().map(|tcs| {
                tcs.iter()
                    .map(|tc| bamboo_core::types::ToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: serde_json::from_str(&tc.function.arguments)
                            .unwrap_or_else(|_| serde_json::json!({})),
                    })
                    .collect()
            }),
            tool_call_id: None,
            metadata: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        let tool_calls = openai_resp.choices[0]
            .message
            .tool_calls
            .as_ref()
            .map(|tcs| {
                tcs.iter()
                    .map(|tc| bamboo_core::types::ToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: serde_json::from_str(&tc.function.arguments)
                            .unwrap_or_else(|_| serde_json::json!({})),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = openai_resp.usage.map(|u| bamboo_core::chat::ChatUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or_default();

        let finish_reason = match openai_resp.choices[0].finish_reason.as_deref() {
            Some("stop") => bamboo_core::chat::FinishReason::Stop,
            Some("length") => bamboo_core::chat::FinishReason::Length,
            Some("tool_calls") => bamboo_core::chat::FinishReason::ToolCalls,
            Some("content_filter") => bamboo_core::chat::FinishReason::ContentFilter,
            _ => bamboo_core::chat::FinishReason::Stop,
        };

        Ok(ChatResponse {
            id: openai_resp.id,
            model: request.model,
            message,
            tool_calls,
            usage,
            finish_reason,
        })
    }

    async fn chat_stream(&self,
        request: ChatRequest,
    ) -> Result<LLMStream> {
        let mut openai_req = self.convert_internal_to_openai(&request)?;
        openai_req.stream = true;
        
        let anthropic_req = openai_to_anthropic_request(&openai_req)
            .map_err(|e| LLMError::Transform(e))?;

        let headers = self.build_headers()?;
        let url = format!("{}/messages", self.config.base_url);

        let response = self.http_client
            .post(&url)
            .headers(headers)
            .json(&anthropic_req)
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

        let model = request.model.clone();
        let stream = response
            .bytes_stream()
            .map_err(|e| LLMError::Network(e.to_string()))
            .filter_map(move |result| {
                let model = model.clone();
                async move {
                    match result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            let mut state = StreamState::new();
                            
                            for line in text.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        return Some(Ok(ChatChunk::finish(
                                            bamboo_core::chat::FinishReason::Stop
                                        )));
                                    }
                                    
                                    match serde_json::from_str::<MessagesStreamEvent>(data.trim()) {
                                        Ok(event) => {
                                            match anthropic_stream_to_openai(&event, &mut state) {
                                                Ok(Some(chunk)) => {
                                                    let chat_chunk = convert_stream_chunk(chunk, &model);
                                                    return Some(Ok(chat_chunk));
                                                }
                                                Ok(None) => continue,
                                                Err(e) => return Some(Err(LLMError::Transform(e))),
                                            }
                                        }
                                        Err(_) => continue,
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

    async fn validate(&self) -> Result<()> {
        let _ = self.build_headers()?;
        Ok(())
    }
}

fn convert_stream_chunk(
    chunk: crate::adapters::openai::ChatStreamChunk,
    _model: &str,
) -> ChatChunk {
    if let Some(choice) = chunk.choices.first() {
        if let Some(ref finish) = choice.finish_reason {
            return match finish.as_str() {
                "stop" => ChatChunk::finish(bamboo_core::chat::FinishReason::Stop),
                "length" => ChatChunk::finish(bamboo_core::chat::FinishReason::Length),
                "tool_calls" => ChatChunk::finish(bamboo_core::chat::FinishReason::ToolCalls),
                _ => ChatChunk::finish(bamboo_core::chat::FinishReason::Stop),
            };
        }

        if let Some(ref content) = choice.delta.content {
            return ChatChunk::content(content.clone());
        }
    }

    if let Some(usage) = chunk.usage {
        return ChatChunk::Usage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        };
    }

    ChatChunk::content("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_provider() {
        let provider = AnthropicProvider::new("test-key");
        assert!(provider.is_ok());
    }

    #[test]
    fn test_with_base_url() {
        let provider = AnthropicProvider::with_base_url("test-key", "https://custom.anthropic.com/v1");
        assert!(provider.is_ok());
    }
}
