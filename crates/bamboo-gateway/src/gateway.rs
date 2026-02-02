//! Gateway main structure
//!
//! The WebSocket server that manages sessions and routes messages.

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::connection::{ConnectionError, ConnectionHandle, ConnectionPool};
use crate::protocol::{ClientMessage, GatewayEvent};
use crate::router::{IncomingMessage, MessageRouter, RouteResult};
use crate::session::SessionManager;

/// Gateway configuration
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Bind address (e.g., "127.0.0.1:18790")
    pub bind: String,
    /// Optional authentication token
    pub auth_token: Option<String>,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:18790".to_string(),
            auth_token: None,
            max_connections: 1000,
            heartbeat_interval_secs: 30,
        }
    }
}

/// 消息处理器类型
type MessageHandler = Arc<RwLock<Option<Box<dyn Fn(String, String) + Send + Sync>>>>;

/// The main Gateway server
pub struct Gateway {
    config: GatewayConfig,
    session_manager: Arc<SessionManager>,
    connection_pool: Arc<ConnectionPool>,
    message_router: Arc<MessageRouter>,
    /// 消息处理器回调
    message_handler: MessageHandler,
    /// 消息发送通道（用于内部消息传递）
    message_tx: Arc<RwLock<Option<mpsc::UnboundedSender<(String, String)>>>>,
}

impl std::fmt::Debug for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gateway")
            .field("config", &self.config)
            .field("session_manager", &self.session_manager)
            .field("connection_pool", &self.connection_pool)
            .field("message_router", &self.message_router)
            .field("message_handler", &"<callback>")
            .finish()
    }
}

impl Clone for Gateway {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            session_manager: self.session_manager.clone(),
            connection_pool: self.connection_pool.clone(),
            message_router: self.message_router.clone(),
            message_handler: self.message_handler.clone(),
            message_tx: self.message_tx.clone(),
        }
    }
}

