//! 统一的错误类型和上下文追踪

use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// 观测性错误类型
#[derive(Debug, thiserror::Error, Clone)]
pub enum ObservabilityError {
    /// 配置错误
    #[error("Configuration error: {message}")]
    Config {
        message: String,
    },

    /// 日志错误
    #[error("Logging error: {message}")]
    Logging {
        message: String,
    },

    /// 指标错误
    #[error("Metrics error: {message}")]
    Metrics {
        message: String,
    },

    /// 健康检查错误
    #[error("Health check error: {message}")]
    Health {
        message: String,
    },

    /// 初始化错误
    #[error("Initialization error: {message}")]
    Init {
        message: String,
    },

    /// 运行时错误
    #[error("Runtime error: {message}")]
    Runtime {
        message: String,
        context: ErrorContext,
    },

    /// IO 错误
    #[error("IO error: {message}")]
    Io {
        message: String,
    },

    /// 序列化/反序列化错误
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
    },
}

impl ObservabilityError {
    /// 创建配置错误
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// 创建日志错误
    pub fn logging(message: impl Into<String>) -> Self {
        Self::Logging {
            message: message.into(),
        }
    }

    /// 创建指标错误
    pub fn metrics(message: impl Into<String>) -> Self {
        Self::Metrics {
            message: message.into(),
        }
    }

    /// 创建健康检查错误
    pub fn health(message: impl Into<String>) -> Self {
        Self::Health {
            message: message.into(),
        }
    }

    /// 创建初始化错误
    pub fn init(message: impl Into<String>) -> Self {
        Self::Init {
            message: message.into(),
        }
    }

    /// 创建运行时错误
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
            context: ErrorContext::default(),
        }
    }

    /// 创建 IO 错误
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// 创建序列化错误
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// 获取错误类别
    pub fn category(&self) -> &'static str {
        match self {
            Self::Config { .. } => "config",
            Self::Logging { .. } => "logging",
            Self::Metrics { .. } => "metrics",
            Self::Health { .. } => "health",
            Self::Init { .. } => "init",
            Self::Runtime { .. } => "runtime",
            Self::Io { .. } => "io",
            Self::Serialization { .. } => "serialization",
        }
    }
}

impl From<std::io::Error> for ObservabilityError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ObservabilityError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}

/// 错误上下文
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// 请求 ID
    pub request_id: Option<String>,
    
    /// 会话 ID
    pub session_id: Option<String>,
    
    /// Agent ID
    pub agent_id: Option<String>,
    
    /// 用户 ID
    pub user_id: Option<String>,
    
    /// 追踪 ID
    pub trace_id: Option<String>,
    
    /// 跨度 ID
    pub span_id: Option<String>,
    
    /// 其他上下文
    pub extra: HashMap<String, String>,
}

impl ErrorContext {
    /// 创建新的错误上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成新的请求 ID
    pub fn with_request_id(mut self) -> Self {
        self.request_id = Some(Uuid::new_v4().to_string());
        self
    }

    /// 设置请求 ID
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// 设置会话 ID
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// 设置 Agent ID
    pub fn agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }

    /// 设置用户 ID
    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    /// 设置追踪 ID
    pub fn trace_id(mut self, id: impl Into<String>) -> Self {
        self.trace_id = Some(id.into());
        self
    }

    /// 设置跨度 ID
    pub fn span_id(mut self, id: impl Into<String>) -> Self {
        self.span_id = Some(id.into());
        self
    }

    /// 添加上下文字段
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// 转换为 JSON
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        
        if let Some(ref id) = self.request_id {
            map.insert("request_id".to_string(), serde_json::json!(id));
        }
        if let Some(ref id) = self.session_id {
            map.insert("session_id".to_string(), serde_json::json!(id));
        }
        if let Some(ref id) = self.agent_id {
            map.insert("agent_id".to_string(), serde_json::json!(id));
        }
        if let Some(ref id) = self.user_id {
            map.insert("user_id".to_string(), serde_json::json!(id));
        }
        if let Some(ref id) = self.trace_id {
            map.insert("trace_id".to_string(), serde_json::json!(id));
        }
        if let Some(ref id) = self.span_id {
            map.insert("span_id".to_string(), serde_json::json!(id));
        }
        
        for (key, value) in &self.extra {
            map.insert(key.clone(), serde_json::json!(value));
        }
        
        serde_json::Value::Object(map)
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = [
            self.request_id.as_ref().map(|id| format!("request_id={}", id)),
            self.session_id.as_ref().map(|id| format!("session_id={}", id)),
            self.agent_id.as_ref().map(|id| format!("agent_id={}", id)),
            self.user_id.as_ref().map(|id| format!("user_id={}", id)),
            self.trace_id.as_ref().map(|id| format!("trace_id={}", id)),
            self.span_id.as_ref().map(|id| format!("span_id={}", id)),
        ]
        .into_iter()
        .flatten()
        .collect();

        write!(f, "[{}]", parts.join(", "))
    }
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, ObservabilityError>;

/// 上下文扩展 trait
pub trait Context<T> {
    /// 添加上下文
    fn context(self, ctx: ErrorContext) -> Result<T>;
    
    /// 添加请求 ID
    fn with_request_id(self, id: impl Into<String>) -> Result<T>;
    
    /// 添加会话 ID
    fn with_session_id(self, id: impl Into<String>) -> Result<T>;
}

impl<T> Context<T> for Result<T> {
    fn context(self, ctx: ErrorContext) -> Result<T> {
        self.map_err(|e| match e {
            ObservabilityError::Runtime { message, .. } => {
                ObservabilityError::Runtime { message, context: ctx }
            }
            _ => e,
        })
    }

    fn with_request_id(self, id: impl Into<String>) -> Result<T> {
        self.context(ErrorContext::new().request_id(id))
    }

    fn with_session_id(self, id: impl Into<String>) -> Result<T> {
        self.context(ErrorContext::new().session_id(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ObservabilityError::config("Invalid port");
        assert!(matches!(err, ObservabilityError::Config { .. }));
        assert_eq!(err.category(), "config");
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_request_id()
            .session_id("session-123")
            .agent_id("agent-456");
        
        assert!(ctx.request_id.is_some());
        assert_eq!(ctx.session_id, Some("session-123".to_string()));
        assert_eq!(ctx.agent_id, Some("agent-456".to_string()));
    }

    #[test]
    fn test_error_context_json() {
        let ctx = ErrorContext::new()
            .request_id("req-123")
            .session_id("sess-456");
        
        let json = ctx.to_json();
        assert_eq!(json["request_id"], "req-123");
        assert_eq!(json["session_id"], "sess-456");
    }
}
