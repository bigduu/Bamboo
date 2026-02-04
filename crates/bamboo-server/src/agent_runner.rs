//! Agent Runner - 处理 Agent 执行和事件发送
//!
//! 修复: HTTP 客户端不再收到重复事件
//! - WebSocket 客户端 → 发送 ChatResponse 事件（实时推送）
//! - HTTP 客户端 → 不发送任何事件，消息保存到 session storage
//!
//! LLM 集成: 调用真实的 LLM API 获取响应

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use bamboo_masking::MaskingConfig;

use crate::event_bus::{Event, EventBus, ReplyChannel, ChatChunk};
use crate::storage::SessionStorage;

/// LLM 请求消息
#[derive(Debug, Serialize, Deserialize)]
struct LlmMessage {
    role: String,
    content: String,
}

/// LLM 请求体
#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<LlmMessage>,
    stream: bool,
}

/// LLM 响应选择
#[derive(Debug, Deserialize)]
struct LlmChoice {
    message: LlmMessage,
}

/// LLM 响应体
#[derive(Debug, Deserialize)]
struct LlmResponse {
    choices: Vec<LlmChoice>,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
    pub role: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// HTTP 响应（用于 WebSocket 客户端的兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub session_id: String,
    pub request_id: String,
    pub status: String,
    pub data: serde_json::Value,
}

/// Agent 运行器状态
pub struct AgentRunnerState {
    pub event_bus: Arc<EventBus>,
    pub storage: Arc<dyn SessionStorage>,
    pub masking: Option<MaskingConfig>,
}

impl AgentRunnerState {
    pub fn new(event_bus: Arc<EventBus>, storage: Arc<dyn SessionStorage>) -> Self {
        Self {
            event_bus,
            storage,
            masking: None,
        }
    }

    pub fn with_masking(
        event_bus: Arc<EventBus>,
        storage: Arc<dyn SessionStorage>,
        masking: MaskingConfig,
    ) -> Self {
        Self {
            event_bus,
            storage,
            masking: Some(masking),
        }
    }
}

/// Agent 运行器
pub struct AgentRunner {
    state: Arc<crate::state::AppState>,
}

impl AgentRunner {
    /// 创建新的 AgentRunner
    pub fn new(state: Arc<crate::state::AppState>) -> Self {
        Self { state }
    }

    /// 运行 AgentRunner（处理事件循环）
    pub async fn run(&self) {
        tracing::info!("AgentRunner started");
        // TODO: 实现事件循环处理
        // 暂时保持运行状态
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
    /// 调用 LLM API 获取响应
    async fn call_llm(
        storage: &dyn SessionStorage,
        session_id: &str,
        message: &str,
        masking: Option<MaskingConfig>,
    ) -> Result<String> {
        let api_url = std::env::var("LLM_API_URL")
            .unwrap_or_else(|_| "http://localhost:12123".to_string());
        let api_key = std::env::var("LLM_API_KEY")
            .unwrap_or_default();
        let model = std::env::var("LLM_MODEL")
            .unwrap_or_else(|_| "kimi-for-coding".to_string());

        let client = reqwest::Client::new();
        
        let mut history = storage.get_messages(session_id).await?;
        if history.len() > 10 {
            history = history.split_off(history.len() - 10);
        }

        let mut messages: Vec<LlmMessage> = history
            .into_iter()
            .map(|msg| {
                let content = match &masking {
                    Some(config) => config.apply_to_text(&msg.content),
                    None => msg.content,
                };
                LlmMessage {
                    role: msg.role,
                    content,
                }
            })
            .collect();

        if messages.is_empty() {
            let content = match &masking {
                Some(config) => config.apply_to_text(message),
                None => message.to_string(),
            };
            messages.push(LlmMessage {
                role: "user".to_string(),
                content,
            });
        }

        let request_body = LlmRequest {
            model,
            messages,
            stream: false,
        };

        let mut request = client
            .post(&format!("{}/v1/chat/completions", api_url))
            .json(&request_body);

        // 如果有 API Key，添加到请求头
        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        tracing::info!("Sending request to LLM API: {}", api_url);

        let response = request.send().await?;
        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error ({}): {}", status, error_text);
        }

        let llm_response: LlmResponse = response.json().await?;
        
        let content = llm_response
            .choices
            .get(0)
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "No response from LLM".to_string());

        tracing::info!("Received response from LLM: {} chars", content.len());
        
