//! 健康检查模块
//!
//! 提供健康检查端点：
//! - /health - 基本健康
//! - /metrics - Prometheus 格式指标
//! - /ready - 就绪检查

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::error::{ObservabilityError, Result};
use crate::metrics::MetricsCollector;

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 降级
    Degraded,
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Healthy
    }
}

impl HealthStatus {
    /// 转换为 HTTP 状态码
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // 200 但标记为降级
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// 检查是否健康
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 检查名称
    pub name: String,
    /// 状态
    pub status: HealthStatus,
    /// 消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 响应时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,
    /// 最后检查时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<String>,
    /// 额外元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// 整体健康响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// 整体状态
    pub status: HealthStatus,
    /// 应用名称
    pub app_name: String,
    /// 版本
    pub version: String,
    /// 运行时间（秒）
    pub uptime_seconds: u64,
    /// 时间戳
    pub timestamp: String,
    /// 各检查项结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<HealthCheckResult>>,
}

/// 就绪检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyResponse {
    /// 是否就绪
    pub ready: bool,
    /// 时间戳
    pub timestamp: String,
    /// 依赖项状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, bool>>,
}

/// 健康检查 trait
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// 执行检查
    async fn check(&self) -> HealthCheckResult;
    
    /// 获取检查名称
    fn name(&self) -> &str;
}

/// 简单的函数式健康检查
pub struct FnHealthCheck<F> {
    name: String,
    check_fn: F,
}

impl<F, Fut> FnHealthCheck<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = HealthCheckResult> + Send + 'static,
{
    /// 创建新的函数式健康检查
    pub fn new(name: impl Into<String>, check_fn: F) -> Self {
        Self {
            name: name.into(),
            check_fn,
        }
    }
}

#[async_trait::async_trait]
impl<F, Fut> HealthCheck for FnHealthCheck<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = HealthCheckResult> + Send + 'static,
{
    async fn check(&self) -> HealthCheckResult {
        (self.check_fn)().await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 应用状态
#[derive(Clone)]
struct AppState {
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 健康检查列表
    checks: Arc<RwLock<Vec<Box<dyn HealthCheck>>>>,
    /// 应用名称
    app_name: String,
    /// 版本
    version: String,
    /// 启动时间
    start_time: Instant,
}

impl AppState {
    /// 计算运行时间
    fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// 健康检查服务器
#[derive(Debug)]
pub struct HealthServer {
    /// 配置
    config: crate::config::HealthConfig,
    /// 应用名称
    app_name: String,
    /// 应用状态
    state: Arc<RwLock<Option<AppState>>>,
    /// 关闭信号
    shutdown_tx: Arc<RwLock<Option<broadcast::Sender<()>>>>,
    /// 服务器句柄
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl HealthServer {
    /// 创建新的健康检查服务器
    pub async fn new(config: &Config) -> Result<Self> {
        let health_config = config.health.clone();
        
        Ok(Self {
            config: health_config,
            app_name: config.app_name.clone(),
            state: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(RwLock::new(None)),
            server_handle: Arc::new(RwLock::new(None)),
        })
    }

    /// 注册健康检查
    pub async fn register<C>(&self, name: &str, check: C)
    where
        C: HealthCheck + Send + Sync + 'static,
    {
        if let Some(ref state) = *self.state.read() {
            let mut checks = state.checks.write();
            checks.push(Box::new(check));
            
            tracing::info!(
                target: "bamboo_observability",
                "Health check '{}' registered",
                name
            );
        }
    }

    /// 启动健康检查服务器
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        
        // 创建应用状态
        let state = AppState {
            metrics: Arc::new(MetricsCollector::new(&Config::default()).await?),
            checks: Arc::new(RwLock::new(Vec::new())),
            app_name: self.app_name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: Instant::now(),
        };

        {
            let mut state_guard = self.state.write();
            *state_guard = Some(state.clone());
        }

        // 构建路由
        let app = self.build_router(state);

        // 绑定地址
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| ObservabilityError::health(format!("Invalid address: {}", e)))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ObservabilityError::health(format!("Failed to bind: {}", e)))?;

        tracing::info!(
            target: "bamboo_observability",
            "Health server starting on http://{}",
            addr
        );

        // 启动服务器
        let server = axum::serve(listener, app);
        
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = server => {},
                _ = shutdown_rx.recv() => {
                    tracing::info!(
                        target: "bamboo_observability",
                        "Health server shutting down"
                    );
                }
            }
        });

        {
            let mut handle_guard = self.server_handle.write();
            *handle_guard = Some(handle);
        }

        // 保存关闭信号发送器
        {
            let mut tx_guard = self.shutdown_tx.write();
            *tx_guard = Some(shutdown_tx);
        }

        Ok(())
    }

    /// 构建路由
    fn build_router(&self, state: AppState) -> Router {
        let state = Arc::new(state);

        Router::new()
            .route(&self.config.health_path, get(health_handler))
            .route(&self.config.ready_path, get(ready_handler))
            .route(&self.config.metrics_path, get(metrics_handler))
            .route("/live", get(live_handler))
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    /// 关闭服务器
    pub async fn shutdown(&self) -> Result<()> {
        {
            let tx_guard = self.shutdown_tx.read();
            if let Some(ref tx) = *tx_guard {
                let _ = tx.send(());
            }
        }

        if let Some(handle) = self.server_handle.write().take() {
            let _ = handle.await;
        }

        Ok(())
    }
}

