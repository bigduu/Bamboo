# Bamboo Observability

Bamboo 统一的日志和监控基础设施。

## 功能特性

- **结构化日志** (tracing)：JSON 格式、模块分级、日志轮转
- **指标收集** (metrics)：Prometheus 格式，内置 HTTP/Session/Agent/Tool/LLM 指标
- **健康检查**：/health、/ready、/metrics 端点
- **配置管理**：支持环境变量、配置文件
- **错误追踪**：统一的错误类型和上下文

## 快速开始

### 添加依赖

```toml
[dependencies]
bamboo-observability = { path = "../bamboo-observability" }
```

### 基本使用

```rust
use bamboo_observability::prelude::*;

#[tokio::main]
async fn main() -> bamboo_observability::Result<()> {
    // 初始化
    let config = Config::default()
        .with_log_level("info")
        .with_json_format(true);
    
    let obs = Observability::init(config).await?;
    
    // 记录日志
    info!(target: "bamboo_server", "Server started");
    
    // 记录指标
    counter!("bamboo_requests_total", 1);
    
    // 启动健康检查服务器
    obs.start_health_server().await?;
    
    Ok(())
}
```

## 配置说明

### 环境变量配置

所有配置都支持通过环境变量设置，前缀为 `BAMBOO__`：

```bash
# 日志级别
BAMBOO__LOGGING__LEVEL=debug

# JSON 格式输出
BAMBOO__LOGGING__JSON_FORMAT=true

# 模块级别日志
BAMBOO__LOGGING__MODULE_LEVELS__bamboo_server=debug
BAMBOO__LOGGING__MODULE_LEVELS__bamboo_gateway=info

# 指标服务器端口
BAMBOO__METRICS__PORT=9090

# 健康检查服务器端口
BAMBOO__HEALTH__PORT=8080
```

### 配置文件 (TOML)

```toml
app_name = "bamboo"
environment = "production"

[logging]
level = "info"
json_format = true
stdout = true
file = true
file_path = "/var/log/bamboo/app.log"
ansi_colors = true
include_target = true
include_thread_id = false
include_line_number = true

[logging.rotation]
strategy = "daily"
max_files = 7

[logging.module_levels]
bamboo_server = "debug"
bamboo_gateway = "info"
bamboo_skill = "warn"

[metrics]
enabled = true
host = "0.0.0.0"
port = 9090
prefix = "bamboo"
prometheus_enabled = true

[metrics.labels]
region = "us-east-1"
version = "1.0.0"

[health]
enabled = true
host = "0.0.0.0"
port = 8080
health_path = "/health"
ready_path = "/ready"
metrics_path = "/metrics"
timeout_seconds = 5
```

### 配置文件 (JSON)

```json
{
  "app_name": "bamboo",
  "environment": "production",
  "logging": {
    "level": "info",
    "json_format": true,
    "file": true,
    "file_path": "/var/log/bamboo/app.log",
    "module_levels": {
      "bamboo_server": "debug",
      "bamboo_gateway": "info"
    }
  },
  "metrics": {
    "port": 9090,
    "labels": {
      "region": "us-east-1"
    }
  },
  "health": {
    "port": 8080
  }
}
```

## 健康检查端点

### GET /health

返回应用整体健康状态：

```json
{
  "status": "healthy",
  "app_name": "bamboo",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": [
    {
      "name": "database",
      "status": "healthy",
      "message": "Database connection OK",
      "response_time_ms": 10,
      "last_checked": "2024-01-15T10:30:00Z"
    }
  ]
}
```

### GET /ready

返回就绪状态：

```json
{
  "ready": true,
  "timestamp": "2024-01-15T10:30:00Z",
  "dependencies": {
    "database": true,
    "cache": true
  }
}
```

### GET /metrics

返回 Prometheus 格式的指标：

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

## 可用指标

### HTTP 请求指标

| 指标名 | 类型 | 描述 |
|--------|------|------|
| `bamboo_http_requests_total` | Counter | HTTP 请求总数 |
| `bamboo_http_request_duration_seconds` | Histogram | HTTP 请求持续时间 |
| `bamboo_http_responses_total` | Counter | HTTP 响应总数 |

### Session 指标