impl Gateway {
    /// Create a new Gateway instance
    pub fn new(config: GatewayConfig) -> Self {
        let session_manager = Arc::new(SessionManager::new());
        let connection_pool = Arc::new(ConnectionPool::new(
            config.max_connections,
            config.heartbeat_interval_secs,
        ));
        let message_router = Arc::new(MessageRouter::new());

        Self {
            config,
            session_manager,
            connection_pool,
            message_router,
            message_handler: Arc::new(RwLock::new(None)),
            message_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Get the connection pool
    pub fn connection_pool(&self) -> &ConnectionPool {
        &self.connection_pool
    }

    /// Get the message router
    pub fn message_router(&self) -> &MessageRouter {
        &self.message_router
    }

    /// 设置消息处理器
    /// 
    /// 当收到 Chat 消息时，会调用此处理器
    /// 参数：session_id, content
    pub async fn on_message<F>(&self, handler: F)
    where
        F: Fn(String, String) + Send + Sync + 'static,
    {
        let mut h = self.message_handler.write().await;
        *h = Some(Box::new(handler));
    }

    /// 获取消息接收通道
    /// 
    /// 返回一个接收器，可以用来接收来自 Gateway 的消息
    pub async fn message_receiver(&self) -> Option<mpsc::UnboundedReceiver<(String, String)>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut message_tx = self.message_tx.write().await;
        *message_tx = Some(tx);
        Some(rx)
    }

    /// Run the gateway server
    pub async fn run(&self) -> Result<(), GatewayError> {
        let addr: SocketAddr = self.config.bind.parse()?;
        let listener = TcpListener::bind(&addr).await?;

        info!("Gateway listening on ws://{}", addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            debug!("New connection from {}", peer_addr);

            if self.connection_pool.is_full().await {
                warn!("Connection pool full, rejecting {}", peer_addr);
                // Send error and close
                let _ = self.reject_connection(stream, "Server at capacity").await;
                continue;
            }

            let gateway = self.clone();
            tokio::spawn(async move {
                if let Err(e) = gateway.handle_connection(stream, peer_addr).await {
                    error!("Connection error for {}: {}", peer_addr, e);
                }
            });
        }
    }

    /// Reject a connection with an error message
    async fn reject_connection(
        &self,
        stream: TcpStream,
        reason: &str,
    ) -> Result<(), GatewayError> {
        let ws_stream = accept_async(stream).await?;
        let (mut sender, _) = ws_stream.split();
        let error_event = GatewayEvent::Error {
            code: "CAPACITY_EXCEEDED".to_string(),
            message: reason.to_string(),
        };
        let json = serde_json::to_string(&error_event)?;
        sender.send(Message::Text(json)).await?;
        sender.close().await?;
        Ok(())
    }

    /// Handle a WebSocket connection
    async fn handle_connection(
        &self,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), GatewayError> {
        let ws_stream = accept_async(stream).await?;
        let (mut sender, mut receiver) = ws_stream.split();
        let connection_id = Uuid::new_v4().to_string();

        // Channel for sending events to this connection
        let (tx, mut rx) = mpsc::unbounded_channel::<GatewayEvent>();

        // Create connection handle
        let conn_handle = ConnectionHandle::new(
            connection_id.clone(),
            addr,
            tx,
        );

        // Add to pool
        self.connection_pool.add(conn_handle.clone()).await;

        // Handle the connection lifecycle
        let mut current_session: Option<Arc<tokio::sync::RwLock<crate::session::Session>>> = None;

        loop {
            tokio::select! {
                // Handle outgoing messages
                Some(event) = rx.recv() => {
                    match serde_json::to_string(&event) {
                        Ok(json) => {
                            if let Err(e) = sender.send(Message::Text(json)).await {
                                error!("Failed to send to {}: {}", addr, e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to serialize event: {}", e);
                        }
                    }
                }

                // Handle incoming messages
                Some(msg) = receiver.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<ClientMessage>(&text) {
                                Ok(client_msg) => {
                                    match &client_msg {
                                        ClientMessage::Connect { session_id, auth } => {
                                            // Validate auth if required
                                            if let Some(ref required_auth) = self.config.auth_token {
                                                if auth.as_ref() != Some(required_auth) {
                                                    let error = GatewayEvent::Error {
                                                        code: "UNAUTHORIZED".to_string(),
                                                        message: "Invalid auth token".to_string(),
                                                    };
                                                    let _ = conn_handle.send(error);
                                                    break;
                                                }
                                            }

                                            // Create or restore session
                                            let session = self.session_manager
                                                .create_or_restore(session_id.clone(), "anonymous")
                                                .await;

                                            {
                                                let mut session_guard = session.write().await;
                                                session_guard.connection = Some(conn_handle.clone());
                                            }

                                            let session_id_clone = session.read().await.id.clone();
                                            current_session = Some(session);

                                            let event = GatewayEvent::Connected {
                                                session_id: session_id_clone,
                                            };
                                            let _ = conn_handle.send(event);
                                        }
                                        ClientMessage::Chat { content, session_id } => {
                                            // 处理 Chat 消息
                                            if let Some(ref session) = current_session {
                                                let session_id = session.read().await.id.clone();
                                                
                                                // 调用消息处理器（如果设置）
                                                let handler = self.message_handler.read().await;
                                                if let Some(ref h) = *handler {
                                                    h(session_id.clone(), content.clone());
                                                }
                                                drop(handler);

                                                // 也通过通道发送
                                                let message_tx = self.message_tx.read().await;
                                                if let Some(ref tx) = *message_tx {
                                                    let _ = tx.send((session_id, content.clone()));
                                                }
                                                drop(message_tx);

                                                // 路由消息
                                                let incoming: IncomingMessage = client_msg.into();
                                                match self.message_router.route(incoming, session).await {
                                                    RouteResult::Response(event) => {
                                                        let _ = conn_handle.send(event);
                                                    }
                                                    RouteResult::ToAgent(session, _msg) => {
                                                        // 消息已转发到外部处理器
                                                        debug!("Chat message routed to agent for session {}", 
                                                            session.read().await.id);
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                let error = GatewayEvent::Error {
                                                    code: "NOT_CONNECTED".to_string(),
                                                    message: "Send Connect first".to_string(),
                                                };
                                                let _ = conn_handle.send(error);
                                            }
                                        }
                                        _ => {
                                            if let Some(ref session) = current_session {
                                                let incoming: IncomingMessage = client_msg.into();
                                                match self.message_router.route(incoming, session).await {
                                                    RouteResult::Response(event) => {
                                                        let _ = conn_handle.send(event);
                                                    }
                                                    RouteResult::ToAgent(session, _msg) => {
                                                        // Handle agent routing
                                                        debug!("Routing to agent");
                                                        let _ = conn_handle.send(GatewayEvent::AgentToken {
                                                            session_id: session.read().await.id.clone(),
                                                            token: "Message received".to_string(),
                                                        });
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                let error = GatewayEvent::Error {
                                                    code: "NOT_CONNECTED".to_string(),
                                                    message: "Send Connect first".to_string(),
                                                };
                                                let _ = conn_handle.send(error);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Invalid message from {}: {}", addr, e);
                                    let error = GatewayEvent::Error {
                                        code: "INVALID_MESSAGE".to_string(),
                                        message: format!("Failed to parse message: {}", e),
                                    };
                                    let _ = conn_handle.send(error);
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!("Connection {} closed", addr);
                            break;
                        }
                        Ok(Message::Ping(data)) => {
                            if let Err(e) = sender.send(Message::Pong(data)).await {
                                error!("Failed to send pong: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("WebSocket error on {}: {}", addr, e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Cleanup
        if let Some(session) = current_session {
            let session_id = session.read().await.id.clone();
            self.session_manager.detach_connection(&session_id).await.ok();
        }
        self.connection_pool.remove(&connection_id).await;
        info!("Connection {} disconnected", addr);

        Ok(())
    }

    /// Broadcast an event to all connected clients
    pub async fn broadcast(&self, event: GatewayEvent) {
        self.connection_pool.broadcast(event).await;
    }

    /// Send an event to a specific session
    pub async fn send_to(&self, session_id: &str, event: GatewayEvent) -> Result<(), GatewayError> {
        if let Some(session) = self.session_manager.get(session_id).await {
            let session = session.read().await;
            if let Some(ref conn) = session.connection {
                conn.send(event)?;
                Ok(())
            } else {
                Err(GatewayError::SessionNotConnected(session_id.to_string()))
            }
        } else {
            Err(GatewayError::SessionNotFound(session_id.to_string()))
        }
    }
}

impl std::fmt::Display for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gateway({})", self.config.bind)
    }
}

/// Gateway-related errors
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session not connected: {0}")]
    SessionNotConnected(String),
    #[error("Channel closed")]
    ChannelClosed,
}

/// Example: Run a gateway server
///
/// ```no_run
/// use bamboo_gateway::{Gateway, GatewayConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = GatewayConfig {
///         bind: "127.0.0.1:18790".to_string(),
///         auth_token: None,
///         max_connections: 1000,
///         heartbeat_interval_secs: 30,
///     };
///
///     let gateway = Gateway::new(config);
///     gateway.run().await?;
///     Ok(())
/// }
/// ```
pub mod examples {}
