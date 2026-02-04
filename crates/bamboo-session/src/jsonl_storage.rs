//! # JsonlStorage Implementation
//! 
//! 基于 JSONL 文件的 Session 持久化存储实现。
//! 
//! 存储结构:
//! ```
//! <base_path>/
//! ├── sessions/
//! │   ├── <session_id>.json      # 会话元数据和消息
//! │   └── ...
//! ├── events/
//! │   ├── <session_id>.jsonl     # 事件流(追加写入)
//! │   └── ...
//! └── index/
//!     ├── by_user.json           # 用户索引
//!     ├── by_time.json           # 时间索引
//!     └── by_state.json          # 状态索引
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

use crate::error::{StorageError, StorageResult};
use crate::storage::{IndexStorage, SessionStorage, Storage, StorageStats};
use crate::types::{
    AgentEvent, Message, Session, SessionFilter, SessionListResult, SessionMetadata, SessionState,
    SortField, SortOrder,
};

/// JsonlStorage 配置
#[derive(Debug, Clone)]
pub struct JsonlStorageConfig {
    /// 存储根目录
    pub base_path: PathBuf,
    /// 自动清理间隔(秒)，None 表示不自动清理
    pub cleanup_interval_secs: Option<u64>,
    /// 默认会话过期时间(秒)，None 表示永不过期
    pub default_ttl_secs: Option<u64>,
    /// 最大会话数限制
    pub max_sessions: Option<u64>,
    /// 启用索引
    pub enable_index: bool,
}

impl JsonlStorageConfig {
    /// 创建默认配置
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            cleanup_interval_secs: Some(3600), // 默认1小时
            default_ttl_secs: None,
            max_sessions: None,
            enable_index: true,
        }
    }

    /// 设置自动清理间隔
    pub fn with_cleanup_interval(mut self, secs: u64) -> Self {
        self.cleanup_interval_secs = Some(secs);
        self
    }

    /// 设置默认 TTL
    pub fn with_default_ttl(mut self, secs: u64) -> Self {
        self.default_ttl_secs = Some(secs);
        self
    }

    /// 设置最大会话数
    pub fn with_max_sessions(mut self, max: u64) -> Self {
        self.max_sessions = Some(max);
        self
    }

    /// 禁用索引
    pub fn disable_index(mut self) -> Self {
        self.enable_index = false;
        self
    }
}

impl Default for JsonlStorageConfig {
    fn default() -> Self {
        Self::new("~/.bamboo/sessions")
    }
}

/// 内存索引结构
#[derive(Debug, Default)]
struct SessionIndex {
    /// 用户索引: user_id -> Vec<session_id>
    by_user: HashMap<String, Vec<String>>,
    /// 状态索引: state -> Vec<session_id>
    by_state: HashMap<SessionState, Vec<String>>,
    /// 时间索引: 按创建时间排序的会话列表
    by_time: Vec<(DateTime<Utc>, String)>,
    /// 所有会话元数据缓存
    metadata_cache: HashMap<String, SessionMetadata>,
}

impl SessionIndex {
    /// 添加会话到索引
    fn add(&mut self, metadata: &SessionMetadata) {
        let id = metadata.id.clone();

        // 更新用户索引
        if let Some(ref user_id) = metadata.user_id {
            self.by_user
                .entry(user_id.clone())
                .or_default()
                .push(id.clone());
        }

        // 更新状态索引
        self.by_state
            .entry(metadata.state)
            .or_default()
            .push(id.clone());

        // 更新时间索引
        self.by_time.push((metadata.created_at, id.clone()));
        // 保持时间索引有序（降序）
        self.by_time.sort_by(|a, b| b.0.cmp(&a.0));

        // 更新缓存
        self.metadata_cache.insert(id, metadata.clone());
    }

    /// 从索引中移除会话
    fn remove(&mut self, session_id: &str) {
        if let Some(metadata) = self.metadata_cache.remove(session_id) {
            // 从用户索引中移除
            if let Some(ref user_id) = metadata.user_id {
                if let Some(sessions) = self.by_user.get_mut(user_id) {
                    sessions.retain(|id| id != session_id);
                    if sessions.is_empty() {
                        self.by_user.remove(user_id);
                    }
                }
            }

            // 从状态索引中移除
            if let Some(sessions) = self.by_state.get_mut(&metadata.state) {
                sessions.retain(|id| id != session_id);
                if sessions.is_empty() {
                    self.by_state.remove(&metadata.state);
                }
            }

            // 从时间索引中移除
            self.by_time.retain(|(_, id)| id != session_id);
        }
    }

