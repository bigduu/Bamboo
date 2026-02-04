//! EventBus 模块 - 用于 HTTP 和 WebSocket 之间的消息传递
//!
//! 提供统一的事件总线，让 AgentRunner 可以同时处理 HTTP 和 WebSocket 请求

use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum Event {
    /// 聊天请求
    ChatRequest {
        session_id: String,
        content: String,
        reply_to: ReplyChannel,
    },
    /// 聊天响应（流式 token）
    ChatResponse {
        session_id: String,
        chunk: ChatChunk,
    },
    /// HTTP 响应事件（用于 SSE）
    HttpResponse {
        session_id: String,
        event: Box<Event>,
    },
    /// 会话创建
    SessionCreated {
        session_id: String,
    },
    /// 会话关闭
    SessionClosed {
        session_id: String,
    },
    /// Agent 完成
    AgentComplete {
        session_id: String,
        usage: TokenUsage,
    },
    /// Agent 错误
    AgentError {
        session_id: String,
        message: String,
    },
    /// 工具开始
    ToolStart {
        session_id: String,
        tool_call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    /// 工具完成
    ToolComplete {
        session_id: String,
        tool_call_id: String,
        result: ToolResult,
    },
    /// 工具错误
    ToolError {
        session_id: String,
        tool_call_id: String,
        error: String,
    },
    /// 配置更新
    ConfigUpdated {
        sections: Vec<String>,
    },
}

/// 回复通道 - 决定响应发送到何处
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplyChannel {
    /// 发送到 Gateway (WebSocket)
    Gateway(String),
    /// 发送到 HTTP (SSE)
    Http(String),
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub result: String,
    pub display_preference: Option<String>,
}

/// 聊天块类型（流式响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatChunk {
    /// 文本内容
    Content { text: String },
    /// 开始
    Start { model: String },
    /// 完成
    Finish { reason: String },
    /// 使用统计
    Usage { input_tokens: u32, output_tokens: u32 },
    /// 错误
    Error { message: String },
}

/// EventBus 错误类型
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("Send error: {0}")]
    Send(#[from] broadcast::error::SendError<Event>),
    #[error("Channel closed")]
    ChannelClosed,
}

/// EventBus 结构体 - 使用广播通道
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    /// 创建新的 EventBus
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// 发布事件
    pub fn publish(&self, event: Event) -> Result<usize, EventBusError> {
        self.sender.send(event).map_err(EventBusError::from)
    }

    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// 获取当前订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let event_bus = EventBus::new(100);
        let mut rx = event_bus.subscribe();

        let event = Event::SessionCreated {
            session_id: "test-123".to_string(),
        };

        event_bus.publish(event.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            Event::SessionCreated { session_id } => {
                assert_eq!(session_id, "test-123");
            }
            _ => panic!("Expected SessionCreated event"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let event_bus = EventBus::new(100);
        let mut rx1 = event_bus.subscribe();
        let mut rx2 = event_bus.subscribe();

        let event = Event::ChatRequest {
            session_id: "test-123".to_string(),
            content: "Hello".to_string(),
            reply_to: ReplyChannel::Http("req-1".to_string()),
        };

        event_bus.publish(event).unwrap();

        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        match (&received1, &received2) {
            (Event::ChatRequest { session_id: s1, .. }, Event::ChatRequest { session_id: s2, .. }) => {
                assert_eq!(s1, s2);
                assert_eq!(s1, "test-123");
            }
            _ => panic!("Expected ChatRequest events"),
        }
    }
}
