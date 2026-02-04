use crate::adapters::anthropic::{
    AnthropicMessage, ContentBlock, ContentDelta, MessagesRequest, MessagesResponse,
    MessagesStreamEvent, Usage as AnthropicUsage,
};
use crate::adapters::openai::{
    ChatChoice, ChatRequest, ChatResponse, ChatStreamChunk, ChatStreamChoice, Content, ContentPart,
    Delta, FunctionCall, Message, Role, ToolCall, Usage,
};
use crate::error::ConversionError;
use serde_json::Value;

pub fn openai_to_anthropic_request(openai_req: &ChatRequest) -> Result<MessagesRequest, ConversionError> {
    let mut system_messages: Vec<String> = Vec::new();
    let mut anthropic_messages: Vec<AnthropicMessage> = Vec::new();

    for msg in &openai_req.messages {
        match msg.role {
            Role::System => {
                if let Some(content) = &msg.content {
                    if let Some(text) = content_to_string(content) {
                        system_messages.push(text);
                    }
                }
            }
            Role::User => {
                let content_blocks = convert_openai_content_to_anthropic(msg.content.as_ref())?;
                anthropic_messages.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: content_blocks,
                });
            }
            Role::Assistant => {
                let mut content_blocks = convert_openai_content_to_anthropic(msg.content.as_ref())?;
                
                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        content_blocks.push(ContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            input: serde_json::from_str(&tc.function.arguments)
                                .unwrap_or_else(|_| Value::Object(serde_json::Map::new())),
                        });
                    }
                }
                
                anthropic_messages.push(AnthropicMessage {
                    role: "assistant".to_string(),
                    content: content_blocks,
                });
            }
            Role::Tool => {
                if let Some(tool_call_id) = &msg.tool_call_id {
                    if let Some(content) = &msg.content {
                        if let Some(text) = content_to_string(content) {
                            anthropic_messages.push(AnthropicMessage {
                                role: "user".to_string(),
                                content: vec![ContentBlock::ToolResult {
                                    tool_use_id: tool_call_id.clone(),
                                    content: text,
                                }],
                            });
                        }
                    }
                }
            }
        }
    }

    let system = if system_messages.is_empty() {
        None
    } else {
        Some(system_messages.join("\n\n"))
    };

    let tools = openai_req.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|t| crate::adapters::anthropic::AnthropicToolDefinition {
                name: t.function.name.clone(),
                description: t.function.description.clone(),
                input_schema: t.function.parameters.clone().unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "object",
                        "properties": {}
                    })
                }),
            })
            .collect()
    });

    Ok(MessagesRequest {
        model: openai_req.model.clone(),
        system,
        messages: anthropic_messages,
        stream: openai_req.stream,
        temperature: openai_req.temperature,
        max_tokens: openai_req.max_tokens,
        top_p: openai_req.top_p,
        top_k: None,
        tools,
        tool_choice: openai_req.tool_choice.clone(),
        extra: std::collections::HashMap::new(),
    })
}

fn content_to_string(content: &Content) -> Option<String> {
    match content {
        Content::Text(text) => Some(text.clone()),
        Content::Parts(parts) => {
            let texts: Vec<String> = parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            if texts.is_empty() {
                None
            } else {
                Some(texts.join(""))
            }
        }
    }
}

fn convert_openai_content_to_anthropic(
    content: Option<&Content>,
) -> Result<Vec<ContentBlock>, ConversionError> {
    match content {
        None => Ok(vec![]),
        Some(Content::Text(text)) => Ok(vec![ContentBlock::Text { text: text.clone() }]),
        Some(Content::Parts(parts)) => {
            let blocks: Vec<ContentBlock> = parts
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => ContentBlock::Text { text: text.clone() },
                    ContentPart::ImageUrl { image_url } => ContentBlock::Image {
                        source: crate::adapters::anthropic::ImageSource {
                            source_type: "base64".to_string(),
                            media_type: "image/jpeg".to_string(),
                            data: image_url.url.clone(),
                        },
                    },
                })
                .collect();
            Ok(blocks)
        }
    }
}

