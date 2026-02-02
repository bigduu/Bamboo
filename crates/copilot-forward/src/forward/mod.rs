//! HTTP forwarding to Copilot API

use reqwest::{Client, header::HeaderMap};
use serde::{Deserialize, Serialize};

const COPILOT_BASE_URL: &str = "https://api.githubcopilot.com";

/// Copilot API client for forwarding requests
pub struct ForwardClient {
    client: Client,
    token: String,
    base_url: String,
}

impl ForwardClient {
    /// Create new forward client with token
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
            base_url: COPILOT_BASE_URL.to_string(),
        }
    }
    
    /// Build Copilot request headers
    fn build_headers(&self) -> HeaderMap {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, ACCEPT, ACCEPT_ENCODING, USER_AGENT};
        
        let mut headers = HeaderMap::new();
        
        // Authorization
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap(),
        );
        
        // Content type
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        // Accept headers
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
        
        // Copilot extension headers (mimic VS Code)
        headers.insert("editor-version", HeaderValue::from_static("vscode/1.99.2"));
        headers.insert("editor-plugin-version", HeaderValue::from_static("copilot-chat/0.20.3"));
        headers.insert("openai-organization", HeaderValue::from_static("github-copilot"));
        headers.insert("openai-intent", HeaderValue::from_static("conversation-panel"));
        headers.insert("copilot-integration-id", HeaderValue::from_static("vscode-chat"));
        
        // User agent
        headers.insert(USER_AGENT, HeaderValue::from_static("GitHubCopilotChat/0.20.3"));
        
        headers
    }
    
    /// Chat completions API
    pub async fn chat_completions(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<reqwest::Response, ForwardError> {
        let url = format!("{}/chat/completions", self.base_url);
        
        let response = self.client
            .post(&url)
            .headers(self.build_headers())
            .json(request)
            .send()
            .await
            .map_err(|e| ForwardError::Http(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ForwardError::Api(format!("HTTP {}: {}", status, text)));
        }
        
        Ok(response)
    }
    
    /// Get available models
    pub async fn models(&self) -> Result<Vec<Model>, ForwardError> {
        let url = format!("{}/models", self.base_url);
        
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| ForwardError::Http(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ForwardError::Api(format!("HTTP {}: {}", status, text)));
        }
        
        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| ForwardError::Parse(e.to_string()))?;
        
        Ok(models_response.data)
    }
    
    /// Check if token is valid by making a test request
    pub async fn check_token(&self) -> Result<(), ForwardError> {
        // Try to get models as a lightweight check
        self.models().await?;
        Ok(())
    }
}

/// Chat completion request
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Chat message
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Models response
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    pub data: Vec<Model>,
}

/// Model information
#[derive(Debug, Deserialize, Clone)]
pub struct Model {
    pub id: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

/// Forward errors
#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    #[error("HTTP error: {0}")]
    Http(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Token expired or invalid")]
    Unauthorized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_headers() {
        let client = ForwardClient::new("test_token");
        let headers = client.build_headers();
        
        assert!(headers.contains_key("authorization"));
        assert!(headers.contains_key("editor-version"));
    }
}
