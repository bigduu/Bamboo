use crate::types::{Message, ToolCall};

/// Chat completion response
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    pub usage: ChatUsage,
    pub finish_reason: crate::chat::FinishReason,
}

impl ChatResponse {
    /// Create a new response
    pub fn new(
        id: impl Into<String>,
        model: impl Into<String>,
        message: Message,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            message,
            tool_calls: Vec::new(),
            usage: ChatUsage::default(),
            finish_reason: crate::chat::FinishReason::Stop,
        }
    }

    /// Add tool calls
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = tool_calls;
        self
    }

    /// Set usage
    pub fn with_usage(mut self, usage: ChatUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Set finish reason
    pub fn with_finish_reason(mut self, reason: crate::chat::FinishReason) -> Self {
        self.finish_reason = reason;
        self
    }

    /// Get the text content
    pub fn text(&self) -> String {
        self.message.text_content()
    }

    /// Check if the response has tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Token usage information
#[derive(Debug, Clone, Default)]
pub struct ChatUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl ChatUsage {
    /// Create new usage info
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }

    /// Add another usage to this one
    pub fn add(&mut self, other: &ChatUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[test]
    fn test_chat_response() {
        let message = Message::assistant("Hello!", None);
        let response = ChatResponse::new("resp_123", "gpt-4", message);
        
        assert_eq!(response.id, "resp_123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.text(), "Hello!");
    }

    #[test]
    fn test_chat_usage() {
        let usage = ChatUsage::new(10, 20);
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }
}
