//! # Session Manager
//! 
//! 提供 Session 生命周期管理、断线重连支持和内存缓存。
//! 
//! 这是 Gateway 层的核心组件，负责：
//! - 管理活跃会话的内存缓存
//! - 处理连接断开和重连
//! - 协调持久化存储操作
//! - 维护会话状态机

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::error::{StorageError, StorageResult};
use crate::jsonl_storage::JsonlStorage;
use crate::storage::{SessionStorage, Storage};
use crate::types::{
    AgentEvent, Message, Session, SessionFilter, SessionListResult, SessionMetadata, SessionState,
};

/// SessionManager 配置
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// 空闲会话超时（断开连接后保持内存缓存的时间）
    pub idle_timeout_secs: u64,
    /// 断开连接会话的保留时间（支持重连）
    pub disconnect_retention_secs: u64,
    /// 自动保存间隔
    pub auto_save_interval_secs: u64,
    /// 最大活跃会话数
    pub max_active_sessions: usize,
    /// 启用自动清理
    pub enable_auto_cleanup: bool,
    /// 清理间隔
    pub cleanup_interval_secs: u64,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 300,       // 5分钟空闲超时
            disconnect_retention_secs: 3600, // 1小时断开保留
            auto_save_interval_secs: 60,  // 1分钟自动保存
            max_active_sessions: 1000,
            enable_auto_cleanup: true,
            cleanup_interval_secs: 3600,  // 1小时清理一次
        }
    }
}

impl SessionManagerConfig {
    /// 设置空闲超时
    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }

    /// 设置断开保留时间
    pub fn with_disconnect_retention(mut self, secs: u64) -> Self {
        self.disconnect_retention_secs = secs;
        self
    }

    /// 设置最大活跃会话数
    pub fn with_max_sessions(mut self, max: usize) -> Self {
        self.max_active_sessions = max;
        self
    }
}

/// 内存中的会话条目
#[derive(Debug)]
struct SessionEntry {
    /// 会话数据
    session: Session,
    /// 最后访问时间
    last_accessed: chrono::DateTime<Utc>,
    /// 连接状态
    connection_state: ConnectionState,
    /// 连接ID（如果有）
    connection_id: Option<String>,
    /// 是否已修改（需要保存）
    dirty: bool,
    /// 事件发送通道（用于流式推送）
    event_tx: Option<mpsc::UnboundedSender<AgentEvent>>,
}

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    /// 已连接
    Connected,
    /// 空闲（无连接但保留在内存）
    Idle,
    /// 已断开（可重连）
    Disconnected,
}

impl SessionEntry {
    fn new(session: Session) -> Self {
        Self {
            session,
            last_accessed: Utc::now(),
            connection_state: ConnectionState::Idle,
            connection_id: None,
            dirty: false,
            event_tx: None,
        }
    }

    fn touch(&mut self) {
        self.last_accessed = Utc::now();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    fn is_expired(&self, timeout_secs: u64) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.last_accessed);
        elapsed.num_seconds() > timeout_secs as i64
    }
}

/// SessionManager
/// 
/// 管理会话的生命周期，提供内存缓存和持久化协调。
pub struct SessionManager {
    config: SessionManagerConfig,
    storage: Arc<JsonlStorage>,
    /// 内存会话缓存
    sessions: DashMap<String, RwLock<SessionEntry>>,
    /// 连接ID到会话ID的映射
    connection_map: DashMap<String, String>,
}

impl SessionManager {
    /// 创建新的 SessionManager
    pub async fn new(
        config: SessionManagerConfig,
        storage: Arc<JsonlStorage>,
    ) -> StorageResult<Arc<Self>> {
        let manager = Arc::new(Self {
            config,
            storage,
            sessions: DashMap::new(),
            connection_map: DashMap::new(),
        });

        // 启动后台任务
        let manager_clone = Arc::clone(&manager);
        tokio::spawn(async move {
            manager_clone.background_tasks().await;
        });

        info!("SessionManager initialized");
        Ok(manager)
    }

    /// 后台任务（自动保存、清理等）
    async fn background_tasks(&self) {
        let mut save_interval = interval(Duration::from_secs(self.config.auto_save_interval_secs));
        let mut cleanup_interval = interval(Duration::from_secs(self.config.cleanup_interval_secs));

        loop {
            tokio::select! {
                _ = save_interval.tick() => {
                    if let Err(e) = self.auto_save_dirty_sessions().await {
                        error!("Auto-save failed: {}", e);
                    }
                }
                _ = cleanup_interval.tick() => {
                    if self.config.enable_auto_cleanup {
                        if let Err(e) = self.cleanup_expired_sessions().await {
                            error!("Cleanup failed: {}", e);
                        }
                    }
                }
            }
        }
    }

