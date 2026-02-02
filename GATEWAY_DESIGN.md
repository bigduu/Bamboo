# Bamboo Gateway 设计文档

## 目标
实现类似 OpenClaw 的 WebSocket 控制平面，管理多 Sessions 和实时消息推送。

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                        Gateway                               │
│                  (WebSocket Server)                          │
│                    ws://127.0.0.1:18790                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Session    │  │   Message    │  │   Channel    │      │
│  │   Manager    │  │   Router     │  │   Manager    │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                 │                 │               │
│         └─────────────────┼─────────────────┘               │
│                           │                                 │
│  ┌────────────────────────┴─────────────────────────────┐   │
│  │              WebSocket Connection Pool               │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐   │   │
│  │  │ Client  │ │ Client  │ │ Client  │ │ Client  │   │   │
│  │  │  #1     │ │  #2     │ │  #3     │ │  #n     │   │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘   │   │
│  └────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 核心组件

### 1. Gateway (bamboo-gateway crate)
```rust
pub struct Gateway {
    config: GatewayConfig,
    session_manager: Arc<SessionManager>,
    connection_pool: Arc<ConnectionPool>,
    message_router: Arc<MessageRouter>,
}

pub struct GatewayConfig {
    pub bind: String,        // "127.0.0.1:18790"
    pub auth_token: Option<String>,
    pub max_connections: usize,
    pub heartbeat_interval_secs: u64,
}

impl Gateway {
    pub async fn run(self) -> Result<()>;
    pub async fn broadcast(&self, event: GatewayEvent);
    pub async fn send_to(&self, session_id: &str, event: GatewayEvent);
}
```

### 2. Session Manager
```rust
pub struct SessionManager {
    sessions: DashMap<String, Session>,
    storage: Arc<dyn SessionStorage>,
}

pub struct Session {
    pub id: String,
    pub user_id: String,
    pub connection: Option<ConnectionHandle>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

impl SessionManager {
    pub async fn create(&self, user_id: &str) -> Session;
    pub async fn get(&self, id: &str) -> Option<Session>;
    pub async fn attach_connection(&self, session_id: &str, conn: ConnectionHandle);
    pub async fn detach_connection(&self, session_id: &str);
    pub async fn cleanup_inactive(&self, max_age: Duration);
}
```

### 3. Message Router
```rust
pub struct MessageRouter;

impl MessageRouter {
    pub async fn route(&self, msg: IncomingMessage) -> RouteResult;
    pub async fn handle_chat(&self, session: &Session, msg: ChatMessage);
    pub async fn handle_command(&self, session: &Session, cmd: Command);
}

pub enum IncomingMessage {
    Chat(ChatMessage),
    Command(Command),
    Ping,
}

pub enum RouteResult {
    ToAgent(Session, ChatMessage),
    ToChannel(ChannelId, Message),
    Response(GatewayEvent),
}
```

### 4. WebSocket 协议
```rust
// 客户端 -> Gateway
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { session_id: Option<String>, auth: Option<String> },
    Chat { content: String, session_id: String },
    Command { name: String, args: Value },
    Ping { timestamp: i64 },
}

// Gateway -> 客户端
#[derive(Serialize, Deserialize)]
pub enum GatewayEvent {
    Connected { session_id: String },
    AgentToken { session_id: String, token: String },
    AgentToolStart { session_id: String, tool: String },
    AgentToolComplete { session_id: String, tool: String, result: String },
    AgentComplete { session_id: String, usage: TokenUsage },
    Error { code: String, message: String },
    Pong { timestamp: i64 },
}
```

## 集成

### 与 bamboo-server 关系
```rust
// bamboo-server 启动时同时启动 Gateway
let gateway = Gateway::new(gateway_config);
let gateway_handle = tokio::spawn(gateway.run());

// Server handlers 通过 Gateway API 发送事件
gateway.broadcast(GatewayEvent::AgentToken { ... }).await;
```

### Session 生命周期
```
1. 客户端连接 WebSocket
2. 发送 Connect { session_id? }
3. Gateway 创建/恢复 Session
4. 返回 Connected { session_id }
5. 客户端发送 Chat/Command
6. Gateway 路由到 Agent
7. Agent 流式响应通过 Gateway 推送
8. 连接断开 -> Session 保持（可重连）
```

## Crate: bamboo-gateway

依赖:
- tokio-tungstenite - WebSocket
- tokio - 异步运行时
- dashmap - 并发 Session 存储
- serde - 序列化
- bamboo-core - 共享类型

## 文件结构
```
crates/bamboo-gateway/
├── Cargo.toml
└── src/
    ├── lib.rs          # 公共导出
    ├── gateway.rs      # Gateway 主结构
    ├── session.rs      # SessionManager
    ├── router.rs       # MessageRouter
    ├── protocol.rs     # WebSocket 消息协议
    ├── connection.rs   # Connection pool
    └── handlers.rs     # 消息处理器
```
