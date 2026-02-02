use crate::MessageHandler;
use crate::message::{Message, MessageKind, MessagePayload};
use crate::router::{MessageBus, Topics};
use crate::Result;
use async_trait::async_trait;
use tracing::{debug, error, info, instrument, warn};

/// 聊天处理器 - 将消息路由到 Agent Loop
pub struct ChatHandler {
    name: String,
}

impl ChatHandler {
    pub fn new() -> Self {
        Self {
            name: "ChatHandler".to_string(),
        }
    }

    /// 预处理聊天消息
    fn preprocess(&self, msg: &Message) -> Message {
        let mut processed = msg.clone();
        
        // 可以在这里添加：
        // - 敏感词过滤
        // - 消息格式化
        // - 上下文注入
        
        processed
    }
}

impl Default for ChatHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageHandler for ChatHandler {
    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip(self, bus), fields(msg_id = %msg.metadata.id))]
    async fn handle(&self, msg: Message, bus: &MessageBus) -> Result<Option<Message>> {
        info!("Processing chat message for session: {}", msg.session_id());

        // 预处理
        let processed = self.preprocess(&msg);

        // 发送到 Agent Loop
        bus.publish(Topics::agent_input(), processed).await?;

        // 异步处理，不立即返回响应
        Ok(None)
    }

    fn can_handle(&self, kind: &MessageKind) -> bool {
        matches!(kind, MessageKind::Chat)
    }
}

/// Agent Loop 处理器 - 模拟 Agent 处理并返回响应
pub struct AgentLoopHandler {
    name: String,
}

impl AgentLoopHandler {
    pub fn new() -> Self {
        Self {
            name: "AgentLoopHandler".to_string(),
        }
    }

    /// 模拟 Agent 处理
    async fn process(&self, msg: &Message) -> String {
        // 模拟处理延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        match &msg.payload {
            MessagePayload::Chat(chat) => {
                format!("Agent received: {}", chat.content)
            }
            _ => "Unknown payload".to_string(),
        }
    }
}

impl Default for AgentLoopHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageHandler for AgentLoopHandler {
    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip(self, bus), fields(msg_id = %msg.metadata.id))]
    async fn handle(&self, msg: Message, bus: &MessageBus) -> Result<Option<Message>> {
        debug!("Agent Loop processing message");

        // 处理消息
        let response_content = self.process(&msg).await;

        // 创建响应
        let response = Message::response(&msg, response_content);

        // 发送回 Gateway
        bus.publish(Topics::agent_output(), response.clone()).await?;

        Ok(Some(response))
    }

    fn can_handle(&self, kind: &MessageKind) -> bool {
        // Agent Loop 处理发送到 agent:input 的消息
        matches!(kind, MessageKind::Chat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_handler() {
        let handler = ChatHandler::new();
        let bus = MessageBus::new();

        let msg = Message::chat("session-1", "client-1", "Hello!");
        let result = handler.handle(msg, &bus).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_agent_loop_handler() {
        let handler = AgentLoopHandler::new();
        let bus = MessageBus::new();

        let msg = Message::chat("session-1", "client-1", "Hello Agent!");
        let result = handler.handle(msg, &bus).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }
}
