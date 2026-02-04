use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::agent_runner::ChatResponse;

/// 会话存储接口
#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save_message(&self, session_id: &str, message: &ChatResponse) -> Result<()>;
    async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatResponse>>;
}

/// 内存会话存储（用于开发/测试）
#[derive(Debug, Default)]
pub struct MemorySessionStorage {
    messages: RwLock<HashMap<String, Vec<ChatResponse>>>,
}

impl MemorySessionStorage {
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn save_message(&self, session_id: &str, message: &ChatResponse) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages
            .entry(session_id.to_string())
            .or_default()
            .push(message.clone());
        Ok(())
    }

    async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatResponse>> {
        let messages = self.messages.read().await;
        Ok(messages.get(session_id).cloned().unwrap_or_default())
    }
}
