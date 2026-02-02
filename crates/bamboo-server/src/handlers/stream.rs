use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_web::http::header;
use tokio::sync::mpsc;
use std::time::Instant;

use crate::state::AppState;
use crate::event_bus::{Event, ReplyChannel};
use crate::logging::DebugLogger;

/// HTTP SSE 流处理器
/// 
/// 通过 EventBus 订阅响应事件，转换为 SSE 返回
pub async fn handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
    _req: HttpRequest,
) -> impl Responder {
    let session_id = path.into_inner();
    let start_time = Instant::now();
    let _debug_logger = DebugLogger::new(log::log_enabled!(log::Level::Debug));
    
    log::debug!("[{}] SSE stream request received", session_id);

    // 获取或创建会话
    let session = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(s) => {
                log::debug!("[{}] Found existing session with {} messages", 
                    session_id, s.messages.len());
                s.clone()
            }
            None => {
                log::warn!("[{}] Session not found", session_id);
                return HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Session not found"
                }));
            }
        }
    };

    // 获取最后一条用户消息
    let initial_message = session.messages.last()
        .filter(|m| matches!(m.role, bamboo_core::agent::types::Role::User))
        .map(|m| m.text_content())
        .unwrap_or_default();

    if initial_message.is_empty() {
        log::warn!("[{}] No user message found for streaming", session_id);
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No user message found"
        }));
    }

    // 创建 SSE 流通道
    let (sse_tx, mut sse_rx) = mpsc::channel::<actix_web::web::Bytes>(100);
    
    // 订阅 EventBus
    let mut event_rx = state.event_bus.subscribe();
    let session_id_clone = session_id.clone();
    
    // 启动事件处理器任务 - 将 EventBus 事件转换为 SSE
    let event_processor = tokio::spawn(async move {
        let mut event_count = 0;
        let mut token_count = 0;
        let mut completed = false;
        
        while let Ok(event) = event_rx.recv().await {
            // 只处理当前 session 的事件
            let event_session_id = match &event {
                Event::ChatResponse { session_id, .. } => session_id.clone(),
                Event::AgentComplete { session_id, .. } => session_id.clone(),
                Event::AgentError { session_id, .. } => session_id.clone(),
                Event::ToolStart { session_id, .. } => session_id.clone(),
                Event::ToolComplete { session_id, .. } => session_id.clone(),
                Event::ToolError { session_id, .. } => session_id.clone(),
                Event::HttpResponse { session_id, .. } => session_id.clone(),
                _ => continue,
            };
            
            if event_session_id != session_id_clone {
                continue;
            }
            
            event_count += 1;
            
            // 将事件转换为 AgentEvent 然后序列化为 SSE
            let agent_event = match event {
                Event::ChatResponse { chunk, .. } => {
                    match chunk {
                        crate::event_bus::ChatChunk::Content { text } => {
                            token_count += text.len();
                            Some(bamboo_core::AgentEvent::Token { content: text })
                        }
                        _ => None,
                    }
                }
                Event::AgentComplete { usage, .. } => {
                    completed = true;
                    Some(bamboo_core::AgentEvent::Complete { 
                        usage: bamboo_core::agent::events::TokenUsage {
                            prompt_tokens: usage.prompt_tokens,
                            completion_tokens: usage.completion_tokens,
                            total_tokens: usage.total_tokens,
                        }
                    })
                }
                Event::AgentError { message, .. } => {
                    completed = true;
                    Some(bamboo_core::AgentEvent::Error { message })
                }
                Event::ToolStart { tool_call_id, tool_name, arguments, .. } => {
                    Some(bamboo_core::AgentEvent::ToolStart { 
                        tool_call_id, 
                        tool_name, 
                        arguments 
                    })
                }
                Event::ToolComplete { tool_call_id, result, .. } => {
                    Some(bamboo_core::AgentEvent::ToolComplete { 
                        tool_call_id, 
                        result: bamboo_core::tools::ToolResult {
                            success: result.success,
                            result: result.result,
                            display_preference: result.display_preference,
                        }
                    })
                }
                Event::ToolError { tool_call_id, error, .. } => {
                    Some(bamboo_core::AgentEvent::ToolError { 
                        tool_call_id, 
                        error 
                    })
                }
                Event::HttpResponse { event, .. } => {
                    // 递归处理 HttpResponse 事件
                    match *event {
                        Event::ChatResponse { chunk, .. } => {
                            match chunk {
                                crate::event_bus::ChatChunk::Content { text } => {
                                    token_count += text.len();
                                    Some(bamboo_core::AgentEvent::Token { content: text })
                                }
                                _ => None,
                            }
                        }
                        Event::AgentComplete { usage, .. } => {
                            completed = true;
                            Some(bamboo_core::AgentEvent::Complete { 
                                usage: bamboo_core::agent::events::TokenUsage {
                                    prompt_tokens: usage.prompt_tokens,
                                    completion_tokens: usage.completion_tokens,
                                    total_tokens: usage.total_tokens,
                                }
                            })
                        }
                        Event::AgentError { message, .. } => {
                            completed = true;
                            Some(bamboo_core::AgentEvent::Error { message })
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            
            if let Some(agent_event) = agent_event {
                let event_json = match serde_json::to_string(&agent_event) {
                    Ok(json) => json,
                    Err(e) => {
                        log::error!("[{}] Failed to serialize event: {}", session_id_clone, e);
                        continue;
                    }
                };
                
                let sse_data = format!("data: {}\n\n", event_json);
                let bytes = actix_web::web::Bytes::from(sse_data);
                
                if sse_tx.send(bytes).await.is_err() {
                    log::debug!("[{}] SSE channel closed", session_id_clone);
                    break;
                }
                
                // 如果是 Complete 或 Error 事件，结束流
                match &agent_event {
                    bamboo_core::AgentEvent::Complete { .. } | 
                    bamboo_core::AgentEvent::Error { .. } => {
                        completed = true;
                        break;
                    }
                    _ => {}
                }
            }
            
            if completed {
                break;
            }
        }
        
        (event_count, token_count, completed)
    });

    // 发布 ChatRequest 事件到 EventBus 触发 AgentRunner
    let chat_request = Event::ChatRequest {
        session_id: session_id.clone(),
        content: initial_message,
        reply_to: ReplyChannel::Http(session_id.clone()),
    };
    
    if let Err(e) = state.event_bus.publish(chat_request) {
        log::error!("[{}] Failed to publish chat request: {}", session_id, e);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to start chat"
        }));
    }

    // 等待事件处理器完成并记录
    let session_id_for_stats = session_id.clone();
    tokio::spawn(async move {
        match event_processor.await {
            Ok((event_count, token_count, completed)) => {
                let duration = start_time.elapsed();
                log::debug!("[{}] Stream completed: {} events, {} tokens, completed={}, {:?} elapsed",
                    session_id_for_stats, event_count, token_count, completed, duration);
            }
            Err(e) => {
                log::error!("[{}] Event processor task failed: {}", session_id_for_stats, e);
            }
        }
    });

    // 返回 SSE 响应
    log::debug!("[{}] Returning SSE response", session_id);
    HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, "text/event-stream"))
        .append_header((header::CACHE_CONTROL, "no-cache"))
        .append_header((header::CONNECTION, "keep-alive"))
        .streaming(async_stream::stream! {
            while let Some(item) = sse_rx.recv().await {
                yield Ok::<_, actix_web::Error>(item);
            }
            log::debug!("[{}] SSE stream closed", session_id);
        })
}
