//! Message routing
//!
//! Routes incoming messages to appropriate handlers.

use crate::protocol::{ChatMessage, ClientMessage, Command, GatewayEvent};
use crate::session::{Session, SessionHandle};
use async_trait::async_trait;
use std::sync::Arc;

/// Result of routing a message
#[derive(Debug, Clone)]
pub enum RouteResult {
    /// Route to agent for processing
    ToAgent(SessionHandle, ChatMessage),
    /// Route to a channel
    ToChannel(String, String), // channel_id, message
    /// Immediate response to client
    Response(GatewayEvent),
    /// No action needed
    None,
}

/// Incoming message wrapper
#[derive(Debug, Clone)]
pub enum IncomingMessage {
    /// Chat message
    Chat(ChatMessage),
    /// Command execution
    Command(Command),
    /// Ping heartbeat
    Ping,
}

impl From<ClientMessage> for IncomingMessage {
    fn from(msg: ClientMessage) -> Self {
        match msg {
            ClientMessage::Chat {
                content,
                session_id,
            } => IncomingMessage::Chat(ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                session_id,
                content,
                timestamp: chrono::Utc::now(),
                metadata: serde_json::Value::Null,
            }),
            ClientMessage::Command { name, args } => IncomingMessage::Command(Command {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                args,
                timestamp: chrono::Utc::now(),
            }),
            ClientMessage::Ping { .. } => IncomingMessage::Ping,
            _ => IncomingMessage::Ping, // Connect handled separately
        }
    }
}

/// Routes messages to appropriate handlers
#[derive(Debug, Clone)]
pub struct MessageRouter;

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self
    }

    /// Route an incoming message
    pub async fn route(
        &self,
        msg: IncomingMessage,
        session: &SessionHandle,
    ) -> RouteResult {
        match msg {
            IncomingMessage::Chat(chat_msg) => {
                self.handle_chat(session, chat_msg).await
            }
            IncomingMessage::Command(cmd) => {
                self.handle_command(session, cmd).await
            }
            IncomingMessage::Ping => {
                RouteResult::Response(GatewayEvent::Pong {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                })
            }
        }
    }

    /// Handle a chat message
    pub async fn handle_chat(
        &self,
        session: &SessionHandle,
        msg: ChatMessage,
    ) -> RouteResult {
        // Update session activity
        session.write().await.touch();

        // Route to agent for processing
        RouteResult::ToAgent(Arc::clone(session), msg)
    }

    /// Handle a command
    pub async fn handle_command(
        &self,
        session: &SessionHandle,
        cmd: Command,
    ) -> RouteResult {
        // Update session activity
        session.write().await.touch();

        // Handle built-in commands
        match cmd.name.as_str() {
            "ping" => RouteResult::Response(GatewayEvent::Pong {
                timestamp: chrono::Utc::now().timestamp_millis(),
            }),
            "status" => {
                let session = session.read().await;
                RouteResult::Response(GatewayEvent::AgentToken {
                    session_id: session.id.clone(),
                    token: format!("Session: {}, Connected: {}", session.id, session.is_connected()),
                })
            }
            _ => {
                // Route other commands to agent
                RouteResult::Response(GatewayEvent::AgentToolStart {
                    session_id: session.read().await.id.clone(),
                    tool: cmd.name,
                })
            }
        }
    }

    /// Broadcast an event to all sessions
    pub async fn broadcast(&self, _event: GatewayEvent) {
        // Implementation would go through ConnectionPool
        // This is a placeholder for router-specific broadcast logic
    }
}

/// Trait for message handlers
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle a chat message
    async fn handle_chat(&self, session: &Session, msg: &ChatMessage) -> GatewayEvent;

    /// Handle a command
    async fn handle_command(&self, session: &Session, cmd: &Command) -> GatewayEvent;
}