    /// 更新会话元数据
    fn update(&mut self, metadata: &SessionMetadata) {
        self.remove(&metadata.id);
        self.add(metadata);
    }

    /// 根据过滤器查询
    fn query(&self, filter: &SessionFilter) -> Vec<SessionMetadata> {
        let mut result: Vec<_> = self.metadata_cache.values().cloned().collect();

        // 应用过滤条件
        if let Some(ref user_id) = filter.user_id {
            result.retain(|m| m.user_id.as_ref() == Some(user_id));
        }

        if let Some(state) = filter.state {
            result.retain(|m| m.state == state);
        }

        if let Some(after) = filter.created_after {
            result.retain(|m| m.created_at >= after);
        }

        if let Some(before) = filter.created_before {
            result.retain(|m| m.created_at <= before);
        }

        if let Some(after) = filter.active_after {
            result.retain(|m| m.last_activity_at >= after);
        }

        if let Some(ref keyword) = filter.title_contains {
            result.retain(|m| {
                m.title
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&keyword.to_lowercase()))
                    .unwrap_or(false)
            });
        }

        // 排序
        match filter.sort_by.unwrap_or(SortField::LastActivityAt) {
            SortField::CreatedAt => match filter.sort_order.unwrap_or(SortOrder::Desc) {
                SortOrder::Asc => result.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
                SortOrder::Desc => result.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            },
            SortField::UpdatedAt => match filter.sort_order.unwrap_or(SortOrder::Desc) {
                SortOrder::Asc => result.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
                SortOrder::Desc => result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            },
            SortField::LastActivityAt => {
                match filter.sort_order.unwrap_or(SortOrder::Desc) {
                    SortOrder::Asc => {
                        result.sort_by(|a, b| a.last_activity_at.cmp(&b.last_activity_at))
                    }
                    SortOrder::Desc => {
                        result.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at))
                    }
                }
            }
            SortField::MessageCount => match filter.sort_order.unwrap_or(SortOrder::Desc) {
                SortOrder::Asc => {
                    result.sort_by(|a, b| a.message_count.cmp(&b.message_count))
                }
                SortOrder::Desc => {
                    result.sort_by(|a, b| b.message_count.cmp(&a.message_count))
                }
            },
        }

        result
    }

    /// 获取过期的会话ID
    fn get_expired_ids(&self) -> Vec<String> {
        self.metadata_cache
            .values()
            .filter(|m| m.is_expired())
            .map(|m| m.id.clone())
            .collect()
    }

    /// 获取在指定时间之前最后活动的会话ID
    fn get_inactive_ids(&self, before: DateTime<Utc>) -> Vec<String> {
        self.metadata_cache
            .values()
            .filter(|m| m.last_activity_at < before)
            .map(|m| m.id.clone())
            .collect()
    }
}

/// JsonlStorage 实现
pub struct JsonlStorage {
    config: JsonlStorageConfig,
    index: Arc<RwLock<SessionIndex>>,
    sessions_path: PathBuf,
    events_path: PathBuf,
    index_path: PathBuf,
}

impl JsonlStorage {
    /// 创建新的 JsonlStorage 实例
    pub async fn new(config: JsonlStorageConfig) -> StorageResult<Self> {
        let base_path_str = config.base_path.to_string_lossy().to_string();
        let base_path = shellexpand::tilde(&base_path_str);
        let base_path = PathBuf::from(base_path.as_ref());

        let sessions_path = base_path.join("sessions");
        let events_path = base_path.join("events");
        let index_path = base_path.join("index");

        // 创建目录
        fs::create_dir_all(&sessions_path).await?;
        fs::create_dir_all(&events_path).await?;
        fs::create_dir_all(&index_path).await?;

        let storage = Self {
            config,
            index: Arc::new(RwLock::new(SessionIndex::default())),
            sessions_path,
            events_path,
            index_path,
        };

        // 加载现有会话到索引
        if storage.config.enable_index {
            storage.rebuild_index_internal().await?;
        }

        info!(
            "JsonlStorage initialized at {:?}",
            storage.config.base_path
        );

        Ok(storage)
    }

