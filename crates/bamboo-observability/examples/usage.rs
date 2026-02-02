//! Bamboo Observability - 使用示例
//!
//! 本文件展示了如何在 Bamboo 项目中使用观测性基础设施。

use bamboo_observability::prelude::*;
use bamboo_observability::{
    HttpMetrics, SessionMetrics, AgentMetrics, ToolMetrics, LlmMetrics,
    create_request_span, create_session_span, create_agent_span,
    simple_check, database_check,
};

// ============================================================================
// 1. 基础使用示例
// ============================================================================

/// 示例 1: 快速初始化观测性基础设施
pub async fn example_basic_init() -> bamboo_observability::Result<()> {
    // 使用默认配置
    let config = Config::default();
    let obs = Observability::init(config).await?;
    
    // 使用日志
    info!(target: "bamboo_server", "Server initialized");
    
    // 启动健康检查服务器
    obs.start_health_server().await?;
    
    Ok(())
}

/// 示例 2: 使用自定义配置
pub async fn example_custom_config() -> bamboo_observability::Result<()> {
    let config = Config::default()
        // 日志配置
        .with_log_level("debug")
        .with_json_format(true)
        .with_log_file("/var/log/bamboo/app.log")
        .with_module_level("bamboo_server", "debug")
        .with_module_level("bamboo_gateway", "info")
        .with_module_level("bamboo_skill", "warn")
        // 指标配置
        .with_metrics_port(9090)
        // 健康检查配置
        .with_health_port(8080);
    
    let obs = Observability::init(config).await?;
    
    info!(target: "bamboo_server", "Server initialized with custom config");
    
    obs.start_health_server().await?;
    
    Ok(())
}

/// 示例 3: 从环境变量加载配置
pub async fn example_env_config() -> bamboo_observability::Result<()> {
    // 设置环境变量示例:
    // BAMBOO__LOGGING__LEVEL=debug
    // BAMBOO__LOGGING__JSON_FORMAT=true
    // BAMBOO__METRICS__PORT=9090
    // BAMBOO__HEALTH__PORT=8080
    
    let config = Config::from_env()
        .map_err(|e| ObservabilityError::config(format!("Failed to load config: {}", e)))?;
    
    let obs = Observability::init(config).await?;
    obs.start_health_server().await?;
    
    Ok(())
}

/// 示例 4: 从配置文件加载
pub async fn example_file_config() -> bamboo_observability::Result<()> {
    let config = Config::from_file("/etc/bamboo/config.toml")
        .map_err(|e| ObservabilityError::config(format!("Failed to load config file: {}", e)))?;
    
    let obs = Observability::init(config).await?;
    obs.start_health_server().await?;
    
    Ok(())
}

// ============================================================================
// 2. 结构化日志使用示例
// ============================================================================

/// 示例 5: 使用 tracing 记录结构化日志
#[tracing::instrument(
    target = "bamboo_server",
    skip(request),
    fields(request_id, session_id)
)]
pub async fn handle_request(request: Request) -> Result<Response, Error> {
    // 记录进入函数
    tracing::info!("Processing request");
    
    // 记录调试信息
    tracing::debug!(request_body = ?request.body, "Request body");
    
    // 模拟处理
    let result = process_request(&request).await;
    
    match &result {
        Ok(response) => {
            tracing::info!(
                status_code = response.status,
                "Request processed successfully"
            );
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                "Request processing failed"
            );
        }
    }
    
    result
}

/// 示例 6: 使用带上下文的 Span
pub async fn example_with_context() {
    // 创建带有 request_id 的 span
    let request_span = create_request_span("req-abc-123");
    
    async {
        // 在这个 span 内的所有日志都会包含 request_id
        tracing::info!("Inside request context");
        
        // 嵌套的 session span
        let session_span = create_session_span("sess-xyz-456", Some("req-abc-123"));
        
        async {
            tracing::info!("Inside session context with both IDs");
        }
        .instrument(session_span)
        .await;
    }
    .instrument(request_span)
    .await;
}

/// 示例 7: 使用自定义宏记录日志
pub async fn example_custom_macros() {
    let request_id = "req-123";
    let session_id = "sess-456";
    
    bamboo_observability::log_request!("bamboo_server", request_id, "Received request");
    bamboo_observability::log_session!("bamboo_server", session_id, "Session started");
    bamboo_observability::log_agent!("bamboo_skill", "agent-789", "Agent invoked");
}