    /// 自动保存脏会话
    async fn auto_save_dirty_sessions(&self) -> StorageResult<()> {
        // 收集需要保存的会话ID和会话数据（不持有锁跨越await）
        let dirty_sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|entry| entry.value().read().dirty)
            .map(|entry| {
                let session_id = entry.key().clone();
                let session = entry.value().read().session.clone();
                (session_id, session)
            })
            .collect();

        for (session_id, session) in dirty_sessions {
            // 先保存到存储
            if let Err(e) = self.storage.save_session(&session).await {
                error!("Failed to auto-save session {}: {}", session_id, e);
            } else {
                // 保存成功后清除dirty标记
                if let Some(entry) = self.sessions.get(&session_id) {
                    entry.write().clear_dirty();
                }
                debug!("Auto-saved session: {}", session_id);
            }
        }

        Ok(())
    }

    /// 清理过期会话
    async fn cleanup_expired_sessions(&self) -> StorageResult<()> {
        // 清理存储中的过期会话
        let count = self.storage.cleanup_expired_sessions().await?;
        if count > 0 {
            info!("Cleaned up {} expired sessions from storage", count);
        }

        // 清理内存中的过期空闲会话
        let expired_ids: Vec<_> = self
            .sessions
            .iter()
            .filter(|entry| {
                let entry = entry.value().read();
                match entry.connection_state {
                    ConnectionState::Idle => {
                        entry.is_expired(self.config.idle_timeout_secs)
                    }
                    ConnectionState::Disconnected => {
                        entry.is_expired(self.config.disconnect_retention_secs)
                    }
                    _ => false,
                }
            })
            .map(|entry| {
                let id = entry.key().clone();
                let session = entry.value().read().session.clone();
                let dirty = entry.value().read().dirty;
                (id, session, dirty)
            })
            .collect();

        let evicted_count = expired_ids.len();
        for (id, session, dirty) in expired_ids {
            // 先保存（如果需要）
            if dirty {
                if let Err(e) = self.storage.save_session(&session).await {
                    error!("Failed to save session {} before eviction: {}", id, e);
                }
            }
            // 从内存中移除
            self.sessions.remove(&id);
            debug!("Evicted session from memory: {}", id);
        }

        if evicted_count > 0 {
            info!("Evicted {} sessions from memory", evicted_count);
        }

        Ok(())
    }

    /// 创建新会话
    pub async fn create_session(
        &self,
        user_id: Option<String>,
        title: Option<String>,
    ) -> StorageResult<Session> {
        // 检查最大会话数
        if self.sessions.len() >= self.config.max_active_sessions {
            return Err(StorageError::other("Max active sessions reached"));
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let mut session = Session::new(&session_id);

        if let Some(uid) = user_id {
            session.metadata.user_id = Some(uid);
        }
        if let Some(t) = title {
            session.metadata.title = Some(t);
        }

        // 保存到存储
        self.storage.create_session(&session).await?;

        // 添加到内存缓存
        let entry = SessionEntry::new(session.clone());
        self.sessions.insert(session_id, RwLock::new(entry));

        info!("Created session: {}", session.metadata.id);
        Ok(session)
    }

    /// 获取会话（优先从内存，否则从存储加载）
    pub async fn get_session(&self, session_id: &str) -> StorageResult<Option<Session>> {
        // 优先从内存获取
        if let Some(entry) = self.sessions.get(session_id) {
            let mut entry = entry.write();
            entry.touch();
            return Ok(Some(entry.session.clone()));
        }

        // 从存储加载
        if let Some(session) = self.storage.load_session(session_id).await? {
            // 添加到内存缓存
            let entry = SessionEntry::new(session.clone());
            self.sessions.insert(session_id.to_string(), RwLock::new(entry));
            debug!("Loaded session from storage: {}", session_id);
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    /// 获取或创建会话
    pub async fn get_or_create_session(
        &self,
        session_id: Option<String>,
        user_id: Option<String>,
    ) -> StorageResult<Session> {
        if let Some(id) = session_id {
            match self.get_session(&id).await? {
                Some(session) => {
                    // 验证用户权限
                    if let Some(ref uid) = user_id {
                        if session.metadata.user_id.as_ref() != Some(uid) {
                            return Err(StorageError::other("Session access denied"));
                        }
                    }
                    Ok(session)
                }
                None => {
                    // 会话不存在，创建新的
                    warn!("Session {} not found, creating new", id);
                    self.create_session(user_id, None).await
                }
            }
        } else {
            // 创建新会话
            self.create_session(user_id, None).await
        }
    }

    /// 连接会话（用于 Gateway 连接建立时）
    /// 
    /// 返回事件接收通道
    pub async fn connect_session(
        &self,
        session_id: &str,
        connection_id: String,
    ) -> StorageResult<mpsc::UnboundedReceiver<AgentEvent>> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        if let Some(entry) = self.sessions.get(session_id) {
            let mut entry = entry.write();
            entry.connection_state = ConnectionState::Connected;
            entry.connection_id = Some(connection_id.clone());
            entry.event_tx = Some(event_tx);
            entry.touch();

            // 更新状态
            entry.session.set_state(SessionState::Active);
            entry.mark_dirty();
        } else {
            // 会话不在内存中，尝试加载
            match self.storage.load_session(session_id).await? {
                Some(session) => {
                    let mut entry = SessionEntry::new(session);
                    entry.connection_state = ConnectionState::Connected;
                    entry.connection_id = Some(connection_id.clone());
                    entry.event_tx = Some(event_tx);
                    entry.session.set_state(SessionState::Active);
                    entry.mark_dirty();

                    self.sessions
                        .insert(session_id.to_string(), RwLock::new(entry));
                }
                None => {
                    return Err(StorageError::SessionNotFound {
                        id: session_id.to_string(),
                    });
                }
            }
        }

        // 记录连接映射
        self.connection_map.insert(connection_id, session_id.to_string());

        info!("Connected session: {}", session_id);
        Ok(event_rx)
    }

    /// 断开会话（用于 Gateway 连接断开时）
    /// 
    /// 会话保持，支持重连
    pub async fn disconnect_session(
        &self,
        session_id: &str,
    ) -> StorageResult<()> {
        if let Some(entry) = self.sessions.get(session_id) {
            let mut entry = entry.write();
            
            // 移除连接映射
            if let Some(ref conn_id) = entry.connection_id {
                self.connection_map.remove(conn_id);
            }

            entry.connection_state = ConnectionState::Disconnected;
            entry.connection_id = None;
            entry.event_tx = None;
            entry.touch();

            // 更新状态
            entry.session.set_state(SessionState::Disconnected);
            entry.mark_dirty();

            // 立即保存
            self.storage.save_session(&entry.session).await?;
        }

        info!("Disconnected session: {}", session_id);
        Ok(())
    }

    /// 通过连接ID断开会话
    pub async fn disconnect_by_connection(&self, connection_id: &str) -> StorageResult<()> {
        if let Some((_, session_id)) = self.connection_map.remove(connection_id) {
            self.disconnect_session(&session_id).await?;
        }
        Ok(())
    }

    /// 重连会话
    /// 
    /// 如果会话处于 Disconnected 状态，可以重连
    pub async fn reconnect_session(
        &self,
        session_id: &str,
        new_connection_id: String,
    ) -> StorageResult<mpsc::UnboundedReceiver<AgentEvent>> {
        if let Some(entry) = self.sessions.get(session_id) {
            let mut entry = entry.write();
            
            match entry.connection_state {
                ConnectionState::Disconnected | ConnectionState::Idle => {
                    let (event_tx, event_rx) = mpsc::unbounded_channel();

                    // 移除旧的连接映射
                    if let Some(ref old_conn_id) = entry.connection_id {
                        self.connection_map.remove(old_conn_id);
                    }

                    entry.connection_state = ConnectionState::Connected;
                    entry.connection_id = Some(new_connection_id.clone());
                    entry.event_tx = Some(event_tx);
                    entry.touch();

                    // 更新状态
                    entry.session.set_state(SessionState::Active);
                    entry.mark_dirty();

                    // 记录新的连接映射
                    self.connection_map
                        .insert(new_connection_id, session_id.to_string());

                    info!("Reconnected session: {}", session_id);
                    Ok(event_rx)
                }
                ConnectionState::Connected => {
                    Err(StorageError::other("Session is already connected"))
                }
            }
        } else {
            // 尝试从存储加载
            match self.storage.load_session(session_id).await? {
                Some(session) if session.metadata.state == SessionState::Disconnected => {
                    let (event_tx, event_rx) = mpsc::unbounded_channel();

                    let mut entry = SessionEntry::new(session);
                    entry.connection_state = ConnectionState::Connected;
                    entry.connection_id = Some(new_connection_id.clone());
                    entry.event_tx = Some(event_tx);
                    entry.session.set_state(SessionState::Active);
                    entry.mark_dirty();

                    self.connection_map
                        .insert(new_connection_id, session_id.to_string());
                    self.sessions
                        .insert(session_id.to_string(), RwLock::new(entry));

                    info!("Reconnected session from storage: {}", session_id);
                    Ok(event_rx)
                }
                _ => Err(StorageError::SessionNotFound {
                    id: session_id.to_string(),
                }),
            }
        }
    }

    /// 发送事件到会话
    pub fn send_event(&self, session_id: &str, event: AgentEvent) -> StorageResult<()> {
        if let Some(entry) = self.sessions.get(session_id) {
            let entry = entry.read();
            if let Some(ref tx) = entry.event_tx {
                if tx.send(event).is_err() {
                    return Err(StorageError::other("Failed to send event"));
                }
            }
        }
        Ok(())
    }

    /// 添加消息到会话
    pub async fn add_message(
        &self,
        session_id: &str,
        message: Message,
    ) -> StorageResult<()> {
        if let Some(entry) = self.sessions.get(session_id) {
            let mut entry = entry.write();
            entry.session.add_message(message);
            entry.touch();
            entry.mark_dirty();
            Ok(())
        } else {
            // 不在内存中，直接操作存储
            self.storage.append_message(session_id, &message).await
        }
    }

    /// 追加事件到会话
    pub async fn append_event(
        &self,
        session_id: &str,
        event: AgentEvent,
    ) -> StorageResult<()> {
        // 同时发送到存储和连接的客户端
        self.storage.append_event(session_id, &event).await?;
        self.send_event(session_id, event)?;
        Ok(())
    }

    /// 结束会话
    pub async fn close_session(&self, session_id: &str) -> StorageResult<()> {
        // 断开连接
        self.disconnect_session(session_id).await.ok();

        // 更新状态
        self.storage
            .update_session_state(session_id, SessionState::Closed)
            .await?;

        // 从内存中移除
        self.sessions.remove(session_id);

        info!("Closed session: {}", session_id);
        Ok(())
    }

    /// 删除会话（彻底删除）
    pub async fn delete_session(&self, session_id: &str) -> StorageResult<()> {
        // 断开连接
        self.disconnect_session(session_id).await.ok();

        // 从内存中移除
        self.sessions.remove(session_id);

        // 从存储中删除
        self.storage.delete_session(session_id).await?;

        info!("Deleted session: {}", session_id);
        Ok(())
    }

    /// 列出会话
    pub async fn list_sessions(
        &self,
        filter: &SessionFilter,
    ) -> StorageResult<SessionListResult> {
        self.storage.list_sessions(filter).await
    }

    /// 获取会话历史
    pub async fn get_session_history(&self, session_id: &str) -> StorageResult<Vec<Message>> {
        // 优先从内存获取
        if let Some(entry) = self.sessions.get(session_id) {
            let entry = entry.read();
            return Ok(entry.session.messages.clone());
        }

        // 从存储加载
        self.storage.get_session_history(session_id).await
    }

    /// 获取会话事件历史
    pub async fn get_session_events(&self, session_id: &str) -> StorageResult<Vec<AgentEvent>> {
        self.storage.load_events(session_id).await
    }

    /// 获取会话统计信息
    pub async fn get_stats(&self) -> StorageResult<crate::storage::StorageStats> {
        self.storage.get_stats().await
    }

    /// 获取活跃会话数量
    pub fn active_session_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|e| e.value().read().connection_state == ConnectionState::Connected)
            .count()
    }

    /// 获取内存中缓存的会话数量
    pub fn cached_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 强制保存所有脏会话
    pub async fn flush(&self) -> StorageResult<()> {
        self.auto_save_dirty_sessions().await
    }
}

