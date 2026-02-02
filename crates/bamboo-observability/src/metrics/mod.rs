//! 指标收集模块
//!
//! 提供基于 metrics 库的指标收集功能。

use metrics::{describe_counter, describe_gauge, describe_histogram, Unit};
#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::config::Config;
use crate::error::{ObservabilityError, Result};

/// 指标收集器
pub struct MetricsCollector {
    /// Prometheus 句柄
    #[cfg(feature = "prometheus")]
    handle: Option<PrometheusHandle>,
    
    /// 配置
    config: crate::config::MetricsConfig,
    
    /// 是否已初始化
    initialized: bool,
}

impl std::fmt::Debug for MetricsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsCollector")
            .field("config", &self.config)
            .field("initialized", &self.initialized)
            .finish()
    }
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub async fn new(config: &Config) -> Result<Self> {
        let metrics_config = config.metrics.clone();
        
        let mut collector = Self {
            #[cfg(feature = "prometheus")]
            handle: None,
            config: metrics_config,
            initialized: false,
        };
        
        collector.init().await?;
        
        Ok(collector)
    }

    /// 初始化指标收集器
    async fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        #[cfg(feature = "prometheus")]
        if self.config.prometheus_enabled {
            let builder = PrometheusBuilder::new();
            let recorder = builder.build_recorder();
            let handle = recorder.handle();
            
            metrics::set_global_recorder(recorder).map_err(|e| {
                ObservabilityError::metrics(format!("Failed to set global recorder: {}", e))
            })?;

            self.handle = Some(handle);
        }

        // 注册标准指标描述
        self.register_descriptions();

        self.initialized = true;

        tracing::info!(
            target: "bamboo_observability",
            "Metrics collector initialized"
        );

        Ok(())
    }

    /// 注册指标描述
    fn register_descriptions(&self) {
        let prefix = &self.config.prefix;

        // HTTP 请求相关
        describe_counter!(
            format!("{}_http_requests_total", prefix),
            Unit::Count,
            "Total number of HTTP requests"
        );
        describe_histogram!(
            format!("{}_http_request_duration_seconds", prefix),
            Unit::Seconds,
            "HTTP request duration in seconds"
        );

        // Session 相关
        describe_gauge!(
            format!("{}_sessions_active", prefix),
            Unit::Count,
            "Number of active sessions"
        );

        // Agent 相关
        describe_counter!(
            format!("{}_agent_calls_total", prefix),
            Unit::Count,
            "Total number of agent calls"
        );

        // Tool 相关
        describe_counter!(
            format!("{}_tool_executions_total", prefix),
            Unit::Count,
            "Total number of tool executions"
        );
    }

    /// 获取 Prometheus 格式的指标
    pub fn render(&self) -> String {
        #[cfg(feature = "prometheus")]
        {
            self.handle.as_ref()
                .map(|h| h.render())
                .unwrap_or_default()
        }
        #[cfg(not(feature = "prometheus"))]
        {
            String::new()
        }
    }

    /// 关闭指标收集器
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!(
            target: "bamboo_observability",
            "Metrics collector shutting down"
        );
        Ok(())
    }
}

/// HTTP 请求指标记录器
pub struct HttpMetrics;

impl HttpMetrics {
    /// 记录 HTTP 请求
    pub fn record_request(_method: &str, _path: &str) {
        metrics::counter!("bamboo_http_requests_total").increment(1);
    }

    /// 记录 HTTP 响应
    pub fn record_response(_method: &str, _path: &str, _status: u16) {
        metrics::counter!("bamboo_http_responses_total").increment(1);
    }

    /// 记录 HTTP 请求持续时间
    pub fn record_duration(_method: &str, _path: &str, duration_secs: f64) {
        metrics::histogram!("bamboo_http_request_duration_seconds").record(duration_secs);
    }
}

/// Session 指标记录器
pub struct SessionMetrics;

impl SessionMetrics {
    /// 增加活跃会话数
    pub fn increment_active() {
        metrics::gauge!("bamboo_sessions_active").increment(1.0);
    }

    /// 减少活跃会话数
    pub fn decrement_active() {
        metrics::gauge!("bamboo_sessions_active").decrement(1.0);
    }

