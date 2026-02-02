//! Bamboo Server 与 Observability 集成示例
//!
//! 展示如何在 bamboo-server 中集成观测性基础设施

use std::sync::Arc;
use std::time::Instant;

use actix_web::{
    dev::Service as _,
    middleware::{self, Logger},
    web, App, HttpRequest, HttpResponse, HttpServer,
};
use bamboo_observability::prelude::*;
use bamboo_observability::{
    AgentMetrics, HttpMetrics, LlmMetrics, SessionMetrics, ToolMetrics,
    create_request_span, create_session_span, simple_check, database_check,
    ErrorContext, Context, HealthStatus, HealthCheckResult,
};
use futures_util::future::FutureExt;

/// 应用状态
struct AppState {
    observability: Observability,
    db_pool: DbPool,
}

/// 数据库连接池（模拟）
struct DbPool;

impl DbPool {
    async fn ping(&self) -> std::result::Result<(), String> {
        Ok(())
    }
}

/// 主函数
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 1. 初始化观测性基础设施
    let config = load_config().await;
    let obs = Observability::init(config)
        .await
        .expect("Failed to initialize observability");
    
    info!(
        target: "bamboo_server",
        "Bamboo server initializing..."
    );
    
    // 2. 注册健康检查
    register_health_checks(&obs).await;
    
    // 3. 启动健康检查服务器
    obs.start_health_server()
        .await
        .expect("Failed to start health server");
    
    // 4. 创建应用状态
    let app_state = web::Data::new(AppState {
        observability: obs,
        db_pool: DbPool,
    });
    
    // 5. 启动 Actix-web 服务器
    let bind_addr = "127.0.0.1:3000";
    info!(
        target: "bamboo_server",
        "Starting HTTP server on {}",
        bind_addr
    );
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // 指标记录中间件
            .wrap(metrics_middleware())
            // Tracing 中间件
            .wrap(tracing_actix_web::TracingLogger::default())
            // 路由
            .service(
                web::scope("/api/v1")
                    .route("/chat", web::post().to(handle_chat))
                    .route("/sessions", web::post().to(create_session))
                    .route("/sessions/{id}", web::get().to(get_session))
                    .route("/agents/{id}/invoke", web::post().to(invoke_agent))
            )
    })
    .bind(bind_addr)?
    .run()
    .await
}

/// 加载配置
async fn load_config() -> Config {
    // 优先从环境变量加载
    if let Ok(config) = Config::from_env() {
        return config;
    }
    
    // 其次从配置文件加载
    if let Ok(config) = Config::from_file("/etc/bamboo/config.toml") {
        return config;
    }
    
    // 使用默认配置
    Config::default()
        .with_log_level("info")
        .with_json_format(true)
        .with_metrics_port(9090)
        .with_health_port(8080)
}

/// 注册健康检查
async fn register_health_checks(obs: &Observability) {
    // 内存检查
    obs.register_health_check(
        "memory",
        simple_check("memory", true)
    ).await;
    
    // 数据库检查
    let pool = DbPool;
    obs.register_health_check(
        "database",
        database_check("database", move || {
            let pool = pool;
            async move { pool.ping().await }
        })
    ).await;
    
    // LLM 服务检查
    obs.register_health_check(
        "llm_service",
        simple_check("llm_service", true)
    ).await;
}

/// 指标记录中间件
fn metrics_middleware() -> impl actix_web::dev::Transform<
    actix_web::dev::ServiceRequest,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
    InitError = (),
> {
    actix_web::dev::fn_transform(|service| {
        actix_web::dev::fn_service(move |req: actix_web::dev::ServiceRequest| {
            let method = req.method().to_string();
            let path = req.path().to_string();
            
            HttpMetrics::record_request(&method, &path);
            
            let start = Instant::now();
            let fut = service.call(req);
            
            async move {
                let res = fut.await;
                let duration = start.elapsed();
                
                if let Ok(ref resp) = res {
                    HttpMetrics::record_response(&method, &path, resp.status().as_u16());
                    HttpMetrics::record_duration(&method, &path, duration.as_secs_f64());
                }
                
                res
            }
            .boxed_local()
        })
    })
}

/// 处理聊天请求
#[tracing::instrument(
    target = "bamboo_server",
    skip(data, body),
    fields(request_id, session_id)
)]
async fn handle_chat(
    data: web::Data<AppState>,
    body: web::Json<ChatRequest>,
) -> HttpResponse {
    let request_id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("request_id", &request_id);
    
    info!("Processing chat request");
    
    // 获取或创建会话
    let session_id = body.session_id.clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    tracing::Span::current().record("session_id", &session_id);
    
    // 记录 Session 指标
    SessionMetrics::record_created();
    SessionMetrics::increment_active();
    
    // 创建 Session Span
    let session_span = create_session_span(&session_id, Some(&request_id));
    
    let result = async {
        info!("Starting chat processing");
        
        // 模拟 Agent 调用
        let agent_id = "default";
        let agent_result = call_agent(agent_id, &body.message).await;
        
        match agent_result {
            Ok(response) => {
                info!("Chat processed successfully");
                HttpResponse::Ok().json(ChatResponse {
                    message: response,
                    session_id,
                    request_id,
                })
            }
            Err(e) => {
                error!(error = %e, "Chat processing failed");
                HttpResponse::InternalServerError().json(json!({
                    "error": e.to_string(),
                    "request_id": request_id,
                }))
            }
        }
    }
    .instrument(session_span)
    .await;
    
    // 清理 Session 指标
    SessionMetrics::decrement_active();
    
    result
}

