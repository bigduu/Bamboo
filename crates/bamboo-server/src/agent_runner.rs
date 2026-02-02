use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use futures::StreamExt;

use bamboo_core::{Session, Message, AgentEvent, AgentError, chat::ChatRequest, types::ToolDefinition};
use bamboo_core::agent::events::TokenUsage;
use bamboo_core::tools::ToolExecutor;
use bamboo_core::types::ToolCall;
use bamboo_llm::LLMProvider;
use bamboo_llm::transformer::LLMStream;
use crate::logging::{DebugLogger, Timer};
use crate::state::AppState;
use crate::event_bus::{Event, ReplyChannel, ChatChunk, TokenUsage as EventTokenUsage, ToolResult};

pub type Result<T> = std::result::Result<T, AgentError>;

/// Configuration for agent loop
pub struct AgentLoopConfig {
    pub max_rounds: usize,
    pub system_prompt: Option<String>,
    pub additional_tool_schemas: Vec<bamboo_core::tools::ToolSchema>,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            system_prompt: None,
            additional_tool_schemas: Vec::new(),
        }
    }
}

/// Convert old ToolSchema to new ToolDefinition
fn convert_schemas_to_definitions(schemas: &[bamboo_core::tools::ToolSchema]) -> Vec<ToolDefinition> {
    schemas.iter().map(|s| {
        ToolDefinition::new(
            s.function.name.clone(),
            s.function.description.clone(),
            s.function.parameters.clone(),
        )
    }).collect()
}

/// AgentRunner - 基于 EventBus 的 Agent 运行器
pub struct AgentRunner {
    state: Arc<AppState>,
}

impl AgentRunner {
    /// 创建新的 AgentRunner
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// 运行 AgentRunner - 订阅 EventBus 并处理事件
    pub async fn run(&self) {
        let mut rx = self.state.event_bus.subscribe();
        
        log::info!("AgentRunner started, waiting for events...");

        while let Ok(event) = rx.recv().await {
            match event {
                Event::ChatRequest { session_id, content, reply_to } => {
                    log::debug!("Received ChatRequest for session: {}", session_id);
                    
                    // 克隆 state 以便在 spawn 中使用
                    let state = self.state.clone();
                    
                    // 在后台任务中处理聊天请求
                    tokio::spawn(async move {
                        if let Err(e) = handle_chat_request(&state, session_id, content, reply_to).await {
                            log::error!("Failed to handle chat request: {}", e);
                        }
                    });
                }
                Event::SessionCreated { session_id } => {
                    log::debug!("Session created: {}", session_id);
                }
                Event::SessionClosed { session_id } => {
                    log::debug!("Session closed: {}", session_id);
                    // 取消正在进行的任务
                    let mut tokens = self.state.cancel_tokens.write().await;
                    if let Some(token) = tokens.remove(&session_id) {
                        token.cancel();
                    }
                }
                _ => {}
            }
        }

        log::warn!("AgentRunner event loop ended");
    }
}

