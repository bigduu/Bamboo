//! HTTP Server - 提供 REST API 和 WebSocket 支持
//!
//! 修复: 通过共享 AppState 确保 HTTP handlers 和 AgentRunner 使用同一个 event_bus

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::{
    AgentRunner, AgentRunnerState, ChatResponse, Event, ReplyChannel,
};

/// 应用状态 - 在 main.rs 中创建并共享给所有组件
#[derive(Clone)]
pub struct AppState {
    /// Agent 运行器状态
    pub agent_state: Arc<AgentRunnerState>,
    /// 服务器配置
    pub config: ServerConfig,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(agent_state: Arc<AgentRunnerState>, config: ServerConfig) -> Self {
        Self {
            agent_state,
            config,
        }
    }
}

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// LLM API 基础 URL
    pub llm_api_url: String,
    /// LLM 模型名称
    pub llm_model: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            llm_api_url: "http://localhost:12123".to_string(),
            llm_model: "kimi-for-coding".to_string(),
        }
    }
}

impl ServerConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("BAMBOO_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("BAMBOO_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            llm_api_url: std::env::var("LLM_API_URL")
                .unwrap_or_else(|_| "http://localhost:12123".to_string()),
            llm_model: std::env::var("LLM_MODEL")
                .unwrap_or_else(|_| "kimi-for-coding".to_string()),
        }
    }
}

/// 创建会话请求
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// 创建会话响应
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub created_at: String,
}

/// 发送消息请求
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

/// 发送消息响应
#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
    pub role: String,
    pub timestamp: String,
}

/// 会话历史响应
#[derive(Debug, Serialize)]
pub struct SessionHistoryResponse {
    pub session_id: String,
    pub messages: Vec<ChatResponse>,
}

/// 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// 运行 HTTP 服务器
/// 
/// 修复: 现在接受 AppState 参数而不是创建新实例
/// 这确保了 HTTP handlers 和 AgentRunner 使用同一个 event_bus
pub async fn run_server(state: AppState) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("{}:{}", state.config.host, state.config.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!(
        "Bamboo server starting on http://{}",
        addr
    );

    axum::serve(listener, app).await?;
    
    Ok(())
}

/// 创建路由
fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);

    Router::new()
        // 健康检查
        .route("/health", get(health_handler))
        // 会话管理
        .route("/api/v1/sessions", post(create_session_handler))
        .route("/api/v1/sessions/:session_id", get(get_session_handler))
        .route("/api/v1/sessions/:session_id/messages", get(get_session_messages_handler))
        // 聊天
        .route("/api/v1/chat", post(chat_handler))
        // WebSocket
        .route("/ws/:session_id", get(websocket_handler))
        // 中间件
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// 健康检查处理器
async fn health_handler() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// 创建会话处理器
async fn create_session_handler(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    
    tracing::info!("Creating new session: {}", session_id);

    let response = CreateSessionResponse {
        session_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::CREATED, Json(response))
}

/// 获取会话处理器
async fn get_session_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    tracing::debug!("Fetching session: {}", session_id);

    // 获取会话历史
    match AgentRunner::get_session_history(&state.agent_state, &session_id).await {
        Ok(messages) => {
            let response = SessionHistoryResponse {
                session_id,
                messages,
            };
            (StatusCode::OK, Json(json!(response)))
        }
        Err(e) => {
            tracing::error!("Failed to get session history: {}", e);
            let error = ErrorResponse {
                error: e.to_string(),
                code: "SESSION_ERROR".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(error)))
        }
    }
}

/// 获取会话消息处理器
async fn get_session_messages_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match AgentRunner::get_session_history(&state.agent_state, &session_id).await {
        Ok(messages) => {
            (StatusCode::OK, Json(json!({
                "session_id": session_id,
                "messages": messages,
            })))
        }
        Err(e) => {
            tracing::error!("Failed to get messages: {}", e);
            let error = ErrorResponse {
                error: e.to_string(),
                code: "FETCH_ERROR".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(error)))
        }
    }
}