    /// 获取会话文件路径
    fn session_file_path(&self, session_id: &str) -> PathBuf {
        self.sessions_path.join(format!("{}.json", session_id))
    }

    /// 获取事件文件路径
    fn event_file_path(&self, session_id: &str) -> PathBuf {
        self.events_path.join(format!("{}.jsonl", session_id))
    }

    /// 保存元数据到文件
    async fn save_metadata_file(&self, metadata: &SessionMetadata) -> StorageResult<()> {
        let path = self.session_file_path(&metadata.id);
        let content = serde_json::to_string_pretty(metadata)?;
        fs::write(&path, content).await?;
        Ok(())
    }

    /// 从文件加载元数据
    async fn load_metadata_file(&self, session_id: &str) -> StorageResult<Option<SessionMetadata>> {
        let path = self.session_file_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).await?;
        let metadata: SessionMetadata = serde_json::from_str(&content)?;
        Ok(Some(metadata))
    }

    /// 追加事件到文件
    async fn append_event_to_file(
        &self,
        session_id: &str,
        event: &AgentEvent,
    ) -> StorageResult<()> {
        let path = self.event_file_path(session_id);
        let line = serde_json::to_string(event)?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    /// 从文件加载事件
    async fn load_events_from_file(&self, session_id: &str) -> StorageResult<Vec<AgentEvent>> {
        let path = self.event_file_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path).await?;
        let mut events = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AgentEvent>(line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    warn!("Failed to parse event line: {}", e);
                }
            }
        }

        Ok(events)
    }

    /// 删除会话的所有数据
    async fn delete_session_data(&self, session_id: &str) -> StorageResult<()> {
        // 删除会话文件
        let session_path = self.session_file_path(session_id);
        if session_path.exists() {
            fs::remove_file(&session_path).await?;
        }

        // 删除事件文件
        let event_path = self.event_file_path(session_id);
        if event_path.exists() {
            fs::remove_file(&event_path).await?;
        }

        // 从索引中移除
        self.index.write().remove(session_id);

        debug!("Deleted session data: {}", session_id);
        Ok(())
    }

    /// 重建索引（内部实现）
    async fn rebuild_index_internal(&self) -> StorageResult<()> {
        let mut entries = fs::read_dir(&self.sessions_path).await?;
        let mut count = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let content = fs::read_to_string(&path).await?;
            match serde_json::from_str::<SessionMetadata>(&content) {
                Ok(metadata) => {
                    self.index.write().add(&metadata);
                    count += 1;
                }
                Err(e) => {
                    warn!("Failed to parse session metadata: {}", e);
                }
            }
        }

        info!("Rebuilt index with {} sessions", count);
        Ok(())
    }
}

#[async_trait]
impl Storage for JsonlStorage {
    async fn save_session(&self, session: &Session) -> StorageResult<()> {
        let path = self.session_file_path(&session.metadata.id);
        let content = serde_json::to_string_pretty(session)?;
        fs::write(&path, content).await?;

        // 更新索引
        if self.config.enable_index {
            self.index.write().update(&session.metadata);
        }

        debug!("Saved session: {}", session.metadata.id);
        Ok(())
    }

    async fn load_session(&self, session_id: &str) -> StorageResult<Option<Session>> {
        let path = self.session_file_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).await?;
        let session: Session = serde_json::from_str(&content)?;

        // 检查是否过期
        if session.metadata.is_expired() {
            return Err(StorageError::SessionExpired {
                id: session_id.to_string(),
            });
        }