/// 处理聊天请求
async fn handle_chat_request(
    state: &AppState,
    session_id: String,
    content: String,
    reply_to: ReplyChannel,
) -> Result<()> {
    let debug_logger = DebugLogger::new(log::log_enabled!(log::Level::Debug));
    
    log::debug!("[{}] Handling chat request: {}", session_id, content);
    debug_logger.log_event(&session_id, "chat_request_start", serde_json::json!({
        "content": content,
        "reply_to": format!("{:?}", reply_to),
    }));

    // 获取或创建会话
    let mut session = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(s) => {
                log::debug!("[{}] Found existing session with {} messages", 
                    session_id, s.messages.len());
                s.clone()
            }
            None => {
                log::info!("[{}] Creating new session", session_id);
                let new_session = Session::new(session_id.clone());
                drop(sessions);
                let mut sessions = state.sessions.write().await;
                sessions.insert(session_id.clone(), new_session.clone());
                new_session
            }
        }
    };

    // 添加用户消息
    session.add_message(Message::user(content.clone()));
    
    // 保存会话
    state.save_session(&session).await;

    // 创建取消令牌
    let cancel_token = CancellationToken::new();
    {
        let mut tokens = state.cancel_tokens.write().await;
        tokens.insert(session_id.clone(), cancel_token.clone());
    }

    // 构建系统提示
    let system_prompt = state.build_system_prompt(
        "You are a helpful AI assistant with access to various tools and skills."
    );
    
    // 获取所有工具 schemas
    let all_tool_schemas = state.get_all_tool_schemas();
    
    // 创建工具执行器（在 run_agent_loop_with_config 内部使用）
    let _tools = state.create_tool_executor();

    // 运行 Agent Loop
    let result = run_agent_loop_with_config(
        &mut session,
        content,
        state,
        &reply_to,
        cancel_token,
        AgentLoopConfig {
            max_rounds: 3,
            system_prompt: Some(system_prompt),
            additional_tool_schemas: all_tool_schemas,
        },
    ).await;

    // 移除取消令牌
    {
        let mut tokens = state.cancel_tokens.write().await;
        tokens.remove(&session_id);
    }

    // 保存会话
    state.save_session(&session).await;
    
    // 更新内存中的会话
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id.clone(), session);
    }

    if let Err(ref e) = result {
        log::error!("[{}] Agent loop error: {}", session_id, e);
        // 发送错误事件
        let error_event = Event::AgentError {
            session_id: session_id.clone(),
            message: e.to_string(),
        };
        if let Err(e) = state.event_bus.publish(error_event) {
            log::error!("Failed to publish error event: {}", e);
        }
    }

    result
}