/// 聊天处理器
async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let session_id = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    tracing::info!(
        "Processing chat request - session: {}, message: {}",
        session_id,
        req.message
    );

    // HTTP 客户端: 使用 Http 回复通道
    let reply_to = ReplyChannel::Http(session_id.clone());

    // 处理聊天消息
    match AgentRunner::handle_chat(
        &state.agent_state,
        &session_id,
        &req.message,
        &reply_to,
    ).await {
        Ok(response) => {
            let api_response = SendMessageResponse {
                session_id: response.session_id,
                message_id: response.message_id,
                content: response.content,
                role: response.role,
                timestamp: response.timestamp.to_rfc3339(),
            };
            (StatusCode::OK, Json(json!(api_response)))
        }
        Err(e) => {
            tracing::error!("Chat processing failed: {}", e);
            let error = ErrorResponse {
                error: e.to_string(),
                code: "CHAT_ERROR".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(error)))
        }
    }
}

/// WebSocket 处理器
async fn websocket_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state, session_id))
}

/// 处理 WebSocket 连接
async fn handle_websocket(
    socket: WebSocket,
    state: Arc<AppState>,
    session_id: String,
) {
    tracing::info!("WebSocket connected for session: {}", session_id);

    let (mut sender, mut receiver) = socket.split();

    // 创建 WebSocket 通道
    let (ws_tx, mut ws_rx) = mpsc::channel::<ChatResponse>(100);

    // 订阅事件总线
    let mut event_rx = state.agent_state.event_bus.subscribe(&session_id).await;

    // 发送任务: 将事件总线的事件转发到 WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(response) = ws_rx.recv().await {
            let msg = serde_json::to_string(&response).unwrap_or_default();
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 接收任务: 处理客户端消息
    let state_clone = state.clone();
    let session_id_clone = session_id.clone();
    let ws_tx_clone = ws_tx.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    tracing::debug!("Received WebSocket message: {}", text);
                    
                    // 解析消息
                    if let Ok(req) = serde_json::from_str::<SendMessageRequest>(&text) {
                        let reply_to = ReplyChannel::WebSocket(ws_tx_clone.clone());
                        
                        if let Err(e) = AgentRunner::handle_chat(
                            &state_clone.agent_state,
                            &session_id_clone,
                            &req.message,
                            &reply_to,
                        ).await {
                            tracing::error!("Failed to handle chat: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    tracing::info!("WebSocket closed for session: {}", session_id_clone);
                    break;
                }
                _ => {}
            }
        }
    });

    // 事件转发任务: 将事件总线的事件转发到 WebSocket 通道
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                Event::ChatResponse { session_id, message_id, content, role, timestamp } => {
                    let response = ChatResponse {
                        session_id,
                        message_id,
                        content,
                        role,
                        timestamp,
                        metadata: None,
                    };
                    let _ = ws_tx.send(response).await;
                }
                _ => {}
            }
        }
    });

    // 等待任意任务完成
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = event_task => {},
    }

    // 取消订阅
    state.agent_state.event_bus.unsubscribe(&session_id).await;
    
    tracing::info!("WebSocket disconnected for session: {}", session_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventBus, SessionStorage};
    use std::sync::Mutex;

    // 模拟存储实现
    struct MockStorage {
        messages: Mutex<Vec<(String, ChatResponse)>>,
    }

    #[async_trait::async_trait]
    impl SessionStorage for MockStorage {
        async fn save_message(&self, session_id: &str, message: &ChatResponse) -> anyhow::Result<()> {
            self.messages
                .lock()
                .unwrap()
                .push((session_id.to_string(), message.clone()));
            Ok(())
        }

        async fn get_messages(&self, session_id: &str) -> anyhow::Result<Vec<ChatResponse>> {
            let messages = self.messages.lock().unwrap();
            Ok(messages
                .iter()
                .filter(|(sid, _)| sid == session_id)
                .map(|(_, msg)| msg.clone())
                .collect())
        }
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let event_bus = Arc::new(EventBus::new());
        let storage: Arc<dyn SessionStorage> = Arc::new(MockStorage {
            messages: Mutex::new(Vec::new()),
        });
        
        let agent_state = Arc::new(AgentRunnerState::new(event_bus, storage));
        let config = ServerConfig::default();
        
        let state = AppState::new(agent_state, config);
        
        assert_eq!(state.config.port, 3000);
    }

    #[tokio::test]
    async fn test_server_config_from_env() {
        // 设置环境变量
        std::env::set_var("BAMBOO_PORT", "4000");
        std::env::set_var("LLM_MODEL", "test-model");
        
        let config = ServerConfig::from_env();
        
        assert_eq!(config.port, 4000);
        assert_eq!(config.llm_model, "test-model");
        
        // 清理环境变量
        std::env::remove_var("BAMBOO_PORT");
        std::env::remove_var("LLM_MODEL");
    }
}
