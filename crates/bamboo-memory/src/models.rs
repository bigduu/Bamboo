use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Memory {
    pub id: String,
    pub session_id: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl Memory {
    pub fn new(session_id: impl Into<String>, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            content: content.into(),
            tags: Vec::new(),
            created_at: now,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionMemory {
    pub session_id: String,
    #[serde(default)]
    pub memories: Vec<Memory>,
    pub updated_at: String,
}

impl SessionMemory {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            memories: Vec::new(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
