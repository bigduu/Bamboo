use serde_json::Value;
use crate::types::{Message, ToolDefinition};

/// Chat completion request
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub options: ChatOptions,
}

impl ChatRequest {
    /// Create a new chat request
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            tools: Vec::new(),
            options: ChatOptions::default(),
        }
    }

    /// Add a message to the request
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Add a tool definition
    pub fn with_tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tool definitions
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set options
    pub fn with_options(mut self, options: ChatOptions) -> Self {
        self.options = options;
        self
    }

    /// Enable streaming
    pub fn stream(mut self) -> Self {
        self.options.stream = true;
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.options.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.options.max_tokens = Some(max);
        self
    }
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            options: ChatOptions::default(),
        }
    }
}

/// Options for chat completion
#[derive(Debug, Clone)]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stream: bool,
    pub response_format: Option<ResponseFormat>,
}

impl ChatOptions {
    /// Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set temperature (0.0 - 2.0)
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Set top_p (0.0 - 1.0)
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Set response format
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: false,
            response_format: None,
        }
    }
}

/// Response format for structured outputs
#[derive(Debug, Clone)]
pub enum ResponseFormat {
    /// Standard text response
    Text,
    /// JSON object response
    JsonObject,
    /// JSON with specific schema
    JsonSchema { schema: Value },
}

impl ResponseFormat {
    /// Create JSON object format
    pub fn json_object() -> Self {
        Self::JsonObject
    }

    /// Create JSON schema format
    pub fn json_schema(schema: Value) -> Self {
        Self::JsonSchema { schema }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new("gpt-4")
            .with_message(Message::user("Hello"))
            .temperature(0.7)
            .max_tokens(100);

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.options.temperature, Some(0.7));
        assert_eq!(request.options.max_tokens, Some(100));
    }

    #[test]
    fn test_chat_options() {
        let options = ChatOptions::new()
            .with_temperature(0.5)
            .with_max_tokens(200)
            .with_streaming();

        assert_eq!(options.temperature, Some(0.5));
        assert_eq!(options.max_tokens, Some(200));
        assert!(options.stream);
    }
}