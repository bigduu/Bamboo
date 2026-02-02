use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;
use uuid::Uuid;

use crate::types::content::Content;
use crate::types::tool::ToolCall;

/// Unique message identifier
pub type MessageId = String;

/// Message role in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// Core message type for LLM conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub role: Role,
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::System,
            content: Content::Text { text: content.into() },
            tool_calls: None,
            tool_call_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::User,
            content: Content::Text { text: content.into() },
            tool_calls: None,
            tool_call_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create an assistant message with optional tool calls
    pub fn assistant(content: impl Into<String>, tool_calls: Option<Vec<ToolCall>>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: Content::Text { text: content.into() },
            tool_calls,
            tool_call_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a tool result message
    pub fn tool_result(call_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Tool,
            content: Content::Text { text: result.into() },
            tool_calls: None,
            tool_call_id: Some(call_id.into()),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a message from parts (multimodal content)
    pub fn from_parts(role: Role, parts: Vec<crate::types::content::ContentPart>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content: Content::Parts { parts },
            tool_calls: None,
            tool_call_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Get text content if available
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            Content::Text { text } => Some(text),
            Content::Parts { parts } => {
                // Try to get text from the first text part
                parts.iter().find_map(|p| match p {
                    crate::types::content::ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
            }
        }
    }

    /// Get all text content concatenated
    pub fn text_content(&self) -> String {
        match &self.content {
            Content::Text { text } => text.clone(),
            Content::Parts { parts } => {
                parts.iter()
                    .filter_map(|p| match p {
                        crate::types::content::ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
        }
    }

    /// Check if this message contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.as_ref().map(|tc| !tc.is_empty()).unwrap_or(false)
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::User,
            content: Content::Text { text: String::new() },
            tool_calls: None,
            tool_call_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_message() {
        let msg = Message::system("You are a helpful assistant");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.text(), Some("You are a helpful assistant"));
    }

    #[test]
    fn test_user_message() {
        let msg = Message::user("Hello!");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.text(), Some("Hello!"));
    }

    #[test]
    fn test_assistant_message() {
        let msg = Message::assistant("Hello!", None);
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.text(), Some("Hello!"));
    }

    #[test]
    fn test_tool_result() {
        let msg = Message::tool_result("call_123", "Tool result");
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
    }
}
