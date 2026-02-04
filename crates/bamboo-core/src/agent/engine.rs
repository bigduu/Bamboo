use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use crate::agent::{Session, Message, AgentEvent, events::TokenUsage};
use crate::tools::ToolExecutor;
use crate::types::{Content, ContentPart};
use bamboo_masking::MaskingConfig;
use thiserror::Error;
use std::sync::Arc;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("LLM error: {0}")]
    LLM(String),
    
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("Cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, AgentError>;

pub struct AgentConfig {
    pub max_rounds: usize,
    pub model: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            model: "gpt-4o-mini".to_string(),
        }
    }
}

pub struct AgentLoop<E: ToolExecutor> {
    config: AgentConfig,
    tool_executor: E,
    masking: Option<Arc<RwLock<MaskingConfig>>>,
}

impl<E: ToolExecutor> AgentLoop<E> {
    pub fn new(config: AgentConfig, tool_executor: E) -> Self {
        Self {
            config,
            tool_executor,
            masking: None,
        }
    }

    pub fn with_masking(
        config: AgentConfig,
        tool_executor: E,
        masking: Option<Arc<RwLock<MaskingConfig>>>,
    ) -> Self {
        Self {
            config,
            tool_executor,
            masking,
        }
    }

    pub fn set_masking(&mut self, masking: Option<Arc<RwLock<MaskingConfig>>>) {
        self.masking = masking;
    }

    pub async fn run(
        &self,
        session: &mut Session,
        initial_message: String,
        event_tx: mpsc::Sender<AgentEvent>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        // 添加用户消息
        session.add_message(Message::user(initial_message));

        let _masked_messages = self.mask_messages(&session.messages).await;
        
        for _round in 0..self.config.max_rounds {
            // 检查取消
            if cancel_token.is_cancelled() {
                return Err(AgentError::Cancelled);
            }

            // TODO: Phase 2 实现 LLM 调用
            // TODO: Phase 3 实现工具执行
            
            // 临时：直接发送完成事件
            event_tx.send(AgentEvent::Complete {
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 10,
                    total_tokens: 20,
                },
            }).await.map_err(|e| AgentError::LLM(e.to_string()))?;
            
            break;
        }

        Ok(())
    }

    async fn mask_messages(&self, messages: &[Message]) -> Vec<Message> {
        let masking = match &self.masking {
            Some(masking) => masking.read().await,
            None => return messages.to_vec(),
        };

        messages
            .iter()
            .map(|message| apply_masking_to_message(&masking, message))
            .collect()
    }
}

fn apply_masking_to_message(config: &MaskingConfig, message: &Message) -> Message {
    let mut masked = message.clone();
    let content = match &message.content {
        Content::Text { text } => Content::Text {
            text: config.apply_to_text(text),
        },
        Content::Parts { parts } => {
            let masked_parts = parts
                .iter()
                .map(|part| match part {
                    ContentPart::Text { text } => ContentPart::Text {
                        text: config.apply_to_text(text),
                    },
                    other => other.clone(),
                })
                .collect();
            Content::Parts { parts: masked_parts }
        }
    };

    masked.content = content;
    masked
}
