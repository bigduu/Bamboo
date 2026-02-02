//! WebSocket protocol definitions
//!
//! Defines the message types for client-gateway communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Messages sent from client to gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Initial connection request
    Connect {
        /// Optional existing session ID for reconnection
        session_id: Option<String>,
        /// Optional authentication token
        auth: Option<String>,
    },
    /// Chat message to agent
    Chat {
        /// Message content
        content: String,
        /// Target session ID
        session_id: String,
    },
    /// Command execution request
    Command {
        /// Command name
        name: String,
        /// Command arguments
        args: serde_json::Value,
    },
    /// Heartbeat ping
    Ping {
        /// Client timestamp
        timestamp: i64,
    },
}

/// Events sent from gateway to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayEvent {
    /// Connection established with session ID
    Connected {
        /// Session ID for this connection
        session_id: String,
    },
    /// Agent token/streaming response
    AgentToken {
        /// Session ID
        session_id: String,
        /// Token content
        token: String,
    },
    /// Tool execution started
    AgentToolStart {
        /// Session ID
        session_id: String,
        /// Tool name
        tool: String,
    },
    /// Tool execution completed
    AgentToolComplete {
        /// Session ID
        session_id: String,
        /// Tool name
        tool: String,
        /// Tool result
        result: String,
    },
    /// Agent response completed
    AgentComplete {
        /// Session ID
        session_id: String,
        /// Token usage statistics
        usage: TokenUsage,
    },
    /// Error response
    Error {
        /// Error code
        code: String,
        /// Error message
        message: String,
    },
    /// Heartbeat pong
    Pong {
        /// Original timestamp
        timestamp: i64,
    },
    /// Session ended
    SessionEnded {
        /// Session ID
        session_id: String,
        /// Reason for session end
        reason: String,
    },
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    /// Prompt tokens consumed
    pub prompt_tokens: u32,
    /// Completion tokens generated
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message ID
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Command structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// Command ID
    pub id: String,
    /// Command name
    pub name: String,
    /// Command arguments
    #[serde(default)]
    pub args: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}
