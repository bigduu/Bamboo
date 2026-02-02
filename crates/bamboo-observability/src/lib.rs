//! Bamboo Observability Infrastructure
//!
//! 提供统一的日志、指标和健康检查功能。

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod health;
pub mod logging;
pub mod metrics;

pub use config::Config;
pub use error::{Context, ObservabilityError, Result};
pub use health::{HealthCheck, HealthStatus, HealthServer};
pub use logging::LogManager;
pub use metrics::MetricsCollector;

use std::sync::Arc;
use parking_lot::RwLock;

/// 统一的观测性句柄
#[derive(Debug)]
pub struct Observability {
    /// 日志管理器
    log_manager: Arc<RwLock<LogManager>>,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 健康检查服务器
    health_server: Arc<HealthServer>,
    /// 配置
    config: Config,
}

impl Observability {
    /// 初始化观测性基础设施
    pub async fn init(config: Config) -> Result<Self> {
        // 初始化日志系统
        let log_manager = Arc::new(RwLock::new(
            LogManager::new(&config).await?
        ));
        
        // 初始化指标收集器
        let metrics = Arc::new(
            MetricsCollector::new(&config).await?
        );
        
        // 初始化健康检查服务器
        let health_server = Arc::new(
            HealthServer::new(&config).await?
        );

        tracing::info!(
            target: "bamboo_observability",
            "Observability infrastructure initialized"
        );

        Ok(Self {
            log_manager,
            metrics,
            health_server,
            config,
        })
    }

    /// 获取日志管理器
    pub fn log_manager(&self) -> Arc<RwLock<LogManager>> {
        Arc::clone(&self.log_manager)
    }

    /// 获取指标收集器
    pub fn metrics(&self) -> Arc<MetricsCollector> {
        Arc::clone(&self.metrics)
    }

    /// 获取健康检查服务器
    pub fn health_server(&self) -> Arc<HealthServer> {
        Arc::clone(&self.health_server)
    }

    /// 获取配置
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 动态更新日志级别
    pub async fn update_log_level(&self, level: &str) -> Result<()> {
        let mut manager = self.log_manager.write();
        manager.update_level(level).await
    }

    /// 注册健康检查
    pub async fn register_health_check<C>(&self, name: &str, check: C)
    where
        C: HealthCheck + Send + Sync + 'static,
    {
        self.health_server.register(name, check).await;
    }

    /// 启动健康检查服务器
    pub async fn start_health_server(&self) -> Result<()> {
        self.health_server.start().await
    }

    /// 优雅关闭
    pub async fn shutdown(self) -> Result<()> {
        tracing::info!(
            target: "bamboo_observability",
            "Shutting down observability infrastructure"
        );
        
        self.health_server.shutdown().await?;
        self.metrics.shutdown().await?;
        
        let manager = self.log_manager.write();
        manager.shutdown().await?;
        
        Ok(())
    }
}

/// 便捷导入模块
pub mod prelude {
    //! 常用类型的便捷导入
    
    pub use crate::{Context, Config, Observability, Result};
    
    // 日志
    pub use tracing::{debug, error, info, instrument, trace, warn, Span};
    
    // 指标
    pub use metrics::{counter, gauge, histogram};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_observability_init() {
        let config = Config::default()
            .with_log_level("debug")
            .with_json_format(false);
        
        let obs = Observability::init(config).await;
        assert!(obs.is_ok());
    }
}
