//! # Bamboo Session Storage
//!
//! Bamboo AI Agent 的 Session 持久化存储系统。
//!
//! ## 功能特性
//!
//! - **会话元数据存储**：创建时间、最后活动、用户ID、状态等
//! - **消息历史存储**：完整的对话历史
//! - **事件流存储**：AgentEvent 的追加写入
//! - **多维度索引**：按用户、状态、时间快速查询
//! - **自动清理**：过期会话自动清理
//! - **断线重连**：连接断开后 Session 保持，支持重连
//! - **内存缓存**：活跃会话内存缓存，提高性能
//!
//! ## 存储结构
//!
//! ```
//! <base_path>/
//! ├── sessions/
//! │   ├── <session_id>.json      # 会话元数据和消息
//! │   └── ...
//! ├── events/
//! │   ├── <session_id>.jsonl     # 事件流（追加写入）
//! │   └── ...
//! └── index/
//!     ├── by_user.json           # 用户索引(内存中)
//!     ├── by_time.json           # 时间索引(内存中)
//!     └── by_state.json          # 状态索引(内存中)
//! ```
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use bamboo_session::{
//!     JsonlStorage, JsonlStorageConfig,
//!     SessionManager, SessionManagerConfig,
//!     Session, SessionFilter, SessionState,
//!     Message, AgentEvent,
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 创建存储
//!     let storage_config = JsonlStorageConfig::new("~/.bamboo/sessions")
//!         .with_default_ttl(86400); // 24小时过期
//!     
//!     let storage = Arc::new(JsonlStorage::new(storage_config).await?);
//!     
//!     // 创建 SessionManager
//!     let manager_config = SessionManagerConfig::default();
//!     let manager = SessionManager::new(manager_config, storage).await?;
//!     
//!     // 创建会话
//!     let session = manager.create_session(
//!         Some("user-123".to_string()),
//!         Some("My Chat".to_string()),
//!     ).await?;
//!     
//!     // 添加消息
//!     manager.add_message(
//!         &session.metadata.id,
//!         Message::user("Hello!"),
//!     ).await?;
//!     
//!     // 查询会话
//!     let sessions = manager.list_sessions(
//!         &SessionFilter::new()
//!             .with_user_id("user-123")
//!             .with_state(SessionState::Active)
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod jsonl_storage;
pub mod session_manager;
pub mod storage;
pub mod types;

// 重新导出主要类型
pub use error::{StorageError, StorageResult};
pub use jsonl_storage::{JsonlStorage, JsonlStorageConfig};
pub use session_manager::{SessionManager, SessionManagerConfig};
pub use storage::{
    EventStats, EventStorage, IndexStorage, SessionStorage, Storage, StorageStats,
};
pub use types::{
    AgentEvent, Message, Role, Session, SessionFilter, SessionListResult, SessionMetadata,
    SessionState, SortField, SortOrder, ToolCall, ToolResult, TokenUsage,
};

/// 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 创建默认存储路径
pub fn default_storage_path() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".bamboo").join("sessions"))
        .unwrap_or_else(|| std::path::PathBuf::from("./bamboo_sessions"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_load_session() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = JsonlStorageConfig::new(temp_dir.path());
        let storage = JsonlStorage::new(config).await.unwrap();

        // 创建会话
        let session = Session::new("test-session-1")
            .with_user_id("user-123");

        storage.create_session(&session).await.unwrap();

        // 加载会话
        let loaded = storage.load_session("test-session-1").await.unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.metadata.id, "test-session-1");
        assert_eq!(loaded.metadata.user_id, Some("user-123".to_string()));
    }

    #[tokio::test]
    async fn test_session_filter() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = JsonlStorageConfig::new(temp_dir.path());
        let storage = JsonlStorage::new(config).await.unwrap();

        // 创建多个会话
        for i in 0..5 {
            let session = Session::new(format!("session-{}", i))
                .with_user_id(if i % 2 == 0 { "user-a" } else { "user-b" });
            storage.create_session(&session).await.unwrap();
        }

        // 按用户过滤
        let result = storage
            .list_sessions(&SessionFilter::new().with_user_id("user-a"))
            .await
            .unwrap();

        assert_eq!(result.total, 3); // session-0, 2, 4
    }
}