/// Run agent loop with EventBus integration
async fn run_agent_loop_with_config(
    session: &mut Session,
    initial_message: String,
    state: &AppState,
    reply_to: &ReplyChannel,
    cancel_token: CancellationToken,
    config: AgentLoopConfig,
) -> Result<()> {
    let debug_logger = DebugLogger::new(log::log_enabled!(log::Level::Debug));
    let session_id = session.id.clone();
    
    log::debug!("[{}] Starting agent loop with message: {}", session_id, initial_message);
    debug_logger.log_event(&session_id, "agent_loop_start", serde_json::json!({
        "message": initial_message,
        "max_rounds": config.max_rounds,
        "initial_message_count": session.messages.len(),
    }));

    // Add system message if provided
    if let Some(ref system_prompt) = config.system_prompt {
        if !session.messages.iter().any(|m| matches!(m.role, bamboo_core::agent::types::Role::System)) {
            session.messages.insert(0, Message::system(system_prompt.clone()));
            log::debug!("[{}] Added system prompt", session_id);
        }
    }

    // 1. Add user message (already added in handle_chat_request)
    
    // 2. Loop for max_rounds
    for round in 0..config.max_rounds {
        log::debug!("[{}] Starting round {}/{}", session_id, round + 1, config.max_rounds);
        debug_logger.log_event(&session_id, "round_start", serde_json::json!({
            "round": round + 1,
            "total_rounds": config.max_rounds,
            "message_count": session.messages.len(),
        }));

        // Check cancellation
        if cancel_token.is_cancelled() {
            log::debug!("[{}] Agent loop cancelled", session_id);
            return Err(AgentError::Cancelled);
        }

        // Get tools list
        let mut tool_schemas = state.create_tool_executor().list_tools();
        tool_schemas.extend(config.additional_tool_schemas.clone());
        log::debug!("[{}] Available tools: {} (base: {}, from skills: {})", 
            session_id, 
            tool_schemas.len(),
            state.create_tool_executor().list_tools().len(),
            config.additional_tool_schemas.len()
        );

        // Convert schemas to tool definitions
        let tool_definitions = convert_schemas_to_definitions(&tool_schemas);

        // Build ChatRequest
        let model = "copilot-chat".to_string(); // Default model, could be passed via config
        let chat_request = ChatRequest::new(model)
            .with_messages(session.messages.clone())
            .with_tools(tool_definitions)
            .stream();

        // Call LLM (streaming)
        let timer = Timer::new("llm_request");
        let mut stream: LLMStream = match state.llm
            .chat_stream(chat_request)
            .await
        {
            Ok(s) => {
                log::debug!("[{}] LLM stream created successfully", session_id);
                s
            }
            Err(e) => {
                log::error!("[{}] Failed to create LLM stream: {}", session_id, e);
                return Err(AgentError::LLM(e.to_string()));
            }
        };

        let mut accumulated_content = String::new();
        let mut current_tool_call_parts: Vec<PartialToolCall> = Vec::new();
        let mut token_count = 0;
        let mut stream_finished = false;
        let mut usage = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        // Process streaming response
        while let Some(chunk_result) = stream.next().await {
            // Check cancellation
            if cancel_token.is_cancelled() {
                log::debug!("[{}] Stream cancelled", session_id);
                return Err(AgentError::Cancelled);
            }

            match chunk_result {
                Ok(bamboo_core::chat::ChatChunk::Content { text }) => {
                    accumulated_content.push_str(&text);
                    token_count += text.len();
                    
                    // Debug info every 10 tokens
                    if token_count % 10 == 0 {
                        log::debug!("[{}] Received {} tokens so far", session_id, token_count);
                    }
                    
                    // Send token event via EventBus
                    let event = Event::ChatResponse {
                        session_id: session_id.clone(),
                        chunk: ChatChunk::Content { text: text.clone() },
                    };
                    if let Err(e) = state.event_bus.publish(event) {
                        log::error!("Failed to publish chat response: {}", e);
                    }
                    
                    // 发送事件到客户端（根据 reply_to 决定是 Gateway 还是 HTTP）
                    state.send_event(&session_id, Event::ChatResponse {
                        session_id: session_id.clone(),
                        chunk: ChatChunk::Content { text },
                    }).await;
                }
                Ok(bamboo_core::chat::ChatChunk::ToolCallStart { call_id, name }) => {
                    log::debug!("[{}] Tool call start: {} (id: {})", session_id, name, call_id);
                    current_tool_call_parts.push(PartialToolCall {
                        id: call_id,
                        name,
                        arguments: String::new(),
                    });
                }
                Ok(bamboo_core::chat::ChatChunk::ToolCallDelta { call_id, arguments_delta }) => {
                    // Find and update the tool call
                    if let Some(part) = current_tool_call_parts.iter_mut().find(|p| p.id == call_id) {
                        part.arguments.push_str(&arguments_delta);
                    }
                }
                Ok(bamboo_core::chat::ChatChunk::ToolCallEnd { call_id: _ }) => {
                    log::debug!("[{}] Tool call end", session_id);
                }
                Ok(bamboo_core::chat::ChatChunk::Finish { reason: _ }) => {
                    log::debug!("[{}] Stream finished", session_id);
                    stream_finished = true;
                    break;
                }
                Ok(bamboo_core::chat::ChatChunk::Usage { input_tokens, output_tokens }) => {
                    log::debug!("[{}] Usage: input={}, output={}", session_id, input_tokens, output_tokens);
                    usage.prompt_tokens = input_tokens;
                    usage.completion_tokens = output_tokens;
                    usage.total_tokens = input_tokens + output_tokens;
                }
                Ok(bamboo_core::chat::ChatChunk::Start { model }) => {
                    log::debug!("[{}] Stream started with model: {}", session_id, model);
                }
                Ok(bamboo_core::chat::ChatChunk::Error { message }) => {
                    log::error!("[{}] Stream error: {}", session_id, message);
                    let error_event = Event::AgentError {
                        session_id: session_id.clone(),
                        message: format!("Stream error: {}", message),
                    };
                    if let Err(e) = state.event_bus.publish(error_event.clone()) {
                        log::error!("Failed to publish error event: {}", e);
                    }
                    state.send_event(&session_id, error_event).await;
                    return Err(AgentError::LLM(message));
                }
                Err(e) => {
                    log::error!("[{}] Stream error: {}", session_id, e);
                    let error_event = Event::AgentError {
                        session_id: session_id.clone(),
                        message: format!("Stream error: {}", e),
                    };
                    if let Err(e) = state.event_bus.publish(error_event.clone()) {
                        log::error!("Failed to publish error event: {}", e);
                    }
                    state.send_event(&session_id, error_event).await;
                    return Err(AgentError::LLM(e.to_string()));
                }
            }
        }

        let llm_duration = timer.elapsed_ms();
        timer.debug(&session_id);
        log::debug!("[{}] LLM response completed in {}ms, {} tokens received", 
            session_id, llm_duration, token_count);

        // Convert partial tool calls to full tool calls
        let accumulated_tool_calls = finalize_tool_calls(current_tool_call_parts);
        log::debug!("[{}] Finalized {} tool calls", session_id, accumulated_tool_calls.len());

        // If no tool calls, send Complete event and finish
        if accumulated_tool_calls.is_empty() && stream_finished {
            log::debug!("[{}] No tool calls, completing", session_id);
            
            // Add assistant message to history
            session.add_message(Message::assistant(
                accumulated_content.clone(),
                None,
            ));

            log::debug!("[{}] Added assistant message, content length: {}", 
                session_id, accumulated_content.len());

            // Send complete event via EventBus
            let complete_event = Event::AgentComplete {
                session_id: session_id.clone(),
                usage: EventTokenUsage {
                    prompt_tokens: usage.prompt_tokens,
                    completion_tokens: token_count as u32,
                    total_tokens: usage.prompt_tokens + token_count as u32,
                },
            };
            if let Err(e) = state.event_bus.publish(complete_event.clone()) {
                log::error!("Failed to publish complete event: {}", e);
            }
            state.send_event(&session_id, complete_event).await;

            debug_logger.log_event(&session_id, "agent_loop_complete", serde_json::json!({
                "rounds": round + 1,
                "total_tokens": token_count,
                "final_message_count": session.messages.len(),
            }));

            break;
        }

        // Have tool calls, continue loop
        log::debug!("[{}] Processing {} tool calls", session_id, accumulated_tool_calls.len());
        
        // Add assistant message (with tool calls)
        session.add_message(Message::assistant(
            accumulated_content.clone(),
            Some(accumulated_tool_calls.clone()),
        ));

        // Execute tools
        for (idx, tool_call) in accumulated_tool_calls.iter().enumerate() {
            log::debug!("[{}] Executing tool {}/{}: {}", 
                session_id, idx + 1, accumulated_tool_calls.len(), tool_call.name);
            
            // Parse arguments
            let args: serde_json::Value = serde_json::from_str(&tool_call.arguments_string())
                .unwrap_or_else(|_| serde_json::json!({}));
            
            log::debug!("[{}] Tool {} arguments: {}", session_id, tool_call.name, args);
            
            debug_logger.log_event(&session_id, "tool_start", serde_json::json!({
                "tool_name": tool_call.name.clone(),
                "tool_call_id": tool_call.id.clone(),
                "arguments": args.clone(),
            }));
            
            // Send ToolStart event
            let tool_start_event = Event::ToolStart {
                session_id: session_id.clone(),
                tool_call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                arguments: args.clone(),
            };
            if let Err(e) = state.event_bus.publish(tool_start_event.clone()) {
                log::error!("Failed to publish tool start event: {}", e);
            }
            state.send_event(&session_id, tool_start_event).await;

            let tool_timer = Timer::new(&format!("tool_{}", tool_call.name));
            
            // Convert new ToolCall to old ToolCall for execution
            let old_tool_call = bamboo_core::tools::ToolCall {
                id: tool_call.id.clone(),
                tool_type: "function".to_string(),
                function: bamboo_core::tools::FunctionCall {
                    name: tool_call.name.clone(),
                    arguments: tool_call.arguments.to_string(),
                },
            };
            
            // Execute tool
            match state.create_tool_executor().execute(&old_tool_call).await {
                Ok(result) => {
                    let tool_duration = tool_timer.elapsed_ms();
                    log::debug!("[{}] Tool {} completed in {}ms, success: {}, result length: {}",
                        session_id, tool_call.name, tool_duration, 
                        result.success, result.result.len());
                    
                    debug_logger.log_event(&session_id, "tool_complete", serde_json::json!({
                        "tool_name": tool_call.name.clone(),
                        "tool_call_id": tool_call.id.clone(),
                        "duration_ms": tool_duration,
                        "success": result.success,
                        "result_preview": &result.result[..result.result.len().min(100)],
                    }));
                    
                    // Send ToolComplete event
                    let tool_complete_event = Event::ToolComplete {
                        session_id: session_id.clone(),
                        tool_call_id: tool_call.id.clone(),
                        result: ToolResult {
                            success: result.success,
                            result: result.result.clone(),
                            display_preference: result.display_preference.clone(),
                        },
                    };
                    if let Err(e) = state.event_bus.publish(tool_complete_event.clone()) {
                        log::error!("Failed to publish tool complete event: {}", e);
                    }
                    state.send_event(&session_id, tool_complete_event).await;

                    // Add tool result to history
                    session.add_message(Message::tool_result(
                        tool_call.id.clone(),
                        result.result.clone(),
                    ));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::error!("[{}] Tool {} failed: {}", session_id, tool_call.name, error_msg);
                    
                    debug_logger.log_event(&session_id, "tool_error", serde_json::json!({
                        "tool_name": tool_call.name.clone(),
                        "tool_call_id": tool_call.id.clone(),
                        "error": error_msg.clone(),
                    }));
                    
                    // Send ToolError event
                    let tool_error_event = Event::ToolError {
                        session_id: session_id.clone(),
                        tool_call_id: tool_call.id.clone(),
                        error: error_msg.clone(),
                    };
                    if let Err(e) = state.event_bus.publish(tool_error_event.clone()) {
                        log::error!("Failed to publish tool error event: {}", e);
                    }
                    state.send_event(&session_id, tool_error_event).await;

                    // Add error result to history
                    session.add_message(Message::tool_result(
                        tool_call.id.clone(),
                        format!("Error: {}", error_msg),
                    ));
                }
            }
        }

        log::debug!("[{}] Round {} completed", session_id, round + 1);
        debug_logger.log_event(&session_id, "round_complete", serde_json::json!({
            "round": round + 1,
            "message_count": session.messages.len(),
        }));

        // Continue to next round
    }

    log::debug!("[{}] Agent loop completed, final message count: {}", 
        session_id, session.messages.len());
    
    Ok(())
}

