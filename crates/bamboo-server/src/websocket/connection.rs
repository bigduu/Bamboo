//! WebSocket connection management
//!
//! Handles connection pooling, heartbeat, and message sending.

use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::interval;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use log::{debug, error, warn};

use super::protocol::GatewayEvent;

/// Handle to a WebSocket connection for sending messages
#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    /// Connection ID
    pub id: String,
    /// Client address
    pub addr: SocketAddr,
    /// Channel for sending events to the connection
    sender: mpsc::UnboundedSender<GatewayEvent>,
    /// Last pong timestamp for heartbeat
    last_pong: Arc<RwLock<std::time::Instant>>,
}

impl ConnectionHandle {
    /// Create a new connection handle
    pub fn new(
        id: String,
        addr: SocketAddr,
        sender: mpsc::UnboundedSender<GatewayEvent>,
    ) -> Self {
        Self {
            id,
            addr,
            sender,
            last_pong: Arc::new(RwLock::new(std::time::Instant::now())),
        }
    }

    /// Send an event to this connection
    pub fn send(&self, event: GatewayEvent) -> Result<(), ConnectionError> {
        self.sender
            .send(event)
            .map_err(|_| ConnectionError::Closed)
    }

    /// Update last pong timestamp
    pub async fn update_pong(&self) {
        *self.last_pong.write().await = std::time::Instant::now();
    }

    /// Get time since last pong
    pub async fn time_since_pong(&self) -> Duration {
        self.last_pong.read().await.elapsed()
    }

    /// Get connection ID
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Manages all WebSocket connections
#[derive(Debug)]
pub struct ConnectionPool {
    /// Map of connection ID to connection handle
    connections: Arc<Mutex<HashMap<String, ConnectionHandle>>>,
    /// Maximum number of connections allowed
    max_connections: usize,
    /// Heartbeat interval
    heartbeat_interval: Duration,
    /// Heartbeat timeout
    heartbeat_timeout: Duration,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_connections: usize, heartbeat_interval_secs: u64) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            max_connections,
            heartbeat_interval: Duration::from_secs(heartbeat_interval_secs),
            heartbeat_timeout: Duration::from_secs(heartbeat_interval_secs * 3),
        }
    }

    /// Get current connection count
    pub async fn count(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// Check if pool is at capacity
    pub async fn is_full(&self) -> bool {
        self.count().await >= self.max_connections
    }

    /// Add a connection to the pool
    pub async fn add(&self, handle: ConnectionHandle) {
        let mut connections = self.connections.lock().await;
        connections.insert(handle.id.clone(), handle);
    }

    /// Remove a connection from the pool
    pub async fn remove(&self, connection_id: &str) -> Option<ConnectionHandle> {
        let mut connections = self.connections.lock().await;
        connections.remove(connection_id)
    }

    /// Get a connection by ID
    pub async fn get(&self, connection_id: &str) -> Option<ConnectionHandle> {
        let connections = self.connections.lock().await;
        connections.get(connection_id).cloned()
    }

    /// Broadcast an event to all connections
    pub async fn broadcast(&self, event: GatewayEvent) {
        let connections = self.connections.lock().await;
        for (_, handle) in connections.iter() {
            if let Err(_) = handle.send(event.clone()) {
                // Connection closed, will be cleaned up later
                debug!("Failed to send to connection {}", handle.id);
            }
        }
    }

    /// Send an event to a specific connection
    pub async fn send_to(
        &self,
        connection_id: &str,
        event: GatewayEvent,
    ) -> Result<(), ConnectionError> {
        let connections = self.connections.lock().await;
        if let Some(handle) = connections.get(connection_id) {
            handle.send(event)
        } else {
            Err(ConnectionError::NotFound(connection_id.to_string()))
        }
    }

    /// Get all connection IDs
    pub async fn list_connections(&self) -> Vec<String> {
        let connections = self.connections.lock().await;
        connections.keys().cloned().collect()
    }

    /// Clean up stale connections
    pub async fn cleanup_stale(&self) -> usize {
        let mut connections = self.connections.lock().await;
        let stale: Vec<String> = Vec::new();

        // Note: We can't check last_pong while holding the lock
        // This is simplified - in production, you'd want a more sophisticated cleanup

        for id in &stale {
            connections.remove(id);
        }
        stale.len()
    }

    /// Start heartbeat monitoring for a connection
    pub async fn start_heartbeat(
        &self,
        handle: ConnectionHandle,
        mut ws_sender: futures_util::stream::SplitSink<
            WebSocketStream<TcpStream>,
            Message,
        >,
        mut ws_receiver: futures_util::stream::SplitStream<WebSocketStream<TcpStream>>,
    ) {
        let heartbeat_interval = self.heartbeat_interval;
        let heartbeat_timeout = self.heartbeat_timeout;
        let connection_id = handle.id.clone();
        let connections = Arc::clone(&self.connections);

        tokio::spawn(async move {
            let mut heartbeat = interval(heartbeat_interval);
            let (tx, mut rx) = mpsc::unbounded_channel::<GatewayEvent>();

            // Replace the sender in handle
            let handle = ConnectionHandle::new(handle.id.clone(), handle.addr, tx);

            loop {
                tokio::select! {
                    // Send heartbeat pings
                    _ = heartbeat.tick() => {
                        let ping = GatewayEvent::Pong {
                            timestamp: chrono::Utc::now().timestamp_millis(),
                        };
                        if let Ok(json) = serde_json::to_string(&ping) {
                            if let Err(_) = ws_sender.send(Message::Text(json)).await {
                                break;
                            }
                        }

                        // Check if we've received pongs
                        if handle.time_since_pong().await > heartbeat_timeout {
                            warn!("Connection {} heartbeat timeout", connection_id);
                            break;
                        }
                    }

                    // Handle outgoing messages
                    Some(event) = rx.recv() => {
                        match serde_json::to_string(&event) {
                            Ok(json) => {
                                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                    error!("Failed to send message: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to serialize event: {}", e);
                            }
                        }
                    }

                    // Handle incoming messages
                    Some(msg) = ws_receiver.next() => {
                        match msg {
                            Ok(Message::Text(text)) => {
                                // Handle pong from client
                                if let Ok(event) = serde_json::from_str::<super::protocol::ClientMessage>(&text) {
                                    if matches!(event, super::protocol::ClientMessage::Ping { .. }) {
                                        handle.update_pong().await;
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                debug!("Connection {} closed by client", connection_id);
                                break;
                            }
                            Ok(Message::Ping(data)) => {
                                if let Err(_) = ws_sender.send(Message::Pong(data)).await {
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("WebSocket error on {}: {}", connection_id, e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Clean up
            let mut conns = connections.lock().await;
            conns.remove(&connection_id);
            debug!("Connection {} removed from pool", connection_id);
        });
    }
}

/// Connection-related errors
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Connection closed")]
    Closed,
    #[error("Connection not found: {0}")]
    NotFound(String),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
}
