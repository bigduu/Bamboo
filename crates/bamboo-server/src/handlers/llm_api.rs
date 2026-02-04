//! OpenAI-compatible LLM API Handler
//!
//! Provides endpoints compatible with OpenAI API format:
//! - POST /v1/chat/completions - Chat completions (streaming and non-streaming)
//! - GET /v1/models - List available models
//! - GET /v1/models/{model} - Get model information

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_web::http::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use futures_util::StreamExt;

use crate::state::AppState;
use crate::middleware::validate_api_key;

/// OpenAI-compatible chat completion request
#[derive(Debug, Deserialize)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(default)]
    pub tools: Option<Vec<OpenAiTool>>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(flatten)]
    pub extra: Value,
}

/// OpenAI-compatible message
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// OpenAI-compatible tool
#[derive(Debug, Deserialize)]
pub struct OpenAiTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAiFunction,
}

/// OpenAI-compatible function definition
#[derive(Debug, Deserialize)]
pub struct OpenAiFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Value,
}

/// OpenAI-compatible tool call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAiFunctionCall,
}

/// OpenAI-compatible function call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAiFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Serialize)]
pub struct OpenAiChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    pub usage: OpenAiUsage,
}

/// OpenAI-compatible choice
#[derive(Debug, Serialize)]
pub struct OpenAiChoice {
    pub index: u32,
    pub message: OpenAiMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// OpenAI-compatible streaming choice
#[derive(Debug, Serialize)]
pub struct OpenAiStreamChoice {
    pub index: u32,
    pub delta: OpenAiDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// OpenAI-compatible delta for streaming
#[derive(Debug, Serialize, Default)]
pub struct OpenAiDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// OpenAI-compatible usage
#[derive(Debug, Serialize, Default)]
pub struct OpenAiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI-compatible model
#[derive(Debug, Serialize)]
pub struct OpenAiModel {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

/// OpenAI-compatible models list response
#[derive(Debug, Serialize)]
pub struct OpenAiModelsResponse {
    pub object: String,
    pub data: Vec<OpenAiModel>,
}

/// Convert OpenAI message to bamboo_core Message
fn convert_message(msg: &OpenAiMessage) -> bamboo_core::types::Message {
    let role = match msg.role.as_str() {
        "system" => bamboo_core::types::Role::System,
        "assistant" => bamboo_core::types::Role::Assistant,
        "tool" => bamboo_core::types::Role::Tool,
        _ => bamboo_core::types::Role::User,
    };
    
    let content = if let Some(text) = msg.content.as_str() {
        bamboo_core::types::Content::Text { text: text.to_string() }
    } else {
        // Handle complex content (array of parts)
        bamboo_core::types::Content::Text { text: msg.content.to_string() }
    };
    
    let tool_calls = msg.tool_calls.as_ref().map(|tcs| {
        tcs.iter().map(|tc| {
            bamboo_core::types::ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments: serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| serde_json::json!({})),
            }
        }).collect()
    });
    
