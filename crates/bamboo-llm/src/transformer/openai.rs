use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatChunk, ChatResponse, ChatUsage, FinishReason};
use bamboo_core::types::{Content, ContentPart, Message, Role, ToolCall};
use serde_json::{json, Value};

use crate::error::ConversionError;
use crate::transformer::SchemaTransformer;

/// OpenAI-compatible schema transformer
/// Works with OpenAI API, Azure OpenAI, and compatible providers
pub struct OpenAiTransformer;

impl OpenAiTransformer {
    /// Create a new OpenAI transformer
    pub fn new() -> Self {
        Self
    }

    /// Convert internal Message to OpenAI format
    fn convert_message(&self, msg: &Message) -> Result<Value, ConversionError> {
        let mut json = json!({
            "role": msg.role.to_string().to_lowercase(),
        });

        // Handle content
        match &msg.content {
            Content::Text { text } => {
                json["content"] = json!(text);
            }
            Content::Parts { parts } => {
                let content_parts: Vec<Value> = parts
                    .iter()
                    .map(|p| self.convert_content_part(p))
                    .collect::<Result<Vec<_>, _>>()?;
                json["content"] = json!(content_parts);
            }
        }

        // Add tool_calls for assistant messages
        if let Some(tool_calls) = &msg.tool_calls {
            json["tool_calls"] = json!(tool_calls.iter().map(|tc| {
                json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments.to_string(),
                    }
                })
            }).collect::<Vec<_>>());
        }

        // Add tool_call_id for tool messages
        if let Some(tool_call_id) = &msg.tool_call_id {
            json["tool_call_id"] = json!(tool_call_id);
        }

        Ok(json)
    }

    /// Convert content part to OpenAI format
    fn convert_content_part(&self, part: &ContentPart) -> Result<Value, ConversionError> {
        match part {
            ContentPart::Text { text } => {
                Ok(json!({
                    "type": "text",
                    "text": text,
                }))
            }
            ContentPart::Image { source } => {
                match source {
                    bamboo_core::types::ImageSource::Base64 { data, mime_type } => {
                        Ok(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64, {}", mime_type, data),
                            }
                        }))
                    }
                    bamboo_core::types::ImageSource::Url { url } => {
                        Ok(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": url,
                            }
                        }))
                    }
                }
            }
        }
    }

    /// Convert finish reason string to enum
    fn convert_finish_reason(&self, reason: Option<&str>) -> FinishReason {
        match reason {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        }
    }
}

impl Default for OpenAiTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SchemaTransformer for OpenAiTransformer {
    fn provider_id(&self) -> &str {
        "openai"
    }

