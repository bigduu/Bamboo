use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 消息类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// 聊天消息 - 路由到 Agent Loop
    Chat,
    /// 命令消息 - 路由到命令处理器
    Command,
    /// 系统消息 - 路由到系统处理器
    System,
    /// 响应消息 - 从 Agent 返回给客户端
    Response,
    /// 错误消息
    Error,
    /// 心跳消息
    Heartbeat,
}

/// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// 消息元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// 消息唯一ID
    pub id: Uuid,
    /// 会话ID（用于关联同一对话的消息）
    pub session_id: String,
    /// 客户端ID
    pub client_id: String,
    /// 消息类型
    pub kind: MessageKind,
    /// 优先级
    #[serde(default)]
    pub priority: Priority,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 源地址/通道
    pub source: String,
    /// 目标地址/通道（可选，用于定向路由）
    pub target: Option<String>,
    /// 扩展属性
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl MessageMetadata {
    pub fn new(kind: MessageKind, session_id: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: session_id.into(),
            client_id: client_id.into(),
            kind,
            priority: Priority::default(),
            created_at: Utc::now(),
            source: String::new(),
            target: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }
}

/// 核心消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息元数据
    #[serde(flatten)]
    pub metadata: MessageMetadata,
    /// 消息负载（具体内容）
    pub payload: MessagePayload,
}

impl Message {
    pub fn new(metadata: MessageMetadata, payload: MessagePayload) -> Self {
        Self { metadata, payload }
    }

    /// 快速创建聊天消息
    pub fn chat(session_id: impl Into<String>, client_id: impl Into<String>, content: impl Into<String>) -> Self {
        let metadata = MessageMetadata::new(MessageKind::Chat, session_id, client_id);
        let payload = MessagePayload::Chat(ChatPayload {
            content: content.into(),
            context: Vec::new(),
            attachments: Vec::new(),
        });
        Self::new(metadata, payload)
    }

    /// 快速创建命令消息
    pub fn command(session_id: impl Into<String>, client_id: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        let metadata = MessageMetadata::new(MessageKind::Command, session_id, client_id);
        let payload = MessagePayload::Command(CommandPayload {
            command: command.into(),
            args,
            options: HashMap::new(),
        });
        Self::new(metadata, payload)
    }

    /// 快速创建系统消息
    pub fn system(session_id: impl Into<String>, client_id: impl Into<String>, action: impl Into<String>) -> Self {
        let metadata = MessageMetadata::new(MessageKind::System, session_id, client_id);
        let payload = MessagePayload::System(SystemPayload {
            action: action.into(),
            params: HashMap::new(),
        });
        Self::new(metadata, payload)
    }

    /// 创建响应消息
    pub fn response(source: &Message, content: impl Into<String>) -> Self {
        let metadata = MessageMetadata::new(
            MessageKind::Response,
            &source.metadata.session_id,
            &source.metadata.client_id,
        )
        .with_target(&source.metadata.source);
        
        let payload = MessagePayload::Response(ResponsePayload {
            request_id: source.metadata.id,
            content: content.into(),
            status: ResponseStatus::Success,
        });
        
        Self::new(metadata, payload)
    }

    /// 创建错误响应
    pub fn error(source: &Message, error: impl Into<String>) -> Self {
        let metadata = MessageMetadata::new(
            MessageKind::Error,
            &source.metadata.session_id,
            &source.metadata.client_id,
        )
        .with_target(&source.metadata.source);
        
        let payload = MessagePayload::Response(ResponsePayload {
            request_id: source.metadata.id,
            content: error.into(),
            status: ResponseStatus::Error,
        });
        
        Self::new(metadata, payload)
    }

    pub fn kind(&self) -> &MessageKind {
        &self.metadata.kind
    }

    pub fn session_id(&self) -> &str {
        &self.metadata.session_id
    }

    pub fn client_id(&self) -> &str {
        &self.metadata.client_id
    }
}

/// 消息负载枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum MessagePayload {
    Chat(ChatPayload),
    Command(CommandPayload),
    System(SystemPayload),
    Response(ResponsePayload),
    Error(ErrorPayload),
    Heartbeat,
}

/// 聊天消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPayload {
    pub content: String,
    #[serde(default)]
    pub context: Vec<ContextMessage>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

/// 上下文消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// 附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Uuid,
    pub name: String,
    pub mime_type: String,
    pub size: usize,
    pub url: Option<String>,
}

/// 命令消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

/// 系统消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPayload {
    pub action: String,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

/// 响应状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Error,
    Pending,
    Cancelled,
}

/// 响应消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePayload {
    pub request_id: Uuid,
    pub content: String,
    pub status: ResponseStatus,
}

/// 错误消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::chat("session-1", "client-1", "Hello, world!");
        assert_eq!(msg.kind(), &MessageKind::Chat);
        assert_eq!(msg.session_id(), "session-1");
        assert_eq!(msg.client_id(), "client-1");
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::chat("session-1", "client-1", "Hello!");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.metadata.id, deserialized.metadata.id);
    }
}