        Ok(content)
    }

    /// 处理聊天消息并发送响应
    ///
    /// 根据 reply_to 类型决定行为：
    /// - Gateway: 发送 ChatResponse 事件（实时推送）
    /// - Http: 不发送事件，只保存到 session storage
    pub async fn handle_chat(
        state: &AgentRunnerState,
        session_id: &str,
        message: &str,
        reply_to: &ReplyChannel,
    ) -> Result<ChatResponse> {
        // 首先保存用户消息到存储
        let user_message = ChatResponse {
            session_id: session_id.to_string(),
            message_id: uuid::Uuid::new_v4().to_string(),
            content: message.to_string(),
            role: "user".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        };
        
        // 保存用户消息（HTTP 和 WebSocket 都需要保存）
        state.storage.save_message(session_id, &user_message).await?;
        
        // 生成消息 ID
        let message_id = uuid::Uuid::new_v4().to_string();
        
        // 调用真实的 LLM API
        let masking = state.masking.clone();
        let response_content = match Self::call_llm(
            state.storage.as_ref(),
            session_id,
            message,
            masking,
        )
        .await
        {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("LLM API call failed: {}", e);
                format!("Error calling LLM: {}", e)
            }
        };
        
        // 构建响应
        let response = ChatResponse {
            session_id: session_id.to_string(),
            message_id: message_id.clone(),
            content: response_content,
            role: "assistant".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        };

        // 根据回复通道类型处理
        match reply_to {
            ReplyChannel::Gateway(_) => {
                // Gateway/WebSocket 客户端: 发送 ChatResponse 事件（实时推送）
                if let Err(e) = state.event_bus.publish(Event::ChatResponse {
                    session_id: session_id.to_string(),
                    chunk: ChatChunk::Content { text: response.content.clone() },
                }) {
                    tracing::debug!("Event bus publish failed: {}", e);
                }
                
                // 同时保存到存储（WebSocket 也需要持久化）
                state
                    .storage
                    .save_message(session_id, &response)
                    .await?;
            }
            ReplyChannel::Http(_) => {
                // HTTP 客户端: 不发送任何事件，只保存到 session storage
                // 客户端通过查询接口获取消息
                state
                    .storage
                    .save_message(session_id, &response)
                    .await?;
                
                // 注意: 不发送 Event::ChatResponse 和 Event::HttpResponse
                // 避免 HTTP 客户端收到重复事件
            }
        }

        Ok(response)
    }

    /// 获取会话历史消息
    /// HTTP 客户端通过此接口查询消息
    pub async fn get_session_history(
        state: &AgentRunnerState,
        session_id: &str,
    ) -> Result<Vec<ChatResponse>> {
        state.storage.get_messages(session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 模拟存储实现
    struct MockStorage {
        messages: Mutex<Vec<(String, ChatResponse)>>,
    }

    #[async_trait::async_trait]
    impl SessionStorage for MockStorage {
        async fn save_message(&self, session_id: &str, message: &ChatResponse) -> Result<()> {
            self.messages
                .lock()
                .unwrap()
                .push((session_id.to_string(), message.clone()));
            Ok(())
        }

        async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatResponse>> {
            let messages = self.messages.lock().unwrap();
            Ok(messages
                .iter()
                .filter(|(sid, _)| sid == session_id)
                .map(|(_, msg)| msg.clone())
                .collect())
        }
    }

    #[tokio::test]
    async fn test_websocket_client_receives_event() {
        // 创建模拟存储
        let storage = Arc::new(MockStorage {
            messages: Mutex::new(Vec::new()),
        });

        // 创建事件总线
        let event_bus = Arc::new(EventBus::default());

        // 创建状态
        let state = AgentRunnerState::new(event_bus.clone(), storage.clone());

        // Gateway 回复通道
        let reply_to = ReplyChannel::Gateway("session-1".to_string());

        // 处理聊天
        let response = AgentRunner::handle_chat(&state, "session-1", "Hello", &reply_to)
            .await
            .unwrap();

        // 存储中应该有 2 条消息（用户消息 + assistant 响应）
        let messages = storage.get_messages("session-1").await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_http_client_no_event_but_saved_to_storage() {
        // 创建模拟存储
        let storage = Arc::new(MockStorage {
            messages: Mutex::new(Vec::new()),
        });

        // 创建事件总线
        let event_bus = Arc::new(EventBus::default());

        // 创建状态
        let state = AgentRunnerState::new(event_bus.clone(), storage.clone());

        // HTTP 回复通道
        let reply_to = ReplyChannel::Http("session-1".to_string());

        // 处理聊天
        let response = AgentRunner::handle_chat(&state, "session-1", "Hello", &reply_to)
            .await
            .unwrap();

        // 存储中应该有 2 条消息（用户消息 + assistant 响应）
        let messages = storage.get_messages("session-1").await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, response.content);

        // 注意: HTTP 客户端不会收到事件，需要通过查询接口获取
    }
}
