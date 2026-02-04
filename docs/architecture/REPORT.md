# Bamboo 架构图创建完成报告

## 📁 创建的文件

所有架构图文件已创建在 `~/workspace/bamboo/docs/architecture/diagrams/` 目录下：

| 文件 | 路径 | 描述 |
|------|------|------|
| 系统架构图 | `~/workspace/bamboo/docs/architecture/diagrams/system_architecture.mmd` | 展示10个crate的层次关系和依赖 |
| 组件图 | `~/workspace/bamboo/docs/architecture/diagrams/component_diagram.mmd` | 详细展示各组件内部结构和接口 |
| 数据流图 | `~/workspace/bamboo/docs/architecture/diagrams/data_flow.mmd` | 请求从客户端到LLM的完整流程 |
| 部署图 | `~/workspace/bamboo/docs/architecture/diagrams/deployment_diagram.mmd` | 开发环境和生产环境的部署结构 |
| 架构说明 | `~/workspace/bamboo/docs/architecture/README.md` | 架构总览和技术选型说明 |

## 🏗️ 架构覆盖的组件

### 1. 客户端层 (3个)
- **bamboo-cli**: 命令行客户端，支持交互式聊天和配置管理
- **bamboo-tui**: 终端UI客户端，基于ratatui的图形界面
- **Web客户端**: 浏览器或扩展，通过HTTP/WebSocket连接

### 2. 服务层 (4个核心组件)
- **HTTP Server (bamboo-server)**: 基于actix-web的REST API服务
  - Router: 路由分发
  - Handlers: chat, stream, history, config, health, stop
- **Gateway**: WebSocket网关，管理长连接
  - ConnectionPool: 连接池管理
  - SessionManager: 会话管理
  - MessageRouter: 消息路由
- **AgentRunner**: Agent执行器，处理对话逻辑
- **EventBus**: 事件总线，基于tokio broadcast

### 3. 核心层 (3个crate)
- **bamboo-core**: 核心类型定义
  - types: Message, ToolCall, Content等
  - agent: AgentLoop, AgentConfig
  - tools: ToolExecutor trait
  - chat: ChatRequest/Response
  - storage: JsonlStorage
- **bamboo-session**: 会话存储管理（高级功能）
- **bamboo-config**: 配置管理，支持热重载

### 4. 能力层 (4个crate)
- **bamboo-llm**: LLM Provider抽象
  - OpenAIProvider: OpenAI兼容API
  - CopilotProvider: GitHub Copilot
  - ForwardProvider: 请求转发
  - AuthManager: 认证管理
- **bamboo-skill**: Skill管理系统
  - SkillParser: SKILL.md解析
  - SkillWatcher: 文件监听热重载
  - SkillManager: Skill生命周期管理
- **bamboo-tool**: 工具执行引擎
  - ToolExecutor: 工具执行trait
  - ToolRegistry: 工具注册表
- **bamboo-mcp**: MCP客户端
  - FilesystemTool: 文件操作
  - CommandTool: 命令执行

### 5. 存储层
- **JsonlStorage**: JSONL文件存储
- **SessionStore**: 会话数据存储
- **EventStore**: 事件流存储

## 🔄 主要交互流程

### HTTP 请求流程
```
Client → POST /api/v1/chat → Server → EventBus → AgentRunner → LLM Provider → 外部LLM
                    ↓
              返回session_id
                    ↓
Client → GET /api/v1/stream → Server → SSE流式响应
```

### WebSocket 请求流程
```
Client → WebSocket连接 → Gateway → EventBus → AgentRunner → LLM Provider
                                              ↓
Gateway ← WebSocket消息 ← EventBus ← 响应
```

### 工具调用流程
```
AgentRunner → LLM → ToolCall响应
                  ↓
AgentRunner → ToolExecutor → 执行工具
                  ↓
AgentRunner → LLM (带工具结果) → 最终响应
```

## 🛠️ 关键技术决策

### 1. 框架选型
| 组件 | 技术 | 理由 |
|------|------|------|
| HTTP服务 | actix-web | 高性能、成熟稳定、异步支持好 |
| WebSocket | tokio-tungstenite | 与tokio集成好、轻量级 |
| 序列化 | serde | Rust标准、生态完善 |
| 异步运行时 | tokio | Rust异步生态事实标准 |

### 2. 架构模式
- **事件驱动**: 使用EventBus解耦组件，支持HTTP和WebSocket统一处理
- **分层架构**: 客户端→服务→核心→能力→外部服务，职责清晰
- **Trait抽象**: LLMProvider、ToolExecutor等使用trait定义接口，便于扩展

### 3. 存储设计
- **JSONL格式**: 追加写入，便于审计和故障排查
- **文件存储**: 简单、无需额外依赖，适合个人使用
- **可选PostgreSQL**: 生产环境可替换为关系型数据库

### 4. 配置管理
- **分层配置**: 默认值 < 配置文件 < 环境变量 < 命令行参数
- **热重载**: 配置文件变更自动生效（可选）
- **环境变量**: 支持 `${VAR}` 语法

### 5. 扩展性设计
- **Skill系统**: YAML定义工具，支持热重载
- **MCP协议**: 标准化工具接口，可与外部工具集成
- **多Provider**: 支持OpenAI、Copilot、本地LLM等多种后端

## 📊 架构图预览

可以使用以下工具查看Mermaid图表：
1. **VSCode**: 安装 Markdown Preview Mermaid Support 扩展
2. **在线工具**: https://mermaid.live
3. **命令行**: 使用 mermaid-cli 生成PNG/SVG

```bash
# 安装 mermaid-cli
npm install -g @mermaid-js/mermaid-cli

# 生成 PNG
mmdc -i system_architecture.mmd -o system_architecture.png

# 生成 SVG
mmdc -i system_architecture.mmd -o system_architecture.svg
```

## 📝 后续建议

1. **完善组件文档**: 为每个crate添加详细的API文档
2. **接口定义**: 明确各组件间的接口契约
3. **性能基准**: 建立性能测试基准
4. **监控指标**: 定义关键监控指标和告警规则
5. **安全审计**: 审查认证、授权和数据安全
