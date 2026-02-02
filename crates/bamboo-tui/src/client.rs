use reqwest::Client;
use serde::{Deserialize, Serialize};
use eventsource_stream::Eventsource;
use futures::StreamExt;

#[derive(Debug, Clone)]
pub struct AgentClient {
    base_url: String,
    client: Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub stream_url: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryResponse {
    pub session_id: String,
    pub messages: Vec<HistoryMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(rename = "tool_calls")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(rename = "tool_call_id")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Token { content: String },
    ToolStart { tool_call_id: String, tool_name: String, arguments: serde_json::Value },
    ToolComplete { tool_call_id: String, result: ToolResult },
    ToolError { tool_call_id: String, error: String },
    Complete { usage: TokenUsage },
    Error { message: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub result: String,
    #[serde(rename = "display_preference")]
    pub display_preference: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenUsage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

impl AgentClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn health_check(&self) -> bool {
        match self.client
            .get(format!("{}/api/v1/health", self.base_url))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn send_message(
        &self,
        message: &str,
        session_id: Option<&str>,
    ) -> anyhow::Result<ChatResponse> {
        let request = ChatRequest {
            message: message.to_string(),
            session_id: session_id.map(|s| s.to_string()),
        };

        let response = self.client
            .post(format!("{}/api/v1/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await?;
            anyhow::bail!("Failed to send message: {}", text);
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    pub async fn stream_events(
        &self,
        session_id: &str,
        tx: tokio::sync::mpsc::Sender<AgentEvent>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/v1/stream/{}", self.base_url, session_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await?;
            anyhow::bail!("Failed to connect to stream: {}", text);
        }

        let mut stream = response
            .bytes_stream()
            .eventsource();

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        break;
                    }

                    // Try to parse as SSE event
                    match serde_json::from_str::<SseEvent>(&event.data) {
                        Ok(sse_event) => {
                            let agent_event = match sse_event.event_type.as_str() {
                                "token" => AgentEvent::Token {
                                    content: sse_event.content.unwrap_or_default(),
                                },
                                "tool_start" => AgentEvent::ToolStart {
                                    tool_call_id: sse_event.tool_call_id.unwrap_or_default(),
                                    tool_name: sse_event.tool_name.unwrap_or_default(),
                                    arguments: sse_event.arguments.unwrap_or(serde_json::Value::Null),
                                },
                                "tool_complete" => AgentEvent::ToolComplete {
                                    tool_call_id: sse_event.tool_call_id.unwrap_or_default(),
                                    result: sse_event.result.unwrap_or(ToolResult {
                                        success: false,
                                        result: "Unknown".to_string(),
                                        display_preference: None,
                                    }),
                                },
                                "tool_error" => AgentEvent::ToolError {
                                    tool_call_id: sse_event.tool_call_id.unwrap_or_default(),
                                    error: sse_event.error.unwrap_or_default(),
                                },
                                "complete" => AgentEvent::Complete {
                                    usage: sse_event.usage.unwrap_or(TokenUsage {
                                        prompt_tokens: 0,
                                        completion_tokens: 0,
                                        total_tokens: 0,
                                    }),
                                },
                                "error" => AgentEvent::Error {
                                    message: sse_event.error.unwrap_or_default(),
                                },
                                _ => continue,
                            };

                            if tx.send(agent_event).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse SSE event: {} - data: {}", e, event.data);
                        }
                    }
                }
                Err(e) => {
                    log::error!("SSE stream error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn stop_generation(&self, session_id: &str) -> anyhow::Result<()> {
        let response = self.client
            .post(format!("{}/api/v1/stop/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await?;
            anyhow::bail!("Failed to stop generation: {}", text);
        }

        Ok(())
    }

    pub async fn get_history(&self, session_id: &str) -> anyhow::Result<HistoryResponse> {
        let response = self.client
            .get(format!("{}/api/v1/history/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await?;
            anyhow::bail!("Failed to get history: {}", text);
        }

        let history: HistoryResponse = response.json().await?;
        Ok(history)
    }
}

#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    event_type: String,
    content: Option<String>,
    #[serde(rename = "tool_call_id")]
    tool_call_id: Option<String>,
    #[serde(rename = "tool_name")]
    tool_name: Option<String>,
    arguments: Option<serde_json::Value>,
    result: Option<ToolResult>,
    error: Option<String>,
    usage: Option<TokenUsage>,
}
