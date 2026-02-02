# Bamboo Observability 项目结构

```
bamboo-observability/
├── Cargo.toml                    # 项目配置
├── README.md                     # 项目说明文档
├── INTEGRATION.md               # 集成指南
├── config.example.toml          # 配置示例
│
├── src/
│   ├── lib.rs                   # 库入口，Observability 主结构
│   ├── config.rs                # 配置管理模块
│   ├── config/
│   │   └── bamboo_integration.rs  # bamboo-config 集成
│   ├── error.rs                 # 统一错误类型和上下文
│   ├── logging/
│   │   └── mod.rs               # 结构化日志实现
│   ├── metrics/
│   │   └── mod.rs               # 指标收集实现
│   └── health/
│       └── mod.rs               # 健康检查实现
│
└── examples/
    ├── usage.rs                 # 基础使用示例
    ├── server_integration.rs    # Actix-web 集成示例
    └── simple_test.rs           # 简单测试示例
```

## 核心组件

### 1. Observability (src/lib.rs)
- 统一的观测性句柄
- 管理 LogManager、MetricsCollector、HealthServer 的生命周期
- 提供动态配置更新功能

### 2. Config (src/config.rs)
- 支持环境变量、TOML/JSON/YAML 配置文件
- 日志级别、格式、轮转配置
- 模块分级日志配置
- 指标和健康检查端口配置

### 3. Logging (src/logging/mod.rs)
- 基于 tracing 的结构化日志
- JSON 和文本两种输出格式
- 支持文件日志和日志轮转
- 动态日志级别调整
- 提供带上下文的 Span 创建函数

### 4. Metrics (src/metrics/mod.rs)
- 基于 metrics 库的指标收集
- Prometheus 格式导出
- 内置指标类型：
  - HttpMetrics: HTTP 请求相关
  - SessionMetrics: Session 相关
  - AgentMetrics: Agent 调用相关
  - ToolMetrics: Tool 执行相关
  - LlmMetrics: LLM 调用相关

### 5. Health (src/health/mod.rs)
- HTTP 健康检查端点
- /health - 整体健康状态
- /ready - 就绪检查
- /metrics - Prometheus 指标
- /live - 存活检查
- 支持自定义健康检查

### 6. Error (src/error.rs)
- 统一的 ObservabilityError 类型
- ErrorContext 用于追踪上下文
- 支持 request_id, session_id, agent_id 等
- Context trait 用于便捷地添加上下文

## 与 Bamboo 项目集成

### bamboo-server
```rust
use bamboo_observability::prelude::*;

let obs = Observability::init(Config::from_env()?).await?;
obs.start_health_server().await?;
```

### bamboo-core
```rust
use bamboo_observability::{AgentMetrics, create_agent_span};

AgentMetrics::record_call(agent_id);
```

### bamboo-gateway
```rust
use bamboo_observability::{HttpMetrics, SessionMetrics};

HttpMetrics::record_request(method, path);
SessionMetrics::record_created();
```

### bamboo-skill
```rust
use bamboo_observability::{ToolMetrics, LlmMetrics};

ToolMetrics::record_execution(tool_name);
LlmMetrics::record_tokens(model, input, output);
```

## 启动顺序

1. 加载配置（环境变量/配置文件）
2. 初始化 Observability
3. 注册健康检查
4. 启动健康检查服务器
5. 启动主服务

## 关闭顺序

1. 停止接收新请求
2. 关闭健康检查服务器
3. 关闭指标收集器
4. 关闭日志管理器