    fn transform_request(&self, request: &ChatRequest) -> Result<Value, ConversionError> {
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect::<Result<Vec<_>, _>>()?;

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": request.options.stream,
        });

        // Add tools if present
        if !request.tools.is_empty() {
            body["tools"] = self.transform_tools(&request.tools)?;
        }

        // Add optional parameters
        if let Some(temp) = request.options.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(max_tokens) = request.options.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        if let Some(top_p) = request.options.top_p {
            body["top_p"] = json!(top_p);
        }

        // Handle response format
        if let Some(format) = &request.options.response_format {
            match format {
                bamboo_core::chat::ResponseFormat::Text => {}
                bamboo_core::chat::ResponseFormat::JsonObject => {
                    body["response_format"] = json!({ "type": "json_object" });
                }
                bamboo_core::chat::ResponseFormat::JsonSchema { schema } => {
                    body["response_format"] = json!({
                        "type": "json_schema",
                        "json_schema": {
                            "schema": schema,
                        }
                    });
                }
            }
        }

        Ok(body)
    }

    fn parse_stream_chunk(&self, data: &str) -> Result<Option<ChatChunk>, ConversionError> {
        // Handle SSE [DONE] marker
        if data == "[DONE]" {
            return Ok(Some(ChatChunk::Finish { reason: FinishReason::Stop }));
        }

        let chunk: Value = serde_json::from_str(data)?;

        // Get first choice
        let choice = chunk["choices"].get(0);

        // Handle content delta
        if let Some(content) = choice.and_then(|c| c["delta"]["content"].as_str()) {
            if !content.is_empty() {
                return Ok(Some(ChatChunk::Content { text: content.to_string() }));
            }
        }

        // Handle tool calls
        if let Some(tool_calls) = choice.and_then(|c| c["delta"]["tool_calls"].as_array()) {
            if let Some(tc) = tool_calls.get(0) {
                let call_id = tc["id"].as_str().unwrap_or_default().to_string();
                let name = tc["function"]["name"].as_str().map(|s| s.to_string());
                let arguments = tc["function"]["arguments"].as_str().unwrap_or_default().to_string();

                // If we have a name, this is a new tool call
                if let Some(name) = name {
                    return Ok(Some(ChatChunk::ToolCallStart { call_id, name }));
                }

                // Otherwise, it's a delta
                if !arguments.is_empty() {
                    return Ok(Some(ChatChunk::ToolCallDelta { call_id, arguments_delta: arguments }));
                }
            }
        }

        // Handle finish reason
        if let Some(reason) = choice.and_then(|c| c["finish_reason"].as_str()) {
            return Ok(Some(ChatChunk::Finish { 
                reason: self.convert_finish_reason(Some(reason)) 
            }));
        }

        // Handle usage information
        if let Some(usage) = chunk.get("usage") {
            let input = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
            let output = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
            return Ok(Some(ChatChunk::Usage { input_tokens: input, output_tokens: output }));
        }

        Ok(None)
    }

    fn transform_tools(&self, tools: &[bamboo_core::types::ToolDefinition]) -> Result<Value, ConversionError> {
        let tools_json: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();

        Ok(json!(tools_json))
    }

    fn parse_response(&self, data: &Value) -> Result<ChatResponse, ConversionError> {
        let id = data["id"].as_str().unwrap_or_default().to_string();
        let model = data["model"].as_str().unwrap_or_default().to_string();

        let choice = data["choices"]
            .get(0)
            .ok_or_else(|| ConversionError::MissingField("choices".to_string()))?;

        let message_data = choice["message"].clone();
        let role = message_data["role"]
            .as_str()
            .map(|r| match r {
                "system" => Role::System,
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "tool" => Role::Tool,
                _ => Role::Assistant,
            })
            .unwrap_or(Role::Assistant);

        let content = message_data["content"]
            .as_str()
            .map(|s| Content::Text { text: s.to_string() })
            .unwrap_or_else(|| Content::Text { text: String::new() });

        let tool_calls: Vec<ToolCall> = message_data["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        Some(ToolCall {
                            id: tc["id"].as_str()?.to_string(),
                            name: tc["function"]["name"].as_str()?.to_string(),
                            arguments: tc["function"]["arguments"]
                                .as_str()
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or_else(|| json!({})),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage_data = data.get("usage");
        let usage = ChatUsage {
            input_tokens: usage_data
                .and_then(|u| u["prompt_tokens"].as_u64())
                .unwrap_or(0) as u32,
            output_tokens: usage_data
                .and_then(|u| u["completion_tokens"].as_u64())
                .unwrap_or(0) as u32,
            total_tokens: usage_data
                .and_then(|u| u["total_tokens"].as_u64())
                .unwrap_or(0) as u32,
        };

        let finish_reason = self.convert_finish_reason(choice["finish_reason"].as_str());

        let message = Message {
            id: format!("msg_{}", id),
            role,
            content,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls.clone()) },
            tool_call_id: None,
            metadata: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        Ok(ChatResponse {
            id,
            model,
            message,
            tool_calls,
            usage,
            finish_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bamboo_core::types::ToolDefinition;

    #[test]
    fn test_transform_request() {
        let transformer = OpenAiTransformer::new();
        let request = ChatRequest::new("gpt-4")
            .with_message(Message::user("Hello"))
            .temperature(0.7);

        let body = transformer.transform_request(&request).unwrap();
        assert_eq!(body["model"], "gpt-4");
        // Use approximate comparison for floating point
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001, "temperature should be approximately 0.7, got {}", temp);
        assert!(body["messages"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_transform_tools() {
        let transformer = OpenAiTransformer::new();
        let tools = vec![ToolDefinition::new(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {}
            }),
        )];

        let result = transformer.transform_tools(&tools).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["function"]["name"], "test_tool");
    }

    #[test]
    fn test_parse_stream_chunk() {
        let transformer = OpenAiTransformer::new();
        
        // Test content chunk
        let chunk = r#"{"choices":[{"delta":{"content":"Hello"}}]}"#;
        let result = transformer.parse_stream_chunk(chunk).unwrap();
        match result {
            Some(ChatChunk::Content { text }) => assert_eq!(text, "Hello"),
            _ => panic!("Expected content chunk"),
        }

        // Test finish chunk
        let finish = r#"{"choices":[{"finish_reason":"stop"}]}"#;
        let result = transformer.parse_stream_chunk(finish).unwrap();
        match result {
            Some(ChatChunk::Finish { reason }) => assert_eq!(reason, FinishReason::Stop),
            _ => panic!("Expected finish chunk"),
        }
    }
}