pub fn anthropic_to_openai_response(
    anthropic_resp: &MessagesResponse,
    model: &str,
) -> Result<ChatResponse, ConversionError> {
    let mut text_content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for block in &anthropic_resp.content {
        match block {
            ContentBlock::Text { text } => {
                text_content.push_str(text);
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id: id.clone(),
                    tool_type: crate::adapters::openai::ToolType::Function,
                    function: FunctionCall {
                        name: name.clone(),
                        arguments: input.to_string(),
                    },
                });
            }
            _ => {}
        }
    }

    let message = Message {
        role: Role::Assistant,
        content: if text_content.is_empty() {
            None
        } else {
            Some(Content::Text(text_content))
        },
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        tool_call_id: None,
        extra: std::collections::HashMap::new(),
    };

    let finish_reason = anthropic_resp.stop_reason.as_ref().map(|r| {
        if r == "end_turn" {
            "stop".to_string()
        } else if r == "max_tokens" {
            "length".to_string()
        } else if r == "tool_use" {
            "tool_calls".to_string()
        } else {
            r.clone()
        }
    });

    Ok(ChatResponse {
        id: anthropic_resp.id.clone(),
        object: Some("chat.completion".to_string()),
        created: Some(chrono::Utc::now().timestamp() as u64),
        model: model.to_string(),
        choices: vec![ChatChoice {
            index: 0,
            message,
            finish_reason,
        }],
        usage: Some(Usage {
            prompt_tokens: anthropic_resp.usage.input_tokens,
            completion_tokens: anthropic_resp.usage.output_tokens,
            total_tokens: anthropic_resp.usage.input_tokens + anthropic_resp.usage.output_tokens,
        }),
    })
}