/// SessionManager 的便捷方法
impl SessionManager {
    /// 获取指定用户的会话列表
    pub async fn list_user_sessions(
        &self,
        user_id: &str,
    ) -> StorageResult<Vec<SessionMetadata>> {
        self.storage.list_user_sessions(user_id).await
    }

    /// 更新会话标题
    pub async fn update_session_title(
        &self,
        session_id: &str,
        title: impl Into<String>,
    ) -> StorageResult<()> {
        if let Some(entry) = self.sessions.get(session_id) {
            let mut entry = entry.write();
            entry.session.metadata.title = Some(title.into());
            entry.touch();
            entry.mark_dirty();
            Ok(())
        } else {
            let mut metadata = match self.storage.get_metadata(session_id).await? {
                Some(m) => m,
                None => {
                    return Err(StorageError::SessionNotFound {
                        id: session_id.to_string(),
                    })
                }
            };
            metadata.title = Some(title.into());
            self.storage.update_metadata(&metadata).await
        }
    }

    /// 检查会话是否活跃（有连接）
    pub fn is_session_active(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .map(|e| e.read().connection_state == ConnectionState::Connected)
            .unwrap_or(false)
    }

    /// 检查会话是否可重连
    pub fn is_session_reconnectable(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .map(|e| {
                let entry = e.read();
                entry.connection_state == ConnectionState::Disconnected
                    && !entry.is_expired(self.config.disconnect_retention_secs)
            })
            .unwrap_or(false)
    }
}
