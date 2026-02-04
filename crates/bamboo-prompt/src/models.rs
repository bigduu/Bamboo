use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemPrompt {
    pub id: String,
    pub name: String,
    pub content: String,
    pub is_default: bool,
    pub is_custom: bool,
    pub category: String,
}

impl SystemPrompt {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            content: content.into(),
            is_default: false,
            is_custom: true,
            category: category.into(),
        }
    }

    pub fn default_prompt() -> Self {
        Self {
            id: String::new(),
            name: "Default".to_string(),
            content: "You are a helpful assistant.".to_string(),
            is_default: true,
            is_custom: false,
            category: "general".to_string(),
        }
    }
}