        Ok(Some(session))
    }

    async fn delete_session(&self, session_id: &str) -> StorageResult<()> {
        if !self.session_exists(session_id).await? {
            return Err(StorageError::SessionNotFound {
                id: session_id.to_string(),
            });
        }

        self.delete_session_data(session_id).await?;
        info!("Deleted session: {}", session_id);
        Ok(())
    }

    async fn session_exists(&self, session_id: &str) -> StorageResult<bool> {
        let path = self.session_file_path(session_id);
        Ok(path.exists())
    }

    async fn append_message(&self, session_id: &str, message: &Message) -> StorageResult<()> {
        let mut session = match self.load_session(session_id).await? {
            Some(s) => s,
            None => {
                return Err(StorageError::SessionNotFound {
                    id: session_id.to_string(),
                })
            }
        };

        session.add_message(message.clone());
        session.metadata.touch();

        self.save_session(&session).await?;
        Ok(())
    }

    async fn append_event(&self, session_id: &str, event: &AgentEvent) -> StorageResult<()> {
        // 检查会话是否存在
        if !self.session_exists(session_id).await? {
            return Err(StorageError::SessionNotFound {
                id: session_id.to_string(),
            });
        }

        // 追加到事件文件
        self.append_event_to_file(session_id, event).await?;

        debug!("Appended event to session: {}", session_id);
        Ok(())
    }

    async fn load_events(&self, session_id: &str) -> StorageResult<Vec<AgentEvent>> {
        self.load_events_from_file(session_id).await
    }

    async fn get_stats(&self) -> StorageResult<StorageStats> {
        // 计算存储大小（先获取文件系统数据）
        let mut storage_size = 0u64;
        let mut entries = fs::read_dir(&self.sessions_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            storage_size += metadata.len();
        }

        let mut event_size = 0u64;
        let mut entries = fs::read_dir(&self.events_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            event_size += metadata.len();
        }

        // 再从索引获取数据（短暂持有锁）
        let (total_sessions, active_count, total_messages) = {
            let index = self.index.read();
            let active_count = index
                .metadata_cache
                .values()
                .filter(|m| m.state == SessionState::Active)
                .count() as u64;
            let total_messages: u32 = index
                .metadata_cache
                .values()
                .map(|m| m.message_count)
                .sum();
            (index.metadata_cache.len() as u64, active_count, total_messages as u64)
        };

        Ok(StorageStats {
            total_sessions,
            active_sessions: active_count,
            total_messages,
            storage_size_bytes: storage_size + event_size,
            index_size_bytes: 0, // 内存索引，无文件大小
        })
    }

    async fn health_check(&self) -> StorageResult<()> {
        // 检查目录是否可写
        let test_file = self.config.base_path.join(".health_check");
        fs::write(&test_file, "ok").await?;
        fs::remove_file(&test_file).await?;
        Ok(())
    }
}

#[async_trait]
impl SessionStorage for JsonlStorage {
    async fn create_session(&self, session: &Session) -> StorageResult<()> {
        // 检查是否已存在
        if self.session_exists(&session.metadata.id).await? {
            return Err(StorageError::SessionAlreadyExists {
                id: session.metadata.id.clone(),
            });
        }

        // 检查最大会话数限制
        if let Some(max) = self.config.max_sessions {
            let stats = self.get_stats().await?;
            if stats.total_sessions >= max {
                return Err(StorageError::QuotaExceeded {
                    used: stats.total_sessions,
                    limit: max,
                });
            }
        }

        // 应用默认 TTL
        let mut session = session.clone();
        if session.metadata.expires_at.is_none() {
            if let Some(ttl) = self.config.default_ttl_secs {
                session.metadata.expires_at =
                    Some(session.metadata.created_at + chrono::Duration::seconds(ttl as i64));
            }
        }

        self.save_session(&session).await?;
        
        // 添加到索引
        if self.config.enable_index {
            self.index.write().add(&session.metadata);
        }

        info!("Created session: {}", session.metadata.id);
        Ok(())
    }

    async fn update_metadata(&self, metadata: &SessionMetadata) -> StorageResult<()> {
        let mut session = match self.load_session(&metadata.id).await? {
            Some(s) => s,
            None => {
                return Err(StorageError::SessionNotFound {
                    id: metadata.id.clone(),
                })
            }
        };

        // 保留消息，只更新元数据
        session.metadata = metadata.clone();
        session.metadata.updated_at = Utc::now();

        self.save_session(&session).await?;
        Ok(())
    }

    async fn get_metadata(&self, session_id: &str) -> StorageResult<Option<SessionMetadata>> {
        // 优先从索引获取
        if self.config.enable_index {
            if let Some(metadata) = self.index.read().metadata_cache.get(session_id).cloned() {
                return Ok(Some(metadata));
            }
        }

        // 从文件加载
        self.load_metadata_file(session_id).await
    }

    async fn list_sessions(&self, filter: &SessionFilter) -> StorageResult<SessionListResult> {
        if !self.config.enable_index {
            return Err(StorageError::index("Index is disabled"));
        }

        let index = self.index.read();
        let all = index.query(filter);
        let total = all.len();

        // 应用分页
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(100);
        
        let sessions: Vec<_> = all
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(SessionListResult::new(sessions, total, offset, limit))
    }