| 指标名 | 类型 | 描述 |
|--------|------|------|
| `bamboo_sessions_active` | Gauge | 活跃 Session 数 |
| `bamboo_sessions_created_total` | Counter | Session 创建总数 |
| `bamboo_sessions_closed_total` | Counter | Session 关闭总数 |

### Agent 指标

| 指标名 | 类型 | 描述 |
|--------|------|------|
| `bamboo_agent_calls_total` | Counter | Agent 调用总数 |
| `bamboo_agent_call_duration_seconds` | Histogram | Agent 调用持续时间 |
| `bamboo_agent_errors_total` | Counter | Agent 错误总数 |

### Tool 指标

| 指标名 | 类型 | 描述 |
|--------|------|------|
| `bamboo_tool_executions_total` | Counter | Tool 执行总数 |
| `bamboo_tool_execution_duration_seconds` | Histogram | Tool 执行持续时间 |
| `bamboo_tool_errors_total` | Counter | Tool 错误总数 |

### LLM 指标

| 指标名 | 类型 | 描述 |
|--------|------|------|
| `bamboo_llm_tokens_total` | Counter | LLM Token 消耗总数 |
| `bamboo_llm_request_duration_seconds` | Histogram | LLM 请求持续时间 |

## 模块分级日志

支持为不同模块设置不同的日志级别：

```rust
let config = Config::default()
    .with_module_level("bamboo_server", "debug")
    .with_module_level("bamboo_gateway", "info")
    .with_module_level("bamboo_skill", "warn")
    .with_module_level("bamboo_mcp", "error");
```

使用时指定目标：

```rust
tracing::info!(target: "bamboo_server", "Server message");
tracing::debug!(target: "bamboo_gateway", "Gateway message");
tracing::warn!(target: "bamboo_skill", "Skill message");
```

## 日志轮转

支持按时间轮转：

```rust
let config = Config::default()
    .with_log_file("/var/log/bamboo/app.log")
    .with_rotation(RotationConfig {
        strategy: RotationStrategy::Daily,
        max_files: 7,
        ..Default::default()
    });
```

轮转策略：
- `Minutely` - 每分钟
- `Hourly` - 每小时
- `Daily` - 每天（默认）
- `Never` - 不轮转

## 动态调整日志级别

```rust
// 提高日志级别进行调试
obs.update_log_level("debug").await?;

// 恢复日志级别
obs.update_log_level("info").await?;
```

## 错误上下文

```rust
use bamboo_observability::{ErrorContext, Context};

let result = some_operation()
    .with_request_id("req-123")
    .with_session_id("sess-456");

// 或使用完整的上下文
let ctx = ErrorContext::new()
    .request_id("req-123")
    .session_id("sess-456")
    .agent_id("agent-789")
    .with_field("custom_key", "custom_value");
```

## 与 Bamboo Server 集成

在 `bamboo-server` 的 `main.rs` 中：

```rust
use bamboo_observability::prelude::*;
use bamboo_observability::{
    HttpMetrics, SessionMetrics, AgentMetrics, ToolMetrics, LlmMetrics,
    simple_check, database_check,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化观测性
    let config = Config::from_env()?;
    let obs = Observability::init(config).await?;
    
    // 注册健康检查
    obs.register_health_check("memory", simple_check("memory", true)).await;
    obs.register_health_check("database", database_check("db", check_db)).await;
    
    // 启动健康服务器
    obs.start_health_server().await?;
    
    // 启动主服务
    start_server(obs).await
}

async fn start_server(obs: Observability) -> anyhow::Result<()> {
    // HTTP 中间件记录指标
    app.wrap_fn(|req, srv| {
        let method = req.method().to_string();
        let path = req.path().to_string();
        
        HttpMetrics::record_request(&method, &path);
        
        async move {
            let start = Instant::now();
            let res = srv.call(req).await;
            let duration = start.elapsed();
            
            if let Ok(ref resp) = res {
                HttpMetrics::record_response(&method, &path, resp.status().as_u16());
                HttpMetrics::record_duration(&method, &path, duration.as_secs_f64());
            }
            
            res
        }
    })
    // ...
}
```

## 依赖

- `tracing` - 结构化日志
- `tracing-subscriber` - 日志订阅者
- `tracing-appender` - 日志文件写入
- `metrics` - 指标收集
- `metrics-exporter-prometheus` - Prometheus 导出
- `axum` - HTTP 服务器

## License

MIT