// Partial tool call accumulation structure
#[derive(Clone)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

fn finalize_tool_calls(parts: Vec<PartialToolCall>) -> Vec<ToolCall> {
    parts
        .into_iter()
        .map(|p| {
            log::debug!("Finalizing tool call: {} (args length: {})", p.name, p.arguments.len());
            // Parse arguments as JSON
            let args = serde_json::from_str(&p.arguments).unwrap_or_else(|_| serde_json::json!({}));
            ToolCall::new(p.id, p.name, args)
        })
        .collect()
}

/// Backward-compatible wrapper for run_agent_loop
pub async fn run_agent_loop(
    _session: &mut Session,
    _initial_message: String,
    _event_tx: mpsc::Sender<AgentEvent>,
    _llm: Arc<dyn LLMProvider>,
    _tools: Arc<dyn ToolExecutor>,
    _cancel_token: CancellationToken,
    _max_rounds: usize,
) -> Result<()> {
    // 新的实现使用 EventBus，此函数为兼容性保留
    log::warn!("run_agent_loop is deprecated, use EventBus-based AgentRunner instead");
    Ok(())
}

/// Backward-compatible wrapper for run_agent_loop_with_config (legacy API)
pub async fn run_agent_loop_with_config_legacy(
    _session: &mut Session,
    _initial_message: String,
    _event_tx: mpsc::Sender<AgentEvent>,
    _llm: Arc<dyn LLMProvider>,
    _tools: Arc<dyn ToolExecutor>,
    _cancel_token: CancellationToken,
    _config: AgentLoopConfig,
) -> Result<()> {
    // 新的实现使用 EventBus，此函数为兼容性保留
    log::warn!("run_agent_loop_with_config_legacy is deprecated, use EventBus-based AgentRunner instead");
    Ok(())
}
