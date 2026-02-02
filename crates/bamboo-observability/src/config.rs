//! 配置管理模块
//!
//! 支持从环境变量、配置文件读取观测性配置。

pub mod bamboo_integration;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 观测性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 应用名称
    #[serde(default = "default_app_name")]
    pub app_name: String,
    
    /// 环境（development, staging, production）
    #[serde(default = "default_environment")]
    pub environment: String,
    
    /// 日志配置
    #[serde(default)]
    pub logging: LoggingConfig,
    
    /// 指标配置
    #[serde(default)]
    pub metrics: MetricsConfig,
    
    /// 健康检查配置
    #[serde(default)]
    pub health: HealthConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: default_app_name(),
            environment: default_environment(),
            logging: LoggingConfig::default(),
            metrics: MetricsConfig::default(),
            health: HealthConfig::default(),
        }
    }
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> crate::error::Result<Self> {
        // Simplified: just return default for now
        Ok(Self::default())
    }

    /// 从文件加载配置
    pub fn from_file(path: impl Into<PathBuf>) -> crate::error::Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        
        let config: Config = match path.extension().and_then(|e| e.to_str()) {
            Some("json") => serde_json::from_str(&content)
                .map_err(|e| crate::error::ObservabilityError::serialization(e.to_string()))?,
            Some("toml") => toml::from_str(&content)
                .map_err(|e| crate::error::ObservabilityError::serialization(e.to_string()))?,
            _ => return Err(crate::error::ObservabilityError::config("Unsupported config file format")),
        };
        
        Ok(config)
    }

    /// 设置日志级别
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.logging.level = level.into();
        self
    }

    /// 设置是否使用 JSON 格式
    pub fn with_json_format(mut self, json: bool) -> Self {
        self.logging.json_format = json;
        self
    }

    /// 设置日志文件路径
    pub fn with_log_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.logging.file = true;
        self.logging.file_path = Some(path.into());
        self
    }

    /// 设置指标服务器端口
    pub fn with_metrics_port(mut self, port: u16) -> Self {
        self.metrics.port = port;
        self
    }

    /// 设置健康检查服务器端口
    pub fn with_health_port(mut self, port: u16) -> Self {
        self.health.port = port;
        self
    }

    /// 添加模块特定的日志级别
    pub fn with_module_level(mut self, module: impl Into<String>, level: impl Into<String>) -> Self {
        self.logging.module_levels.insert(module.into(), level.into());
        self
    }
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别（trace, debug, info, warn, error）
    #[serde(default = "default_log_level")]
    pub level: String,
    
    /// 是否使用 JSON 格式
    #[serde(default = "default_false")]
    pub json_format: bool,
    
    /// 是否输出到 stdout
    #[serde(default = "default_true")]
    pub stdout: bool,
    
    /// 是否输出到文件
    #[serde(default = "default_false")]
    pub file: bool,
    
    /// 日志文件路径
    #[serde(default)]
    pub file_path: Option<PathBuf>,
    
    /// 模块级别的日志配置
    #[serde(default)]
    pub module_levels: HashMap<String, String>,
    
    /// 是否启用 ANSI 颜色
    #[serde(default = "default_true")]
    pub ansi_colors: bool,
    
    /// 是否包含目标（target）
    #[serde(default = "default_true")]
    pub include_target: bool,
    
    /// 是否包含线程 ID
    #[serde(default = "default_false")]
    pub include_thread_id: bool,
    
    /// 是否包含行号
    #[serde(default = "default_true")]
    pub include_line_number: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: false,
            stdout: true,
            file: false,
            file_path: None,
            module_levels: HashMap::new(),
            ansi_colors: true,
            include_target: true,
            include_thread_id: false,
            include_line_number: true,
        }
    }
}

/// 指标配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// 是否启用指标收集
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// 指标服务器监听地址
    #[serde(default = "default_metrics_host")]
    pub host: String,
    
    /// 指标服务器端口
    #[serde(default = "default_metrics_port")]
    pub port: u16,
    
    /// 指标路径前缀
    #[serde(default = "default_metrics_prefix")]
    pub prefix: String,
    
    /// 是否启用 Prometheus 导出器
    #[serde(default = "default_true")]
    pub prometheus_enabled: bool,
    
    /// 标签
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_metrics_host(),
            port: default_metrics_port(),
            prefix: default_metrics_prefix(),
            prometheus_enabled: true,
            labels: HashMap::new(),
        }
    }
}

/// 健康检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// 是否启用健康检查服务器
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// 监听地址
    #[serde(default = "default_health_host")]
    pub host: String,
    
    /// 监听端口
    #[serde(default = "default_health_port")]
    pub port: u16,
    
    /// 健康检查端点路径
    #[serde(default = "default_health_path")]
    pub health_path: String,
    
    /// 就绪检查端点路径
    #[serde(default = "default_ready_path")]
    pub ready_path: String,
    
    /// 指标端点路径
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    
    /// 超时时间（秒）
    #[serde(default = "default_health_timeout")]
    pub timeout_seconds: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_health_host(),
            port: default_health_port(),
            health_path: default_health_path(),
            ready_path: default_ready_path(),
            metrics_path: default_metrics_path(),
            timeout_seconds: default_health_timeout(),
        }
    }
}

// 默认值函数
fn default_app_name() -> String {
    "bamboo".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_log_level() -> String {
    std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
}

fn default_metrics_host() -> String {
    "0.0.0.0".to_string()
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_metrics_prefix() -> String {
    "bamboo".to_string()
}

fn default_health_host() -> String {
    "0.0.0.0".to_string()
}

fn default_health_port() -> u16 {
    8080
}

fn default_health_path() -> String {
    "/health".to_string()
}

fn default_ready_path() -> String {
    "/ready".to_string()
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_health_timeout() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.app_name, "bamboo");
        assert_eq!(config.environment, "development");
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.metrics.port, 9090);
        assert_eq!(config.health.port, 8080);
    }

    #[test]
    fn test_config_builder() {
        let config = Config::default()
            .with_log_level("debug")
            .with_json_format(true)
            .with_metrics_port(9091)
            .with_health_port(8081);
        
        assert_eq!(config.logging.level, "debug");
        assert!(config.logging.json_format);
        assert_eq!(config.metrics.port, 9091);
        assert_eq!(config.health.port, 8081);
    }

    #[test]
    fn test_module_levels() {
        let config = Config::default()
            .with_module_level("bamboo_server", "debug")
            .with_module_level("bamboo_gateway", "warn");
        
        assert_eq!(
            config.logging.module_levels.get("bamboo_server"),
            Some(&"debug".to_string())
        );
        assert_eq!(
            config.logging.module_levels.get("bamboo_gateway"),
            Some(&"warn".to_string())
        );
    }
}
