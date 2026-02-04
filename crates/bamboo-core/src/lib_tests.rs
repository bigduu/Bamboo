#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Session, Message, Role};
    use crate::types::tool::{ToolCall, ToolResult, ToolDefinition};

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-123");
        assert_eq!(session.id, "test-123");
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.text(), Some("Hello"));
        assert!(matches!(msg.role, Role::User));
        assert!(!msg.id.is_empty());
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new("test");
        let msg = Message::user("Test message");
        session.add_message(msg);
        
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].text(), Some("Test message"));
    }

    #[test]
    fn test_tool_call_creation() {
        let args = serde_json::json!({"key": "value"});
        let tool_call = ToolCall::new("call-1", "test_tool", args);
        
        assert_eq!(tool_call.id, "call-1");
        assert_eq!(tool_call.name, "test_tool");
    }

    #[test]
    fn test_tool_result_creation() {
        let result = ToolResult::success("Success output");
        
        assert!(result.is_success());
        assert_eq!(result.content, "Success output");
    }

    #[test]
    fn test_tool_definition_creation() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {}
        });
        let schema = ToolDefinition::new("test", "Test tool", params);
        
        assert_eq!(schema.name, "test");
        assert_eq!(schema.description, "Test tool");
    }

    #[test]
    fn test_assistant_message_with_tool_calls() {
        let tool_calls = vec![
            ToolCall::new("call-1", "get_weather", serde_json::json!({"city": "Beijing"}))
        ];
        
        let msg = Message::assistant("", Some(tool_calls));
        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.tool_calls.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_tool_result_message() {
        let msg = Message::tool_result("call-1", "Sunny, 25°C");
        assert!(matches!(msg.role, Role::Tool));
        assert_eq!(msg.tool_call_id, Some("call-1".to_string()));
        assert_eq!(msg.text(), Some("Sunny, 25°C"));
    }
}
