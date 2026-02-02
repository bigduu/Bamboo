//! Session management
//!
//! Handles session creation, lookup, and lifecycle management.

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::connection::ConnectionHandle;

/// A handle to a session for external reference
pub type SessionHandle = Arc<RwLock<Session>>;

/// Session state
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Associated user ID
    pub user_id: String,
    /// Active connection handle (if connected)
    pub connection: Option<ConnectionHandle>,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(user_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            connection: None,
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a session with specific ID
    pub fn with_id(id: impl Into<String>, user_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            user_id: user_id.into(),
            connection: None,
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Check if session is currently connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Get session age
    pub fn age(&self) -> Duration {
        Utc::now() - self.created_at
    }

    /// Get time since last activity
    pub fn idle_time(&self) -> Duration {
        Utc::now() - self.last_activity
    }
}

/// Manages all active sessions
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// Map of session ID to session handle
    sessions: Arc<DashMap<String, SessionHandle>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Create a new session for a user
    pub async fn create(&self, user_id: impl Into<String>) -> SessionHandle {
        let session = Session::new(user_id);
        let handle = Arc::new(RwLock::new(session));
        let id = handle.read().await.id.clone();
        self.sessions.insert(id, Arc::clone(&handle));
        handle
    }

    /// Create or restore a session by ID
    pub async fn create_or_restore(
        &self,
        session_id: Option<String>,
        user_id: impl Into<String>,
    ) -> SessionHandle {
        if let Some(id) = session_id {
            if let Some(session) = self.get(&id).await {
                session.write().await.touch();
                return session;
            }
        }
        self.create(user_id).await
    }

    /// Get a session by ID
    pub async fn get(&self, id: &str) -> Option<SessionHandle> {
        self.sessions.get(id).map(|entry| Arc::clone(entry.value()))
    }

    /// Attach a connection to a session
    pub async fn attach_connection(
        &self,
        session_id: &str,
        conn: ConnectionHandle,
    ) -> Result<(), SessionError> {
        if let Some(session) = self.sessions.get(session_id) {
            let mut session = session.write().await;
            session.connection = Some(conn);
            session.touch();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Detach connection from a session
    pub async fn detach_connection(&self, session_id: &str) -> Result<(), SessionError> {
        if let Some(session) = self.sessions.get(session_id) {
            let mut session = session.write().await;
            session.connection = None;
            session.touch();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Remove a session
    pub async fn remove(&self, session_id: &str) -> Option<SessionHandle> {
        self.sessions.remove(session_id).map(|(_, handle)| handle)
    }

    /// Get all session IDs
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get connected session count
    pub fn connected_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|entry| {
                // Try to read without blocking - if we can't get the lock, assume not connected
                entry.value().try_read().map(|s| s.is_connected()).unwrap_or(false)
            })
            .count()
    }

    /// Get total session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Cleanup inactive sessions older than max_age
    pub async fn cleanup_inactive(&self, max_age: Duration) -> usize {
        let to_remove: Vec<String> = self
            .sessions
            .iter()
            .filter(|entry| {
                entry
                    .value()
                    .try_read()
                    .map(|s| s.idle_time() > max_age && !s.is_connected())
                    .unwrap_or(true)
            })
            .map(|entry| entry.key().clone())
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.sessions.remove(&id);
        }
        count
    }

    /// Check if a session exists
    pub fn exists(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }
}

/// Session-related errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session already connected: {0}")]
    AlreadyConnected(String),
}