    async fn get_session_history(&self, session_id: &str) -> StorageResult<Vec<Message>> {
        match self.load_session(session_id).await? {
            Some(session) => Ok(session.messages),
            None => Err(StorageError::SessionNotFound {
                id: session_id.to_string(),
            }),
        }
    }

    async fn cleanup_expired_sessions(&self) -> StorageResult<u64> {
        if !self.config.enable_index {
            return Ok(0);
        }

        let expired_ids: Vec<_> = self.index.read().get_expired_ids();
        let count = expired_ids.len() as u64;

        for id in expired_ids {
            if let Err(e) = self.delete_session_data(&id).await {
                error!("Failed to delete expired session {}: {}", id, e);
            }
        }

        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }

        Ok(count)
    }

    async fn cleanup_old_sessions(
        &self,
        before: DateTime<Utc>,
    ) -> StorageResult<u64> {
        if !self.config.enable_index {
            return Ok(0);
        }

        let inactive_ids: Vec<_> = self.index.read().get_inactive_ids(before);
        let count = inactive_ids.len() as u64;

        for id in inactive_ids {
            if let Err(e) = self.delete_session_data(&id).await {
                error!("Failed to delete inactive session {}: {}", id, e);
            }
        }

        if count > 0 {
            info!("Cleaned up {} old sessions (before {})", count, before);
        }

        Ok(count)
    }

    async fn update_session_state(
        &self,
        session_id: &str,
        state: SessionState,
    ) -> StorageResult<()> {
        let mut session = match self.load_session(session_id).await? {
            Some(s) => s,
            None => {
                return Err(StorageError::SessionNotFound {
                    id: session_id.to_string(),
                })
            }
        };

        session.set_state(state);
        self.save_session(&session).await?;

        info!("Updated session {} state to {:?}", session_id, state);
        Ok(())
    }

    async fn touch_session(&self, session_id: &str) -> StorageResult<()> {
        let mut session = match self.load_session(session_id).await? {
            Some(s) => s,
            None => {
                return Err(StorageError::SessionNotFound {
                    id: session_id.to_string(),
                })
            }
        };

        session.metadata.touch();
        self.save_session(&session).await?;
        Ok(())
    }
}

#[async_trait]
impl IndexStorage for JsonlStorage {
    async fn index_session(&self, metadata: &SessionMetadata) -> StorageResult<()> {
        if !self.config.enable_index {
            return Ok(());
        }
        self.index.write().add(metadata);
        Ok(())
    }

    async fn remove_from_index(&self, session_id: &str) -> StorageResult<()> {
        if !self.config.enable_index {
            return Ok(());
        }
        self.index.write().remove(session_id);
        Ok(())
    }

    async fn query_by_user(&self, user_id: &str) -> StorageResult<Vec<String>> {
        if !self.config.enable_index {
            return Err(StorageError::index("Index is disabled"));
        }
        
        let index = self.index.read();
        let ids = index
            .by_user
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids)
    }

    async fn query_by_state(&self, state: SessionState) -> StorageResult<Vec<String>> {
        if !self.config.enable_index {
            return Err(StorageError::index("Index is disabled"));
        }
        
        let index = self.index.read();
        let ids = index
            .by_state
            .get(&state)
            .cloned()
            .unwrap_or_default();
        Ok(ids)
    }

    async fn query_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<Vec<String>> {
        if !self.config.enable_index {
            return Err(StorageError::index("Index is disabled"));
        }
        
        let index = self.index.read();
        let ids: Vec<_> = index
            .by_time
            .iter()
            .filter(|(time, _)| *time >= start && *time <= end)
            .map(|(_, id)| id.clone())
            .collect();
        Ok(ids)
    }

    async fn rebuild_index(&self) -> StorageResult<()> {
        self.rebuild_index_internal().await
    }

    async fn cleanup_index(&self) -> StorageResult<u64> {
        // 检查索引中是否有过期的引用
        let to_remove: Vec<_> = {
            let index = self.index.read();
            index
                .metadata_cache
                .keys()
                .filter(|id| !self.session_file_path(id).exists())
                .cloned()
                .collect()
        };

        let count = to_remove.len() as u64;
        for id in to_remove {
            self.index.write().remove(&id);
        }

        if count > 0 {
            info!("Cleaned up {} stale index entries", count);
        }

        Ok(count)
    }
}