#[derive(Debug, Clone)]
pub struct StreamState {
    pub id: Option<String>,
    pub model: Option<String>,
    pub content_buffer: String,
    pub tool_calls: Vec<ToolCall>,
    pub current_tool_call: Option<ToolCallBuilder>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolCallBuilder {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}

impl StreamState {
    pub fn new() -> Self {
        Self {
            id: None,
            model: None,
            content_buffer: String::new(),
            tool_calls: Vec::new(),
            current_tool_call: None,
            finish_reason: None,
        }
    }
}

pub fn anthropic_stream_to_openai(
    event: &MessagesStreamEvent,
    state: &mut StreamState,
) -> Result<Option<ChatStreamChunk>, ConversionError> {
    match event {
        MessagesStreamEvent::MessageStart { message } => {
            state.id = Some(message.id.clone());
            state.model = Some(message.model.clone());
            Ok(None)
        }
        MessagesStreamEvent::ContentBlockStart { index: _, content_block } => {
            match content_block {
                ContentBlock::ToolUse { id, name, .. } => {
                    state.current_tool_call = Some(ToolCallBuilder {
                        id: Some(id.clone()),
                        name: Some(name.clone()),
                        arguments: String::new(),
                    });
                    Ok(None)
                }
                _ => Ok(None),
            }
        }
        MessagesStreamEvent::ContentBlockDelta { index: _, delta } => {
            match delta {
                ContentDelta::TextDelta { text } => {
                    state.content_buffer.push_str(text);
                    Ok(Some(ChatStreamChunk {
                        id: state.id.clone().unwrap_or_default(),
                        object: Some("chat.completion.chunk".to_string()),
                        created: Some(chrono::Utc::now().timestamp() as u64),
                        model: state.model.clone().unwrap_or_default(),
                        choices: vec![ChatStreamChoice {
                            index: 0,
                            delta: Delta {
                                role: Some(Role::Assistant),
                                content: Some(text.clone()),
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                        usage: None,
                    }))
                }
                ContentDelta::InputJsonDelta { partial_json } => {
                    if let Some(ref mut builder) = state.current_tool_call {
                        builder.arguments.push_str(partial_json);
                    }
                    Ok(None)
                }
            }
        }
        MessagesStreamEvent::ContentBlockStop { index: _ } => {
            if let Some(builder) = state.current_tool_call.take() {
                if let (Some(id), Some(name)) = (builder.id, builder.name) {
                    state.tool_calls.push(ToolCall {
                        id,
                        tool_type: crate::adapters::openai::ToolType::Function,
                        function: FunctionCall {
                            name,
                            arguments: builder.arguments,
                        },
                    });
                }
            }
            Ok(None)
        }
        MessagesStreamEvent::MessageDelta { delta, usage: _ } => {
            if let Some(reason) = &delta.stop_reason {
                let finish_reason = if reason == "end_turn" {
                    "stop"
                } else if reason == "max_tokens" {
                    "length"
                } else if reason == "tool_use" {
                    "tool_calls"
                } else {
                    reason.as_str()
                };
                state.finish_reason = Some(finish_reason.to_string());
            }
            Ok(None)
        }
        MessagesStreamEvent::MessageStop => {
            Ok(Some(ChatStreamChunk {
                id: state.id.clone().unwrap_or_default(),
                object: Some("chat.completion.chunk".to_string()),
                created: Some(chrono::Utc::now().timestamp() as u64),
                model: state.model.clone().unwrap_or_default(),
                choices: vec![ChatStreamChoice {
                    index: 0,
                    delta: Delta::default(),
                    finish_reason: state.finish_reason.clone(),
                }],
                usage: None,
            }))
        }
        MessagesStreamEvent::Ping => Ok(None),
        MessagesStreamEvent::Error { error } => Err(ConversionError::InvalidFormat(format!(
            "Anthropic stream error: {} - {}",
            error.error_type, error.message
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_to_anthropic_request() {
        let openai_req = ChatRequest {
            model: "claude-3-sonnet".to_string(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: Some(Content::Text("You are a helpful assistant.".to_string())),
                    tool_calls: None,
                    tool_call_id: None,
                    extra: std::collections::HashMap::new(),
                },
                Message {
                    role: Role::User,
                    content: Some(Content::Text("Hello!".to_string())),
                    tool_calls: None,
                    tool_call_id: None,
                    extra: std::collections::HashMap::new(),
                },
            ],
            stream: false,
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            extra: std::collections::HashMap::new(),
        };

        let anthropic_req = openai_to_anthropic_request(&openai_req).unwrap();
        
        assert_eq!(anthropic_req.model, "claude-3-sonnet");
        assert_eq!(anthropic_req.system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(anthropic_req.messages.len(), 1);
        assert_eq!(anthropic_req.messages[0].role, "user");
        assert_eq!(anthropic_req.temperature, Some(0.7));
        assert_eq!(anthropic_req.max_tokens, Some(1000));
    }

    #[test]
    fn test_anthropic_to_openai_response() {
        let anthropic_resp = MessagesResponse {
            id: "msg_01Xxx".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ContentBlock::Text {
                text: "Hello! How can I help you today?".to_string(),
            }],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: AnthropicUsage {
                input_tokens: 10,
                output_tokens: 20,
            },
        };

        let openai_resp = anthropic_to_openai_response(&anthropic_resp, "gpt-4").unwrap();
        
        assert_eq!(openai_resp.id, "msg_01Xxx");
        assert_eq!(openai_resp.model, "gpt-4");
        assert_eq!(openai_resp.choices.len(), 1);
        assert_eq!(openai_resp.choices[0].finish_reason, Some("stop".to_string()));
        assert!(openai_resp.usage.is_some());
        let usage = openai_resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn test_anthropic_stream_to_openai() {
        let mut state = StreamState::new();
        
        let event = MessagesStreamEvent::MessageStart {
            message: crate::adapters::anthropic::StreamMessage {
                id: "msg_01Xxx".to_string(),
                message_type: "message".to_string(),
                role: "assistant".to_string(),
                content: vec![],
                model: "claude-3-sonnet".to_string(),
                stop_reason: None,
                stop_sequence: None,
                usage: AnthropicUsage {
                    input_tokens: 10,
                    output_tokens: 0,
                },
            },
        };
        
        let result = anthropic_stream_to_openai(&event, &mut state).unwrap();
        assert!(result.is_none());
        assert_eq!(state.id, Some("msg_01Xxx".to_string()));
    }
}
