//! # Storage Error Types
//! 
//! 定义存储系统相关的错误类型。

use thiserror::Error;

/// 存储错误类型
#[derive(Error, Debug)]
pub enum StorageError {
    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化/反序列化错误
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 会话不存在
    #[error("Session not found: {id}")]
    SessionNotFound { id: String },

    /// 会话已存在
    #[error("Session already exists: {id}")]
    SessionAlreadyExists { id: String },

    /// 会话已过期
    #[error("Session expired: {id}")]
    SessionExpired { id: String },

    /// 无效的会话状态
    #[error("Invalid session state: {id}, current: {current}, expected: {expected}")]
    InvalidSessionState {
        id: String,
        current: String,
        expected: String,
    },

    /// 索引错误
    #[error("Index error: {message}")]
    IndexError { message: String },

    /// 存储已满
    #[error("Storage quota exceeded: {used}/{limit}")]
    QuotaExceeded { used: u64, limit: u64 },

    /// 并发冲突
    #[error("Concurrent modification conflict: {id}")]
    ConcurrentModification { id: String },

    /// 配置错误
    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    /// 其他错误
    #[error("Storage error: {message}")]
    Other { message: String },
}

impl StorageError {
    /// 创建其他错误
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }

    /// 创建配置错误
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// 创建索引错误
    pub fn index(message: impl Into<String>) -> Self {
        Self::IndexError {
            message: message.into(),
        }
    }
}

/// 存储结果类型
pub type StorageResult<T> = Result<T, StorageError>;
