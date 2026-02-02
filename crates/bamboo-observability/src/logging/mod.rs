//! 结构化日志模块
//!
//! 提供基于 tracing 的结构化日志功能。

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing_subscriber::{
    layer::SubscriberExt,
    reload::{self, Handle},
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

use crate::config::Config;
use crate::error::{ObservabilityError, Result};

/// 日志级别重新加载句柄类型
type ReloadHandle = Handle<EnvFilter, Registry>;

/// 日志管理器
#[derive(Debug)]
pub struct LogManager {
    /// 配置
    config: crate::config::LoggingConfig,
    
    /// 过滤器重新加载句柄
    reload_handle: Option<Arc<RwLock<ReloadHandle>>>,
    
    /// 是否已初始化
    initialized: bool,
}

impl LogManager {
    /// 创建新的日志管理器
    pub async fn new(config: &Config) -> Result<Self> {
        let logging_config = config.logging.clone();
        
        let mut manager = Self {
            config: logging_config,
            reload_handle: None,
            initialized: false,
        };
        
        manager.init().await?;
        
        Ok(manager)
    }

    /// 初始化日志系统
    async fn init(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // 构建环境过滤器
        let filter = self.build_filter()?;
        let (filter, reload_handle) = reload::Layer::new(filter);
        self.reload_handle = Some(Arc::new(RwLock::new(reload_handle)));

        // 创建基础注册表
        let registry = tracing_subscriber::registry().with(filter);

        // 添加输出层
        if self.config.json_format {
            let layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(self.config.include_target)
                .with_thread_ids(self.config.include_thread_id)
                .with_line_number(self.config.include_line_number)
                .with_file(true)
                .with_ansi(self.config.ansi_colors);
            
            registry.with(layer).init();
        } else {
            let layer = tracing_subscriber::fmt::layer()
                .with_target(self.config.include_target)
                .with_thread_ids(self.config.include_thread_id)
                .with_line_number(self.config.include_line_number)
                .with_file(true)
                .with_ansi(self.config.ansi_colors);
            
            registry.with(layer).init();
        }

        self.initialized = true;

        tracing::info!(
            target: "bamboo_observability",
            "Log manager initialized with level: {}",
            self.config.level
        );

        Ok(())
    }

    /// 构建环境过滤器
    fn build_filter(&self) -> Result<EnvFilter> {
        let mut filter = EnvFilter::try_new(&self.config.level)
            .map_err(|e| ObservabilityError::logging(format!("Invalid log level: {}", e)))?;

        // 添加模块级别的过滤器
        for (module, level) in &self.config.module_levels {
            filter = filter.add_directive(
                format!("{}={}", module, level)
                    .parse()
                    .map_err(|e| ObservabilityError::logging(format!("Invalid directive: {}", e)))?
            );
        }

        Ok(filter)
    }

    /// 动态更新日志级别
    pub async fn update_level(&mut self, level: &str) -> Result<()> {
        let new_filter = EnvFilter::try_new(level)
            .map_err(|e| ObservabilityError::logging(format!("Invalid log level: {}", e)))?;

        if let Some(ref handle) = self.reload_handle {
            handle.write().modify(|filter| {
                *filter = new_filter;
            }).map_err(|e| ObservabilityError::logging(format!("Failed to update log level: {}", e)))?;
            
            self.config.level = level.to_string();
            
            tracing::info!(
                target: "bamboo_observability",
                "Log level updated to: {}",
                level
            );
            
            Ok(())
        } else {
            Err(ObservabilityError::logging("Log manager not initialized"))
        }
    }

    /// 获取当前配置
    pub fn config(&self) -> &crate::config::LoggingConfig {
        &self.config
    }

    /// 关闭日志管理器
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!(
            target: "bamboo_observability",
            "Log manager shutting down"
        );
        Ok(())
    }
}

/// 创建带有上下文的 span
pub fn create_request_span(request_id: &str) -> tracing::Span {
    tracing::info_span!(
        "request",
        request_id = %request_id,
    )
}

/// 创建带有会话上下文的 span
pub fn create_session_span(session_id: &str, request_id: Option<&str>) -> tracing::Span {
    if let Some(req_id) = request_id {
        tracing::info_span!(
            "session",
            session_id = %session_id,
            request_id = %req_id,
        )
    } else {
        tracing::info_span!(
            "session",
            session_id = %session_id,
        )
    }
}

/// 创建带有 Agent 上下文的 span
pub fn create_agent_span(agent_id: &str, session_id: Option<&str>) -> tracing::Span {
    if let Some(sess_id) = session_id {
        tracing::info_span!(
            "agent",
            agent_id = %agent_id,
            session_id = %sess_id,
        )
    } else {
        tracing::info_span!(
            "agent",
            agent_id = %agent_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_manager_init() {
        let config = Config::default()
            .with_log_level("debug")
            .with_json_format(false);
        
        let manager = LogManager::new(&config).await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_create_spans() {
        let request_span = create_request_span("req-123");
        assert_eq!(request_span.metadata().unwrap().name(), "request");

        let session_span = create_session_span("sess-456", Some("req-123"));
        assert_eq!(session_span.metadata().unwrap().name(), "session");

        let agent_span = create_agent_span("agent-789", Some("sess-456"));
        assert_eq!(agent_span.metadata().unwrap().name(), "agent");
    }
}