    bamboo_core::types::Message {
        id: Uuid::new_v4().to_string(),
        role,
        content,
        tool_calls,
        tool_call_id: msg.tool_call_id.clone(),
        metadata: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
    }
}

/// Convert bamboo_core Message to OpenAI message
fn convert_to_openai_message(msg: &bamboo_core::types::Message) -> OpenAiMessage {
    let role = msg.role.to_string().to_lowercase();
    let content = msg.text_content();
    
    let tool_calls = msg.tool_calls.as_ref().map(|tcs| {
        tcs.iter().map(|tc| {
            OpenAiToolCall {
                id: tc.id.clone(),
                call_type: "function".to_string(),
                function: OpenAiFunctionCall {
                    name: tc.name.clone(),
                    arguments: tc.arguments.to_string(),
                },
            }
        }).collect()
    });
    
    OpenAiMessage {
        role,
        content: Value::String(content),
        tool_calls,
        tool_call_id: msg.tool_call_id.clone(),
    }
}

/// POST /v1/chat/completions
/// 
/// OpenAI-compatible chat completion endpoint
/// Supports both streaming and non-streaming responses
pub async fn chat_completions(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<OpenAiChatRequest>,
) -> impl Responder {
    // Validate API key
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    if let Err(e) = validate_api_key(&state, auth_header).await {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": {
                "message": e,
                "type": "authentication_error",
                "code": "invalid_api_key"
            }
        }));
    }
    
    let request = body.into_inner();
    let is_streaming = request.stream.unwrap_or(false);
    
    // Convert messages
    let messages: Vec<bamboo_core::types::Message> = request.messages.iter()
        .map(convert_message)
        .collect();
    
    // Convert tools if provided
    let tools: Vec<bamboo_core::types::ToolDefinition> = request.tools.as_ref()
        .map(|t| t.iter().map(|tool| {
            bamboo_core::types::ToolDefinition {
                name: tool.function.name.clone(),
                description: tool.function.description.clone().unwrap_or_default(),
                parameters: tool.function.parameters.clone(),
            }
        }).collect())
        .unwrap_or_default();
    
    // Build chat request
    let chat_request = bamboo_core::chat::ChatRequest::new(&request.model)
        .with_messages(messages)
        .with_tools(tools);
    
    let chat_request = if let Some(temp) = request.temperature {
        chat_request.temperature(temp)
    } else {
        chat_request
    };
    
    let chat_request = if let Some(max_tokens) = request.max_tokens {
        chat_request.max_tokens(max_tokens)
    } else {
        chat_request
    };
    
    let chat_request = if is_streaming {
        chat_request.stream()
    } else {
        chat_request
    };
    
    if is_streaming {
        // Handle streaming response
        handle_streaming_response(state, chat_request, request.model).await
    } else {
        // Handle non-streaming response
        handle_non_streaming_response(state, chat_request, request.model).await
    }
}

/// Handle non-streaming chat completion
async fn handle_non_streaming_response(
    state: web::Data<AppState>,
    chat_request: bamboo_core::chat::ChatRequest,
    model: String,
) -> HttpResponse {
    match state.llm.chat(chat_request).await {
        Ok(response) => {
            let openai_response = OpenAiChatResponse {
                id: response.id.clone(),
                object: "chat.completion".to_string(),
                created: chrono::Utc::now().timestamp(),
                model: response.model.clone(),
                choices: vec![
                    OpenAiChoice {
                        index: 0,
                        message: convert_to_openai_message(&response.message),
                        finish_reason: Some(response.finish_reason.as_str().to_string()),
                    }
                ],
                usage: OpenAiUsage {
                    prompt_tokens: response.usage.input_tokens,
                    completion_tokens: response.usage.output_tokens,
                    total_tokens: response.usage.total_tokens,
                },
            };
            
            HttpResponse::Ok().json(openai_response)
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": {
                    "message": format!("LLM error: {}", e),
                    "type": "api_error",
                    "code": "internal_error"
                }
            }))
        }
    }
}