/// 示例 8: 动态更新日志级别
pub async fn example_dynamic_log_level(obs: &Observability) -> bamboo_observability::Result<()> {
    // 临时提高日志级别进行调试
    obs.update_log_level("debug").await?;
    
    tracing::debug!("This will be logged now");
    
    // 恢复日志级别
    obs.update_log_level("info").await?;
    
    Ok(())
}

// ============================================================================
// 3. 指标收集使用示例
// ============================================================================

/// 示例 9: 记录 HTTP 请求指标
pub async fn handle_http_request(req: &HttpRequest) -> HttpResponse {
    let start = std::time::Instant::now();
    
    // 记录请求
    HttpMetrics::record_request(&req.method, &req.path);
    
    // 处理请求...
    let response = process_http_request(req).await;
    
    // 记录响应
    HttpMetrics::record_response(&req.method, &req.path, response.status);
    
    // 记录持续时间
    HttpMetrics::record_duration(
        &req.method,
        &req.path,
        start.elapsed().as_secs_f64()
    );
    
    response
}

/// 示例 10: 记录 Session 指标
pub fn create_session() -> Session {
    SessionMetrics::record_created();
    SessionMetrics::increment_active();
    
    Session::new()
}

pub fn close_session(session: Session) {
    SessionMetrics::record_closed();
    SessionMetrics::decrement_active();
    
    // 或者设置具体的活跃数
    SessionMetrics::set_active(10);
}

/// 示例 11: 记录 Agent 调用指标
pub async fn invoke_agent(agent_id: &str, request: &str) -> AgentResult {
    let start = std::time::Instant::now();
    
    AgentMetrics::record_call(agent_id);
    
    match execute_agent(agent_id, request).await {
        Ok(result) => {
            AgentMetrics::record_duration(agent_id, start.elapsed().as_secs_f64());
            Ok(result)
        }
        Err(e) => {
            AgentMetrics::record_error(agent_id, &e.error_type);
            Err(e)
        }
    }
}

/// 示例 12: 记录 Tool 执行指标
pub async fn execute_tool(tool_name: &str, args: &Args) -> ToolResult {
    let start = std::time::Instant::now();
    
    ToolMetrics::record_execution(tool_name);
    
    match run_tool(tool_name, args).await {
        Ok(result) => {
            ToolMetrics::record_duration(tool_name, start.elapsed().as_secs_f64());
            Ok(result)
        }
        Err(e) => {
            ToolMetrics::record_error(tool_name, &e.to_string());
            Err(e)
        }
    }
}

/// 示例 13: 记录 LLM 指标
pub async fn call_llm(model: &str, messages: &[Message]) -> LlmResponse {
    let start = std::time::Instant::now();
    
    let response = send_to_llm(model, messages).await;
    
    // 记录 Token 消耗
    LlmMetrics::record_tokens(
        model,
        response.usage.input_tokens,
        response.usage.output_tokens
    );
    
    // 记录请求持续时间
    LlmMetrics::record_duration(model, start.elapsed().as_secs_f64());
    
    response
}

// ============================================================================
// 4. 健康检查使用示例
// ============================================================================

/// 示例 14: 注册基本健康检查
pub async fn example_basic_health_checks(obs: &Observability) {
    // 简单健康检查
    obs.register_health_check("memory", simple_check("memory", true)).await;
    
    // 函数式健康检查
    let check = FnHealthCheck::new("disk_space", || async {
        HealthCheckResult {
            name: "disk_space".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Disk usage: 45%".to_string()),
            response_time_ms: Some(10),
            last_checked: Some(chrono::Utc::now().to_rfc3339()),
            metadata: Some(serde_json::json!({
                "total_bytes": 1000000000000u64,
                "used_bytes": 450000000000u64,
            })),
        }
    });
    
    obs.register_health_check("disk", check).await;
}

/// 示例 15: 数据库健康检查
pub async fn example_db_health_check(obs: &Observability, pool: &DbPool) {
    let pool = pool.clone();
    
    let db_check = database_check("database", move || {
        let pool = pool.clone();
        async move {
            pool.execute("SELECT 1").await
                .map_err(|e| e.to_string())
        }
    });
    
    obs.register_health_check("database", db_check).await;
}

