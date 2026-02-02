use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool call from the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }

    /// Get arguments as a JSON string
    pub fn arguments_string(&self) -> String {
        self.arguments.to_string()
    }

    /// Parse arguments to a specific type
    pub fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.arguments.clone())
    }
}

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,  // JSON Schema
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    /// Create a simple tool with no parameters
    pub fn simple(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            success: false,
            content: msg.clone(),
            error: Some(msg),
        }
    }

    /// Check if the result is successful
    pub fn is_success(&self) -> bool {
        self.success
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call() {
        let args = serde_json::json!({"key": "value"});
        let call = ToolCall::new("call_123", "my_tool", args.clone());
        assert_eq!(call.id, "call_123");
        assert_eq!(call.name, "my_tool");
        assert_eq!(call.arguments, args);
    }

    #[test]
    fn test_tool_definition() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let def = ToolDefinition::new("greet", "Greet someone", params);
        assert_eq!(def.name, "greet");
        assert_eq!(def.description, "Greet someone");
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("Done!");
        assert!(result.is_success());
        assert_eq!(result.content, "Done!");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Failed!");
        assert!(!result.is_success());
        assert_eq!(result.content, "Failed!");
        assert_eq!(result.error, Some("Failed!".to_string()));
    }
}