/// Handle streaming chat completion
async fn handle_streaming_response(
    state: web::Data<AppState>,
    chat_request: bamboo_core::chat::ChatRequest,
    model: String,
) -> HttpResponse {
    let llm = state.llm.clone();
    
    let stream = async_stream::stream! {
        let request_id = format!("chatcmpl-{}", Uuid::new_v4().to_string().replace("-", ""));
        let created = chrono::Utc::now().timestamp();
        
        // Send initial role delta
        let initial_chunk = serde_json::json!({
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant"
                },
                "finish_reason": null
            }]
        });
        
        yield Ok::<_, actix_web::Error>(
            actix_web::web::Bytes::from(format!("data: {}\n\n", initial_chunk.to_string()))
        );
        
        match llm.chat_stream(chat_request).await {
            Ok(mut stream) => {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            match chunk {
                                bamboo_core::chat::ChatChunk::Content { text } => {
                                    let chunk_json = serde_json::json!({
                                        "id": request_id,
                                        "object": "chat.completion.chunk",
                                        "created": created,
                                        "model": model,
                                        "choices": [{
                                            "index": 0,
                                            "delta": {
                                                "content": text
                                            },
                                            "finish_reason": null
                                        }]
                                    });
                                    
                                    yield Ok(
                                        actix_web::web::Bytes::from(format!("data: {}\n\n", chunk_json.to_string()))
                                    );
                                }
                                bamboo_core::chat::ChatChunk::Finish { reason } => {
                                    let finish_reason = reason.as_str().to_string();
                                    let chunk_json = serde_json::json!({
                                        "id": request_id,
                                        "object": "chat.completion.chunk",
                                        "created": created,
                                        "model": model,
                                        "choices": [{
                                            "index": 0,
                                            "delta": {},
                                            "finish_reason": finish_reason
                                        }]
                                    });
                                    
                                    yield Ok(
                                        actix_web::web::Bytes::from(format!("data: {}\n\n", chunk_json.to_string()))
                                    );
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            let error_json = serde_json::json!({
                                "error": {
                                    "message": format!("Stream error: {}", e),
                                    "type": "api_error"
                                }
                            });
                            yield Ok(
                                actix_web::web::Bytes::from(format!("data: {}\n\n", error_json.to_string()))
                            );
                            break;
                        }
                    }
                }
                
                // Send [DONE] marker
                yield Ok(actix_web::web::Bytes::from("data: [DONE]\n\n"));
            }
            Err(e) => {
                let error_json = serde_json::json!({
                    "error": {
                        "message": format!("Failed to start stream: {}", e),
                        "type": "api_error"
                    }
                });
                yield Ok(
                    actix_web::web::Bytes::from(format!("data: {}\n\n", error_json.to_string()))
                );
            }
        }
    };
    
    HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "text/event-stream"))
        .append_header((header::CACHE_CONTROL, "no-cache"))
        .append_header((header::CONNECTION, "keep-alive"))
        .streaming(stream)
}

/// GET /v1/models
///
/// List available models from configured providers
pub async fn list_models(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Validate API key
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    if let Err(e) = validate_api_key(&state, auth_header).await {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": {
                "message": e,
                "type": "authentication_error",
                "code": "invalid_api_key"
            }
        }));
    }
    
    let config = state.config.get().read().await.clone();
    let mut models = Vec::new();
    
    // Add models from configured providers
    for (provider_id, provider_settings) in &config.llm.providers {
        if !provider_settings.enabled {
            continue;
        }
        
        let model_id = provider_settings.model.as_ref()
            .map(|m| format!("{}/{}", provider_id, m))
            .unwrap_or_else(|| provider_id.clone());
        
        models.push(OpenAiModel {
            id: model_id,
            object: "model".to_string(),
            created: chrono::Utc::now().timestamp(),
            owned_by: provider_id.clone(),
        });
    }
    
    // If no models configured, add a default
    if models.is_empty() {
        models.push(OpenAiModel {
            id: "default".to_string(),
            object: "model".to_string(),
            created: chrono::Utc::now().timestamp(),
            owned_by: "bamboo".to_string(),
        });
    }
    
    HttpResponse::Ok().json(OpenAiModelsResponse {
        object: "list".to_string(),
        data: models,
    })
}

/// GET /v1/models/{model}
///
/// Get information about a specific model
pub async fn get_model(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    // Validate API key
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    if let Err(e) = validate_api_key(&state, auth_header).await {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": {
                "message": e,
                "type": "authentication_error",
                "code": "invalid_api_key"
            }
        }));
    }
    
    let model_id = path.into_inner();
    let config = state.config.get().read().await.clone();
    
    // Find the model in configured providers
    for (provider_id, provider_settings) in &config.llm.providers {
        if !provider_settings.enabled {
            continue;
        }
        
        let provider_model_id = provider_settings.model.as_ref()
            .map(|m| format!("{}/{}", provider_id, m))
            .unwrap_or_else(|| provider_id.clone());
        
        if provider_model_id == model_id {
            return HttpResponse::Ok().json(OpenAiModel {
                id: model_id,
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: provider_id.clone(),
            });
        }
    }
    
    HttpResponse::NotFound().json(serde_json::json!({
        "error": {
            "message": format!("Model '{}' not found", model_id),
            "type": "invalid_request_error",
            "code": "model_not_found"
        }
    }))
}