/// 示例 16: 自定义健康检查
pub async fn example_custom_health_check(obs: &Observability) {
    struct CustomHealthCheck {
        name: String,
        threshold: f64,
    }
    
    #[async_trait::async_trait]
    impl HealthCheck for CustomHealthCheck {
        async fn check(&self) -> HealthCheckResult {
            let cpu_usage = get_cpu_usage().await;
            
            let status = if cpu_usage > self.threshold * 1.5 {
                HealthStatus::Unhealthy
            } else if cpu_usage > self.threshold {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            };
            
            HealthCheckResult {
                name: self.name.clone(),
                status,
                message: Some(format!("CPU usage: {:.1}%", cpu_usage)),
                response_time_ms: Some(0),
                last_checked: Some(chrono::Utc::now().to_rfc3339()),
                metadata: Some(serde_json::json!({
                    "cpu_usage": cpu_usage,
                    "threshold": self.threshold,
                })),
            }
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    let cpu_check = CustomHealthCheck {
        name: "cpu_usage".to_string(),
        threshold: 80.0,
    };
    
    obs.register_health_check("cpu", cpu_check).await;
}

// ============================================================================
// 5. 错误处理与上下文示例
// ============================================================================

/// 示例 17: 使用错误上下文
pub async fn example_error_context() -> bamboo_observability::Result<()> {
    let result: std::result::Result<(), std::io::Error> = Err(
        std::io::Error::new(std::io::ErrorKind::NotFound, "File not found")
    );
    
    // 添加上下文
    let result = result
        .with_request_id("req-123")
        .with_session_id("sess-456");
    
    result
}

/// 示例 18: 完整的错误处理
pub async fn process_with_error_handling(
    request_id: &str,
    session_id: &str,
) -> bamboo_observability::Result<ProcessingResult> {
    // 创建错误上下文
    let ctx = ErrorContext::new()
        .request_id(request_id)
        .session_id(session_id);
    
    // 执行业务逻辑
    let data = fetch_data().await
        .map_err(|e| ObservabilityError::runtime_with_context(
            format!("Failed to fetch data: {}", e),
            ctx.clone()
        ))?;
    
    let result = process_data(data).await
        .map_err(|e| ObservabilityError::runtime_with_context(
            format!("Failed to process data: {}", e),
            ctx
        ))?;
    
    Ok(result)
}

// ============================================================================
// 辅助类型定义（仅用于示例）
// ============================================================================

struct Request { body: String }
struct Response { status: u16 }
struct HttpRequest { method: String, path: String }
struct HttpResponse { status: u16 }
struct Session;
struct AgentResult;
struct ToolResult;
struct LlmResponse { usage: TokenUsage }
struct TokenUsage { input_tokens: u64, output_tokens: u64 }
struct ProcessingResult;
struct Message;
struct Args;
struct Error;
struct DbPool;

impl Session {
    fn new() -> Self { Self }
}

async fn process_request(_: &Request) -> Result<Response, Error> { 
    Ok(Response { status: 200 }) 
}
async fn process_http_request(_: &HttpRequest) -> HttpResponse { 
    HttpResponse { status: 200 } 
}
async fn execute_agent(_: &str, _: &str) -> Result<AgentResult, Error> { 
    Ok(AgentResult) 
}
async fn run_tool(_: &str, _: &Args) -> Result<ToolResult, Error> { 
    Ok(ToolResult) 
}
async fn send_to_llm(_: &str, _: &[Message]) -> LlmResponse { 
    LlmResponse { 
        usage: TokenUsage { 
            input_tokens: 100, 
            output_tokens: 50 
        } 
    } 
}
async fn fetch_data() -> std::io::Result<String> { Ok(String::new()) }
async fn process_data(_: String) -> std::io::Result<ProcessingResult> { 
    Ok(ProcessingResult) 
}
async fn get_cpu_usage() -> f64 { 50.0 }

impl DbPool {
    async fn execute(&self, _: &str) -> Result<(), String> { Ok(()) }
}

// ============================================================================
// 6. 完整的服务器示例
// ============================================================================

/// 完整示例：在 Bamboo Server 中使用观测性基础设施
pub async fn full_server_example() -> bamboo_observability::Result<()> {
    // 1. 初始化观测性
    let config = Config::from_env()
        .unwrap_or_else(|_| {
            Config::default()
                .with_log_level("info")
                .with_json_format(true)
                .with_log_file("/var/log/bamboo/server.log")
                .with_metrics_port(9090)
                .with_health_port(8080)
        });
    
    let obs = Observability::init(config).await?;
    
    // 2. 注册健康检查
    obs.register_health_check("memory", simple_check("memory", true)).await;
    obs.register_health_check("disk", simple_check("disk", true)).await;
    
    // 3. 启动健康检查服务器
    obs.start_health_server().await?;
    
    // 4. 记录启动信息
    info!(
        target: "bamboo_server",
        version = env!("CARGO_PKG_VERSION"),
        "Bamboo server started"
    );
    
    // 5. 启动主服务循环
    // server_loop().await;
    
    // 6. 优雅关闭
    // obs.shutdown().await?;
    
    Ok(())
}