/// 创建新会话
async fn create_session(data: web::Data<AppState>) -> HttpResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    
    info!(
        target: "bamboo_server",
        session_id = %session_id,
        "Creating new session"
    );
    
    SessionMetrics::record_created();
    SessionMetrics::increment_active();
    
    HttpResponse::Created().json(json!({
        "session_id": session_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
    }))
}

/// 获取会话信息
async fn get_session(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    
    debug!(
        target: "bamboo_server",
        session_id = %session_id,
        "Fetching session"
    );
    
    // 模拟获取会话
    HttpResponse::Ok().json(json!({
        "session_id": session_id,
        "messages": [],
    }))
}

/// 调用 Agent
#[tracing::instrument(target = "bamboo_server", skip(data, body))]
async fn invoke_agent(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<AgentRequest>,
) -> HttpResponse {
    let agent_id = path.into_inner();
    
    let start = Instant::now();
    AgentMetrics::record_call(&agent_id);
    
    // 模拟 Agent 执行
    let result = execute_agent_logic(&agent_id, &body.input).await;
    
    match result {
        Ok(output) => {
            AgentMetrics::record_duration(&agent_id, start.elapsed().as_secs_f64());
            
            HttpResponse::Ok().json(AgentResponse {
                agent_id,
                output,
            })
        }
        Err(e) => {
            AgentMetrics::record_error(&agent_id, &e.error_type);
            
            HttpResponse::InternalServerError().json(json!({
                "error": e.to_string(),
            }))
        }
    }
}

/// 调用 Agent 逻辑（模拟）
async fn call_agent(agent_id: &str, message: &str) -> Result<String, AgentError> {
    let start = Instant::now();
    
    // 模拟 LLM 调用
    LlmMetrics::record_tokens("gpt-4", 100, 50);
    LlmMetrics::record_duration("gpt-4", start.elapsed().as_secs_f64());
    
    // 模拟 Tool 调用
    ToolMetrics::record_execution("search");
    ToolMetrics::record_duration("search", 0.5);
    
    Ok(format!("Response to: {}", message))
}

/// 执行 Agent 逻辑（模拟）
async fn execute_agent_logic(agent_id: &str, input: &str) -> Result<String, AgentError> {
    // 模拟处理
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    Ok(format!("Agent {} processed: {}", agent_id, input))
}

// ============================================================================
// 请求/响应类型
// ============================================================================

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, debug, error, Instrument};

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    message: String,
    session_id: String,
    request_id: String,
}

#[derive(Debug, Deserialize)]
struct AgentRequest {
    input: String,
}

#[derive(Debug, Serialize)]
struct AgentResponse {
    agent_id: String,
    output: String,
}

#[derive(Debug)]
struct AgentError {
    error_type: String,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Agent error: {}", self.error_type)
    }
}

impl std::error::Error for AgentError {}

// ============================================================================
// 与 bamboo-core 集成示例
// ============================================================================

use bamboo_observability::create_agent_span;

/// 在 Agent Loop 中集成观测性
async fn agent_loop_with_observability(
    session_id: &str,
    agent_id: &str,
) -> bamboo_observability::Result<()> {
    let agent_span = create_agent_span(agent_id, Some(session_id));
    
    async {
        info!(target: "bamboo_skill", "Starting agent loop");
        
        // 记录 Agent 调用开始
        AgentMetrics::record_call(agent_id);
        let start = Instant::now();
        
        // 模拟 Agent 执行
        // ...
        
        // 记录完成
        AgentMetrics::record_duration(agent_id, start.elapsed().as_secs_f64());
        
        info!(target: "bamboo_skill", "Agent loop completed");
        
        Ok(())
    }
    .instrument(agent_span)
    .await
}

// ============================================================================
// 错误处理集成示例
// ============================================================================

async fn handle_request_with_context(
    request_id: &str,
    session_id: &str,
) -> bamboo_observability::Result<()> {
    // 创建错误上下文
    let ctx = ErrorContext::new()
        .request_id(request_id)
        .session_id(session_id)
        .agent_id("agent-001");
    
    // 执行业务逻辑，自动添加上下文
    let result = risky_operation()
        .map_err(|e| ObservabilityError::runtime_with_context(
            format!("Operation failed: {}", e),
            ctx
        ));
    
    result
}

fn risky_operation() -> std::result::Result<(), String> {
    // 模拟可能失败的操作
    Ok(())
}
