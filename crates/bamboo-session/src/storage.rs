//! # Storage Traits
//! 
//! 定义存储系统的核心 trait，包括基础存储、Session 存储和事件存储。

use async_trait::async_trait;

use crate::error::StorageResult;
use crate::types::{
    AgentEvent, Message, Session, SessionFilter, SessionListResult, SessionMetadata,
};

/// 基础存储 trait
/// 
/// 定义所有存储实现必须提供的基本操作。
#[async_trait]
pub trait Storage: Send + Sync {
    /// 保存会话（完整替换）
    async fn save_session(&self, session: &Session) -> StorageResult<()>;

    /// 加载会话
    async fn load_session(&self, session_id: &str) -> StorageResult<Option<Session>>;

    /// 删除会话
    async fn delete_session(&self, session_id: &str) -> StorageResult<()>;

    /// 检查会话是否存在
    async fn session_exists(&self, session_id: &str) -> StorageResult<bool>;

    /// 追加消息到会话
    async fn append_message(&self, session_id: &str, message: &Message) -> StorageResult<()>;

    /// 追加事件到会话事件流
    async fn append_event(&self, session_id: &str, event: &AgentEvent) -> StorageResult<()>;

    /// 加载会话的所有事件
    async fn load_events(&self, session_id: &str) -> StorageResult<Vec<AgentEvent>>;

    /// 获取存储统计信息
    async fn get_stats(&self) -> StorageResult<StorageStats>;

    /// 健康检查
    async fn health_check(&self) -> StorageResult<()>;
}

/// 存储统计信息
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// 总会话数
    pub total_sessions: u64,
    /// 活跃会话数
    pub active_sessions: u64,
    /// 总消息数
    pub total_messages: u64,
    /// 存储大小（字节）
    pub storage_size_bytes: u64,
    /// 索引大小（字节）
    pub index_size_bytes: u64,
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            active_sessions: 0,
            total_messages: 0,
            storage_size_bytes: 0,
            index_size_bytes: 0,
        }
    }
}

/// Session 存储 trait（增强版）
/// 
/// 在基础 Storage 之上提供更高级的 Session 管理功能。
#[async_trait]
pub trait SessionStorage: Storage {
    /// 创建新会话
    async fn create_session(&self, session: &Session) -> StorageResult<()>;

    /// 更新会话元数据
    async fn update_metadata(&self, metadata: &SessionMetadata) -> StorageResult<()>;

    /// 获取会话元数据
    async fn get_metadata(&self, session_id: &str) -> StorageResult<Option<SessionMetadata>>;

    /// 列出会话（支持过滤和分页）
    async fn list_sessions(&self, filter: &SessionFilter) -> StorageResult<SessionListResult>;

    /// 获取用户的所有会话
    async fn list_user_sessions(&self, user_id: &str) -> StorageResult<Vec<SessionMetadata>> {
        self.list_sessions(&SessionFilter::new().with_user_id(user_id))
            .await
            .map(|result| result.sessions)
    }

    /// 获取会话历史（仅消息，不包含事件）
    async fn get_session_history(&self, session_id: &str) -> StorageResult<Vec<Message>>;

    /// 批量删除过期会话
    /// 
    /// 返回删除的会话数量
    async fn cleanup_expired_sessions(&self) -> StorageResult<u64>;

    /// 批量删除旧会话（按最后活动时间）
    /// 
    /// 返回删除的会话数量
    async fn cleanup_old_sessions(&self, before: chrono::DateTime<chrono::Utc>) -> StorageResult<u64>;

    /// 更新会话状态
    async fn update_session_state(
        &self,
        session_id: &str,
        state: crate::types::SessionState,
    ) -> StorageResult<()>;

    /// 更新最后活动时间
    async fn touch_session(&self, session_id: &str) -> StorageResult<()>;
}

/// 事件存储 trait（专门用于 AgentEvent）
/// 
/// 提供事件流的存储和查询功能。
#[async_trait]
pub trait EventStorage: Send + Sync {
    /// 追加事件
    async fn append_event(&self, session_id: &str, event: &AgentEvent) -> StorageResult<()>;

    /// 加载会话的所有事件
    async fn load_events(&self, session_id: &str) -> StorageResult<Vec<AgentEvent>>;

    /// 加载会话的最近 N 个事件
    async fn load_recent_events(
        &self,
        session_id: &str,
        limit: usize,
    ) -> StorageResult<Vec<AgentEvent>>;

    /// 加载特定类型的事件
    async fn load_events_by_type(
        &self,
        session_id: &str,
        event_type: &str,
    ) -> StorageResult<Vec<AgentEvent>>;

    /// 删除会话的所有事件
    async fn delete_session_events(&self, session_id: &str) -> StorageResult<()>;

    /// 获取事件流统计
    async fn get_event_stats(&self, session_id: &str) -> StorageResult<EventStats>;
}

/// 事件统计
#[derive(Debug, Clone)]
pub struct EventStats {
    pub total_events: u64,
    pub token_events: u64,
    pub tool_start_events: u64,
    pub tool_complete_events: u64,
    pub error_events: u64,
}

/// 索引存储 trait
/// 
/// 提供 Session 索引功能，支持快速查询。
#[async_trait]
pub trait IndexStorage: Send + Sync {
    /// 添加会话到索引
    async fn index_session(&self, metadata: &SessionMetadata) -> StorageResult<()>;

    /// 从索引中移除会话
    async fn remove_from_index(&self, session_id: &str) -> StorageResult<()>;

    /// 按用户ID查询
    async fn query_by_user(&self, user_id: &str) -> StorageResult<Vec<String>>;

    /// 按状态查询
    async fn query_by_state(
        &self,
        state: crate::types::SessionState,
    ) -> StorageResult<Vec<String>>;

    /// 按时间范围查询
    async fn query_by_time_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> StorageResult<Vec<String>>;

    /// 重建索引
    async fn rebuild_index(&self) -> StorageResult<()>;

    /// 清理索引（移除指向不存在会话的条目）
    async fn cleanup_index(&self) -> StorageResult<u64>;
}
