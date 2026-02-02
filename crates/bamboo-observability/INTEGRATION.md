# Bamboo Observability 集成指南

## 目录

1. [快速开始](#快速开始)
2. [配置详解](#配置详解)
3. [日志使用](#日志使用)
4. [指标收集](#指标收集)
5. [健康检查](#健康检查)
6. [错误处理](#错误处理)
7. [与 Bamboo 项目集成](#与-bamboo-项目集成)

---

## 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
bamboo-observability = { path = "../bamboo-observability" }
tokio = { version = "1", features = ["full"] }
```

### 2. 基础初始化

```rust
use bamboo_observability::prelude::*;

#[tokio::main]
async fn main() -> bamboo_observability::Result<()> {
    // 初始化观测性基础设施
    let config = Config::default()
        .with_log_level("info")
        .with_json_format(true)
        .with_metrics_port(9090)
        .with_health_port(8080);
    
    let obs = Observability::init(config).await?;
    
    // 启动健康检查服务器
    obs.start_health_server().await?;
    
    // 记录日志
    info!(target: "bamboo_server", "Server started successfully");
    
    // 记录指标
    counter!("bamboo_server_start_total", 1);
    
    // 保持运行
    tokio::signal::ctrl_c().await?;
    
    // 优雅关闭
    obs.shutdown().await?;
    
    Ok(())
}
```

---

## 配置详解

### 环境变量配置

所有配置都支持通过环境变量设置，使用 `__` 作为嵌套分隔符：

```bash
# 基础配置
export BAMBOO__APP_NAME="bamboo"
export BAMBOO__ENVIRONMENT="production"

# 日志配置
export BAMBOO__LOGGING__LEVEL=debug
export BAMBOO__LOGGING__JSON_FORMAT=true
export BAMBOO__LOGGING__FILE=true
export BAMBOO__LOGGING__FILE_PATH=/var/log/bamboo/app.log

# 模块级别日志
export BAMBOO__LOGGING__MODULE_LEVELS__bamboo_server=debug
export BAMBOO__LOGGING__MODULE_LEVELS__bamboo_gateway=info
export BAMBOO__LOGGING__MODULE_LEVELS__bamboo_skill=warn

# 日志轮转
export BAMBOO__LOGGING__ROTATION__STRATEGY=daily
export BAMBOO__LOGGING__ROTATION__MAX_FILES=7

# 指标配置
export BAMBOO__METRICS__ENABLED=true
export BAMBOO__METRICS__PORT=9090
export BAMBOO__METRICS__HOST=0.0.0.0

# 健康检查配置
export BAMBOO__HEALTH__ENABLED=true
export BAMBOO__HEALTH__PORT=8080
```

### 配置文件

#### TOML 格式

```toml
app_name = "bamboo"
environment = "production"

[logging]
level = "info"
json_format = true
stdout = true
file = true
file_path = "/var/log/bamboo/app.log"

[logging.rotation]
strategy = "daily"
max_files = 7

[logging.module_levels]
bamboo_server = "debug"
bamboo_gateway = "info"
bamboo_skill = "warn"
bamboo_mcp = "error"

[metrics]
enabled = true
port = 9090
host = "0.0.0.0"
prefix = "bamboo"

[health]
enabled = true
port = 8080
host = "0.0.0.0"
```

#### JSON 格式

```json
{
  "app_name": "bamboo",
  "environment": "production",
  "logging": {
    "level": "info",
    "json_format": true,
    "file_path": "/var/log/bamboo/app.log",
    "module_levels": {
      "bamboo_server": "debug",
      "bamboo_gateway": "info"
    }
  },
  "metrics": {
    "port": 9090
  },
  "health": {
    "port": 8080
  }
}
```

### 动态配置加载

```rust
// 从环境变量加载
let config = Config::from_env()?;

// 从文件加载
let config = Config::from_file("/etc/bamboo/config.toml")?;

// 混合加载（环境变量覆盖文件配置）
let config = Config::from_file("/etc/bamboo/config.toml")
    .unwrap_or_default()
    .merge_env()?;
```

---

## 日志使用

### 基础日志记录

```rust
use bamboo_observability::prelude::*;

// 不同级别的日志
trace!("This is a trace message");
debug!("This is a debug message");
info!("This is an info message");
warn!("This is a warning message");
error!("This is an error message");

// 带目标的日志
tracing::info!(target: "bamboo_server", "Server message");
tracing::debug!(target: "bamboo_gateway", "Gateway message");
tracing::warn!(target: "bamboo_skill", "Skill message");
```

### 结构化日志

```rust
// JSON 格式会自动包含字段
info!(
    target: "bamboo_server",
    request_id = %request_id,
    session_id = %session_id,
    user_id = %user_id,
    duration_ms = 100,
    "Request processed"
);
```

输出示例（JSON 格式）：
```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "target": "bamboo_server",
  "fields": {
    "message": "Request processed",
    "request_id": "req-abc-123",
    "session_id": "sess-xyz-456",
    "user_id": "user-789",
    "duration_ms": 100
  },
  "span": {
    "request_id": "req-abc-123",
    "session_id": "sess-xyz-456"
  }
}
```

### 使用 Span 进行上下文追踪

```rust
use bamboo_observability::{create_request_span, create_session_span, create_agent_span};

async fn handle_request(request: Request) -> Response {
    // 创建带有 request_id 的 span
    let request_span = create_request_span(&request.id);
    
    async {
        // 所有在这个 async 块内的日志都会包含 request_id
        info!("Processing request");
        
        // 创建嵌套的 session span
        let session_span = create_session_span(&request.session_id, Some(&request.id));
        
        async {
            info!("Inside session context");
            
            // 创建 agent span
            let agent_span = create_agent_span(&request.agent_id, Some(&request.session_id));
            
            async {
                info!("Agent processing");
            }
            .instrument(agent_span)
            .await;
        }
        .instrument(session_span)
        .await;
    }
    .instrument(request_span)
    .await
}
```

### 模块分级日志

```rust
// 在配置中设置模块级别
let config = Config::default()
    .with_module_level("bamboo_server", "debug")
    .with_module_level("bamboo_gateway", "info")
    .with_module_level("bamboo_skill", "warn");

// 在代码中指定目标
tracing::debug!(target: "bamboo_server", "This will be logged");
tracing::debug!(target: "bamboo_gateway", "This won't be logged (level=info)");
tracing::warn!(target: "bamboo_skill", "This will be logged");
```

### 动态调整日志级别

```rust
// 临时提高日志级别进行调试
obs.update_log_level("debug").await?;

// 恢复
obs.update_log_level("info").await?;

// 调整特定模块的日志级别
obs.log_manager().write().await
    .update_module_level("bamboo_server", "trace").await?;
```

---

## 指标收集

### 自动指标

内置的 HTTP 中间件会自动记录：
- `bamboo_http_requests_total` - HTTP 请求总数
- `bamboo_http_request_duration_seconds` - 请求持续时间
- `bamboo_http_responses_total` - HTTP 响应总数

### 手动记录指标

```rust
use bamboo_observability::{
    HttpMetrics, SessionMetrics, AgentMetrics, ToolMetrics, LlmMetrics,
};

// Session 指标
fn create_session() {
    SessionMetrics::record_created();
    SessionMetrics::increment_active();
}

fn close_session() {
    SessionMetrics::record_closed();
    SessionMetrics::decrement_active();
}

// Agent 指标
async fn invoke_agent(agent_id: &str) {
    let start = Instant::now();
    AgentMetrics::record_call(agent_id);
    
    let result = run_agent(agent_id).await;
    
    match result {
        Ok(_) => {
            AgentMetrics::record_duration(agent_id, start.elapsed().as_secs_f64());
        }
        Err(e) => {
            AgentMetrics::record_error(agent_id, &e.error_type);
        }
    }
}

// Tool 指标
async fn execute_tool(tool_name: &str) {
    let start = Instant::now();
    ToolMetrics::record_execution(tool_name);
    
    let result = run_tool(tool_name).await;
    
    if result.is_ok() {
        ToolMetrics::record_duration(tool_name, start.elapsed().as_secs_f64());
    } else {
        ToolMetrics::record_error(tool_name, "execution_failed");
    }
}

// LLM 指标
async fn call_llm(model: &str, messages: &[Message]) {
    let start = Instant::now();
    
    let response = llm_client.chat(model, messages).await;
    
    LlmMetrics::record_tokens(
        model,
        response.usage.input_tokens,
        response.usage.output_tokens
    );
    LlmMetrics::record_duration(model, start.elapsed().as_secs_f64());
}
```

### 自定义指标

```rust
use metrics::{counter, gauge, histogram};

// 计数器
counter!("bamboo_custom_events_total", 1, "event_type" => "user_login");

// 仪表盘
gauge!("bamboo_active_connections", 42.0);

// 直方图
histogram!("bamboo_custom_operation_seconds", 0.5);

// 带标签的指标
counter!(
    "bamboo_api_calls_total",
    1,
    "endpoint" => "/api/v1/chat",
    "method" => "POST",
    "status" => "200"
);
```

### 查看指标

访问 `http://localhost:8080/metrics`（默认）：

```
# HELP bamboo_http_requests_total Total number of HTTP requests
# TYPE bamboo_http_requests_total counter
bamboo_http_requests_total{method="GET",path="/api/v1/chat"} 100

# HELP bamboo_sessions_active Number of active sessions
# TYPE bamboo_sessions_active gauge
bamboo_sessions_active 5

# HELP bamboo_agent_calls_total Total number of agent calls
# TYPE bamboo_agent_calls_total counter
bamboo_agent_calls_total{agent_id="default"} 50
```

---

## 健康检查

### 端点说明

- `GET /health` - 返回应用整体健康状态
- `GET /ready` - 返回就绪状态（检查依赖项）
- `GET /live` - 存活检查（简单返回 200）
- `GET /metrics` - Prometheus 格式指标

### 注册健康检查

```rust
use bamboo_observability::{simple_check, database_check};

// 简单检查
obs.register_health_check(
    "memory",
    simple_check("memory", true)
).await;

// 数据库检查
obs.register_health_check(
    "database",
    database_check("postgres", || async {
        db_pool.ping().await.map_err(|e| e.to_string())
    })
).await;

// 自定义检查
struct CpuHealthCheck {
    threshold: f64,
}

#[async_trait::async_trait]
impl HealthCheck for CpuHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let usage = get_cpu_usage().await;
        
        HealthCheckResult {
            name: "cpu".to_string(),
            status: if usage > self.threshold {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            },
            message: Some(format!("CPU: {:.1}%", usage)),
            response_time_ms: Some(0),
            last_checked: Some(chrono::Utc::now().to_rfc3339()),
            metadata: None,
        }
    }
    
    fn name(&self) -> &str {
        "cpu"
    }
}

obs.register_health_check("cpu", CpuHealthCheck { threshold: 80.0 }).await;
```

### /health 响应示例

```json
{
  "status": "healthy",
  "app_name": "bamboo",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": [
    {
      "name": "memory",
      "status": "healthy",
      "message": "Memory usage OK",
      "response_time_ms": 1,
      "last_checked": "2024-01-15T10:30:00Z"
    },
    {
      "name": "database",
      "status": "healthy",
      "message": "Database connection OK",
      "response_time_ms": 10,
      "last_checked": "2024-01-15T10:30:00Z"
    },
    {
      "name": "cpu",
      "status": "degraded",
      "message": "CPU: 85.2%",
      "response_time_ms": 0,
      "last_checked": "2024-01-15T10:30:00Z"
    }
  ]
}
```

---

## 错误处理

### 统一错误类型

```rust
use bamboo_observability::{ObservabilityError, ErrorContext, Context};

fn may_fail() -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Not found"))
}

async fn handle_request(request_id: &str) -> bamboo_observability::Result<()> {
    // 添加上下文到错误
    may_fail()
        .with_request_id(request_id)
        .with_session_id("sess-123")
}

// 或使用完整的上下文
async fn complex_operation(request_id: &str, session_id: &str) -> bamboo_observability::Result<()> {
    let ctx = ErrorContext::new()
        .request_id(request_id)
        .session_id(session_id)
        .agent_id("agent-001")
        .user_id("user-789")
        .with_field("custom_field", "custom_value");
    
    some_operation()
        .map_err(|e| ObservabilityError::runtime_with_context(
            format!("Operation failed: {}", e),
            ctx
        ))
}
```

### 错误类型转换

```rust
// 自动转换
let result: bamboo_observability::Result<()> = Err(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "File not found"
));

// 手动创建错误
Err(ObservabilityError::config("Invalid configuration"))
Err(ObservabilityError::logging("Failed to write log"))
Err(ObservabilityError::metrics("Failed to record metric"))
```

---

## 与 Bamboo 项目集成

### 在 bamboo-server 中使用

```rust
// main.rs
use bamboo_observability::prelude::*;
use bamboo_observability::{
    HttpMetrics, SessionMetrics, AgentMetrics, ToolMetrics,
    create_request_span, simple_check, database_check,
};
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 1. 初始化观测性
    let config = Config::from_env()
        .unwrap_or_else(|_| Config::default()
            .with_log_level("info")
            .with_json_format(true));
    
    let obs = Observability::init(config).await
        .expect("Failed to initialize observability");
    
    // 2. 注册健康检查
    obs.register_health_check("server", simple_check("server", true)).await;
    
    // 3. 启动健康服务器
    obs.start_health_server().await
        .expect("Failed to start health server");
    
    // 4. 启动 HTTP 服务器
    HttpServer::new(move || {
        App::new()
            .wrap(metrics_middleware())
            .route("/api/v1/chat", web::post().to(handle_chat))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}

// HTTP 中间件记录指标
fn metrics_middleware() -> impl actix_web::dev::Transform<...> {
    // ... 参见 examples/server_integration.rs
}

// 处理函数
#[tracing::instrument(target: "bamboo_server", skip(req))]
async fn handle_chat(req: web::Json<ChatRequest>) -> HttpResponse {
    let request_id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("request_id", &request_id);
    
    info!("Processing chat request");
    
    // 记录 Session 指标
    SessionMetrics::record_created();
    SessionMetrics::increment_active();
    
    // 调用 Agent
    let result = call_agent(&req.message).await;
    
    // 记录 Agent 指标
    match &result {
        Ok(_) => AgentMetrics::record_call("default"),
        Err(e) => AgentMetrics::record_error("default", &e.to_string()),
    }
    
    SessionMetrics::decrement_active();
    
    match result {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
```

### 在 bamboo-core 中使用

```rust
// agent_loop.rs
use bamboo_observability::prelude::*;
use bamboo_observability::{AgentMetrics, create_agent_span};

pub struct AgentLoop {
    agent_id: String,
}

impl AgentLoop {
    #[tracing::instrument(target: "bamboo_core", skip(self))]
    pub async fn run(&self, session: &Session, message: &str) -> Result<()> {
        let agent_span = create_agent_span(&self.agent_id, Some(&session.id));
        
        async {
            info!("Agent loop started");
            
            let start = Instant::now();
            AgentMetrics::record_call(&self.agent_id);
            
            // 执行 Agent 逻辑
            self.execute_round(session, message).await?;
            
            AgentMetrics::record_duration(&self.agent_id, start.elapsed().as_secs_f64());
            
            info!("Agent loop completed");
            
            Ok(())
        }
        .instrument(agent_span)
        .await
    }
}
```

### 在 bamboo-skill 中使用

```rust
// skill_executor.rs
use bamboo_observability::prelude::*;
use bamboo_observability::{ToolMetrics, SkillMetrics};

pub async fn invoke_skill(skill_id: &str, input: &str) -> Result<String> {
    info!(target: "bamboo_skill", skill_id = %skill_id, "Invoking skill");
    
    SkillMetrics::record_invocation(skill_id);
    
    // 执行 skill
    let result = execute_skill(skill_id, input).await;
    
    match &result {
        Ok(_) => info!(target: "bamboo_skill", "Skill executed successfully"),
        Err(e) => error!(target: "bamboo_skill", error = %e, "Skill execution failed"),
    }
    
    result
}
```

---

## 最佳实践

1. **始终指定日志目标**：使用 `target: "bamboo_xxx"` 便于模块分级
2. **使用结构化日志**：添加关键字段如 request_id, session_id
3. **合理设置日志级别**：生产环境使用 info，调试使用 debug
4. **记录所有关键指标**：请求数、响应时间、错误数
5. **注册必要的健康检查**：数据库、缓存、外部服务
6. **使用 Span 追踪上下文**：便于追踪请求流程
7. **添加上下文到错误**：request_id, session_id 等
8. **动态调整日志级别**：遇到问题时可临时提高日志级别

---

## 故障排查

### 日志不输出

1. 检查日志级别设置
2. 确认模块级别配置
3. 检查 `RUST_LOG` 环境变量

### 指标不更新

1. 确认指标服务器已启动
2. 检查端口是否被占用
3. 确认 `metrics.enabled = true`

### 健康检查失败

1. 检查端口配置
2. 确认健康检查服务器已启动
3. 查看健康检查具体错误