    /// 记录会话创建
    pub fn record_created() {
        metrics::counter!("bamboo_sessions_created_total").increment(1);
    }

    /// 记录会话关闭
    pub fn record_closed() {
        metrics::counter!("bamboo_sessions_closed_total").increment(1);
    }

    /// 设置活跃会话数
    pub fn set_active(count: usize) {
        metrics::gauge!("bamboo_sessions_active").set(count as f64);
    }
}

/// Agent 指标记录器
pub struct AgentMetrics;

impl AgentMetrics {
    /// 记录 Agent 调用
    pub fn record_call(_agent_id: &str) {
        metrics::counter!("bamboo_agent_calls_total").increment(1);
    }

    /// 记录 Agent 调用持续时间
    pub fn record_duration(_agent_id: &str, duration_secs: f64) {
        metrics::histogram!("bamboo_agent_call_duration_seconds").record(duration_secs);
    }

    /// 记录 Agent 错误
    pub fn record_error(_agent_id: &str, _error_type: &str) {
        metrics::counter!("bamboo_agent_errors_total").increment(1);
    }
}

/// Tool 指标记录器
pub struct ToolMetrics;

impl ToolMetrics {
    /// 记录工具执行
    pub fn record_execution(_tool_name: &str) {
        metrics::counter!("bamboo_tool_executions_total").increment(1);
    }

    /// 记录工具执行持续时间
    pub fn record_duration(_tool_name: &str, duration_secs: f64) {
        metrics::histogram!("bamboo_tool_execution_duration_seconds").record(duration_secs);
    }

    /// 记录工具错误
    pub fn record_error(_tool_name: &str, _error_type: &str) {
        metrics::counter!("bamboo_tool_errors_total").increment(1);
    }
}

/// LLM 指标记录器
pub struct LlmMetrics;

impl LlmMetrics {
    /// 记录 Token 消耗
    pub fn record_tokens(_model: &str, input_tokens: u64, output_tokens: u64) {
        metrics::counter!("bamboo_llm_tokens_input_total").increment(input_tokens);
        metrics::counter!("bamboo_llm_tokens_output_total").increment(output_tokens);
    }

    /// 记录 LLM 请求持续时间
    pub fn record_duration(_model: &str, duration_secs: f64) {
        metrics::histogram!("bamboo_llm_request_duration_seconds").record(duration_secs);
    }
}

/// Skill 指标记录器
pub struct SkillMetrics;

impl SkillMetrics {
    /// 记录 Skill 调用
    pub fn record_invocation(_skill_id: &str) {
        metrics::counter!("bamboo_skill_invocations_total").increment(1);
    }
}

/// 指标属性构建器
#[derive(Debug, Default)]
pub struct MetricAttributes {
    attributes: Vec<(String, String)>,
}

impl MetricAttributes {
    /// 创建新的属性构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加属性
    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// 添加请求 ID
    pub fn request_id(self, id: impl Into<String>) -> Self {
        self.add("request_id", id)
    }

    /// 添加会话 ID
    pub fn session_id(self, id: impl Into<String>) -> Self {
        self.add("session_id", id)
    }

    /// 添加 Agent ID
    pub fn agent_id(self, id: impl Into<String>) -> Self {
        self.add("agent_id", id)
    }

    /// 添加用户 ID
    pub fn user_id(self, id: impl Into<String>) -> Self {
        self.add("user_id", id)
    }

    /// 转换为属性引用列表
    pub fn as_refs(&self) -> Vec<(&str, &str)> {
        self.attributes
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_init() {
        let config = Config::default();
        let collector = MetricsCollector::new(&config).await;
        assert!(collector.is_ok());
    }

    #[test]
    fn test_metric_attributes() {
        let attrs = MetricAttributes::new()
            .request_id("req-123")
            .session_id("sess-456")
            .agent_id("agent-789");
        
        let refs = attrs.as_refs();
        assert!(refs.iter().any(|(k, v)| *k == "request_id" && *v == "req-123"));
        assert!(refs.iter().any(|(k, v)| *k == "session_id" && *v == "sess-456"));
    }
}