/// 健康检查处理器
async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let checks = state.checks.read();
    
    let mut results = Vec::new();
    let mut overall_status = HealthStatus::Healthy;

    for check in checks.iter() {
        let check_start = Instant::now();
        let result = check.check().await;
        let duration = check_start.elapsed();

        let mut result = result;
        result.response_time_ms = Some(duration.as_millis() as u64);
        result.last_checked = Some(chrono::Utc::now().to_rfc3339());

        // 更新整体状态
        match result.status {
            HealthStatus::Unhealthy => overall_status = HealthStatus::Unhealthy,
            HealthStatus::Degraded if overall_status == HealthStatus::Healthy => {
                overall_status = HealthStatus::Degraded;
            }
            _ => {}
        }

        results.push(result);
    }

    let response = HealthResponse {
        status: overall_status,
        app_name: state.app_name.clone(),
        version: state.version.clone(),
        uptime_seconds: state.uptime_seconds(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        checks: if results.is_empty() { None } else { Some(results) },
    };

    let status_code = overall_status.to_status_code();
    
    (
        status_code,
        [(header::CONTENT_TYPE, "application/json")],
        Json(response),
    )
}

/// 就绪检查处理器
async fn ready_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // 这里可以检查依赖项（数据库、缓存等）
    let response = ReadyResponse {
        ready: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
        dependencies: None,
    };

    (StatusCode::OK, Json(response))
}

/// 存活检查处理器
async fn live_handler() -> impl IntoResponse {
    StatusCode::OK
}

/// 指标处理器
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let metrics = state.metrics.render();
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(metrics))
        .unwrap()
}

/// 创建简单的健康检查
pub fn simple_check(name: impl Into<String>, healthy: bool) -> impl HealthCheck {
    struct SimpleCheck {
        name: String,
        healthy: bool,
    }

    #[async_trait::async_trait]
    impl HealthCheck for SimpleCheck {
        async fn check(&self) -> HealthCheckResult {
            HealthCheckResult {
                name: self.name.clone(),
                status: if self.healthy {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Unhealthy
                },
                message: None,
                response_time_ms: Some(0),
                last_checked: Some(chrono::Utc::now().to_rfc3339()),
                metadata: None,
            }
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    SimpleCheck {
        name: name.into(),
        healthy,
    }
}

/// 创建数据库健康检查
pub fn database_check<F, Fut>(name: impl Into<String>, check_fn: F) -> impl HealthCheck
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = std::result::Result<(), String>> + Send + 'static,
{
    struct DbCheck<F> {
        name: String,
        check_fn: F,
    }

    #[async_trait::async_trait]
    impl<F, Fut> HealthCheck for DbCheck<F>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = std::result::Result<(), String>> + Send + 'static,
    {
        async fn check(&self) -> HealthCheckResult {
            let start = Instant::now();
            
            match (self.check_fn)().await {
                Ok(_) => HealthCheckResult {
                    name: self.name.clone(),
                    status: HealthStatus::Healthy,
                    message: Some("Database connection OK".to_string()),
                    response_time_ms: Some(start.elapsed().as_millis() as u64),
                    last_checked: Some(chrono::Utc::now().to_rfc3339()),
                    metadata: None,
                },
                Err(e) => HealthCheckResult {
                    name: self.name.clone(),
                    status: HealthStatus::Unhealthy,
                    message: Some(format!("Database connection failed: {}", e)),
                    response_time_ms: Some(start.elapsed().as_millis() as u64),
                    last_checked: Some(chrono::Utc::now().to_rfc3339()),
                    metadata: None,
                },
            }
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    DbCheck {
        name: name.into(),
        check_fn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(HealthStatus::Degraded.is_healthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());
        
        assert_eq!(HealthStatus::Healthy.to_status_code(), StatusCode::OK);
        assert_eq!(HealthStatus::Unhealthy.to_status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_simple_check() {
        let check = simple_check("test", true);
        assert_eq!(check.name(), "test");
    }
}
