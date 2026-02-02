# Bamboo Gateway 集成到 Server 设计方案

## 目标
将 bamboo-gateway 集成到 bamboo-server，实现：
1. HTTP API (REST + SSE) - 已有
2. WebSocket Gateway - 新增
3. 两者共享 Session 和 Agent 逻辑

## 架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                      Bamboo Server                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐          ┌──────────────────────────────────┐ │
│  │  HTTP Server │          │      WebSocket Gateway           │ │
│  │  (Actix-web) │          │    (tokio-tungstenite)           │ │
│  │              │          │                                  │ │
│  │ /api/v1/chat │          │  ws://127.0.0.1:18790            │ │
│  │ /api/v1/...  │          │                                  │ │
│  └──────┬───────┘          └──────────────┬───────────────────┘ │
│         │                                  │                     │
│         │          ┌───────────────────────┘                     │
│         │          │                                             │
│         ▼          ▼                                             │
│  ┌──────────────────────────────────────────┐                   │
│  │           AppState (共享状态)              │                   │
│  │  ┌──────────────┐  ┌──────────────────┐  │                   │
│  │  │ SessionStore │  │ LLM Provider     │  │                   │
│  │  │ (DashMap)    │  │ (OpenAiProvider) │  │                   │
│  │  └──────────────┘  └──────────────────┘  │                   │
│  │  ┌──────────────┐  ┌──────────────────┐  │                   │
│  │  │ SkillManager │  │ EventBus         │  │                   │
│  │  │ (热重载)      │  │ (消息总线)        │  │                   │
│  │  └──────────────┘  └──────────────────┘  │                   │
│  └──────────────────────────────────────────┘                   │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────────────────────────────────┐                   │
│  │           Agent Runner                   │                   │
│  │  (处理聊天请求 → 调用 LLM → 返回流)        │                   │
│  └──────────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

## 数据流

### HTTP API 流（已有）
```
Client → POST /api/v1/chat → Handler → AgentRunner → LLM → SSE Stream → Client
```

### WebSocket 流（新增）
```
Client → WebSocket Connect → Gateway → SessionManager
   ↓
Client → Chat Message → Gateway → EventBus → AgentRunner
   ↓
AgentRunner → EventBus → Gateway → Push to Client (WebSocket)
```

### 混合流（HTTP + WebSocket）
```
Client 可以通过 WebSocket 发送消息
        ↓
Gateway 转发到 AgentRunner
        ↓
AgentRunner 可以同时：
  - 通过 HTTP SSE 返回（兼容旧客户端）
  - 通过 WebSocket Push 返回（新客户端）
```

## 核心组件集成

### 1. AppState 扩展

```rust
// bamboo-server/src/state.rs
use bamboo_gateway::{Gateway, GatewayConfig, EventBus};

pub struct AppState {
    // 已有
    pub sessions: Arc<SessionStore>,
    pub llm: Arc<dyn LLMProvider>,
    pub skill_manager: Arc<SkillManager>,
    
    // 新增
    pub gateway: Arc<Gateway>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        // ... 初始化已有组件 ...
        
        // 初始化 Gateway
        let gateway_config = GatewayConfig {
            bind: config.gateway.bind.clone(),
            auth_token: config.gateway.auth_token.clone(),
            max_connections: config.gateway.max_connections,
            heartbeat_interval_secs: config.gateway.heartbeat_interval_secs,
        };
        let gateway = Arc::new(Gateway::new(gateway_config));
        
        // 初始化 EventBus
        let event_bus = Arc::new(EventBus::new());
        
        Self {
            sessions,
            llm,
            skill_manager,
            gateway,
            event_bus,
        }
    }
    
    /// 启动 Gateway（在单独的任务中）
    pub async fn start_gateway(self: Arc<Self>) {
        let gateway = self.gateway.clone();
        
        // 设置 Gateway 的事件处理器
        gateway.on_message(|msg| {
            self.handle_gateway_message(msg).await;
        });
        
        // 运行 Gateway
        tokio::spawn(async move {
            gateway.run().await.expect("Gateway failed");
        });
    }
    
    /// 处理来自 Gateway 的消息
    async fn handle_gateway_message(&self, msg: IncomingMessage) {
        match msg {
            IncomingMessage::Chat { session_id, content } => {
                // 通过 EventBus 触发 AgentRunner
                self.event_bus.publish(Event::ChatRequest {
                    session_id,
                    content,
                    reply_to: ReplyChannel::Gateway(session_id),
                }).await;
            }
            // ... 其他消息类型
        }
    }
    
    /// 发送事件到客户端（HTTP SSE 或 WebSocket）
    pub async fn send_event(&self, session_id: &str, event: Event) {
        // 尝试通过 Gateway 发送
        if let Err(_) = self.gateway.send_to(session_id, event.clone()).await {
            // 如果 Gateway 发送失败，说明是 HTTP 客户端
            // 通过 EventBus 让 HTTP handler 处理
            self.event_bus.publish(Event::HttpResponse {
                session_id: session_id.to_string(),
                event,
            }).await;
        }
    }
}
```

### 2. EventBus 设计

```rust
// bamboo-server/src/event_bus.rs
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum Event {
    ChatRequest {
        session_id: String,
        content: String,
        reply_to: ReplyChannel,
    },
    ChatResponse {
        session_id: String,
        chunk: ChatChunk,
    },
    HttpResponse {
        session_id: String,
        event: Event,
    },
    SessionCreated {
        session_id: String,
    },
    SessionClosed {
        session_id: String,
    },
}

#[derive(Debug, Clone)]
pub enum ReplyChannel {
    Gateway(String),  // WebSocket session_id
    Http(String),     // HTTP request_id
}

pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }
    
    pub async fn publish(&self, event: Event) -> Result<(), EventError> {
        self.sender.send(event)?;
        Ok(())
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
```

### 3. AgentRunner 集成

```rust
// bamboo-server/src/agent_runner.rs
pub struct AgentRunner {
    state: Arc<AppState>,
}

impl AgentRunner {
    pub async fn run(&self) {
        let mut rx = self.state.event_bus.subscribe();
        
        while let Ok(event) = rx.recv().await {
            match event {
                Event::ChatRequest { session_id, content, reply_to } => {
                    self.handle_chat_request(session_id, content, reply_to).await;
                }
                _ => {}
            }
        }
    }
    
    async fn handle_chat_request(
        &self,
        session_id: String,
        content: String,
        reply_to: ReplyChannel,
    ) {
        // 获取或创建 Session
        let session = self.state.sessions.get_or_create(&session_id).await;
        
        // 构建 LLM 请求
        let request = ChatRequest::new(&session.model)
            .with_messages(session.messages.clone())
            .with_tools(self.state.skill_manager.get_tools());
        
        // 调用 LLM
        let mut stream = self.state.llm.chat_stream(request).await.unwrap();
        
        // 流式返回结果
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chat_chunk) => {
                    // 保存到 Session
                    session.add_chunk(&chat_chunk).await;
                    
                    // 发送到客户端
                    match &reply_to {
                        ReplyChannel::Gateway(sid) => {
                            self.state.gateway.send_to(sid, Event::ChatResponse {
                                session_id: session_id.clone(),
                                chunk: chat_chunk,
                            }).await.ok();
                        }
                        ReplyChannel::Http(_) => {
                            // HTTP 通过 SSE，由 HTTP handler 处理
                            self.state.event_bus.publish(Event::HttpResponse {
                                session_id: session_id.clone(),
                                event: Event::ChatResponse {
                                    session_id: session_id.clone(),
                                    chunk: chat_chunk,
                                },
                            }).await.ok();
                        }
                    }
                }
                Err(e) => {
                    // 发送错误
                }
            }
        }
    }
}
```

### 4. HTTP Handler 适配

```rust
// bamboo-server/src/handlers/stream.rs
pub async fn stream_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let session_id = path.into_inner();
    let (tx, rx) = mpsc::channel(100);
    
    // 订阅 EventBus
    let mut event_rx = state.event_bus.subscribe();
    let session_id_clone = session_id.clone();
    
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let Event::HttpResponse { session_id: sid, event } = event {
                if sid == session_id_clone {
                    if let Event::ChatResponse { chunk, .. } = event {
                        let _ = tx.send(Ok::<_, Error>(chunk_to_bytes(chunk))).await;
                    }
                }
            }
        }
    });
    
    // 返回 SSE
    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(ReceiverStream::new(rx))
}
```

### 5. Gateway 适配

```rust
// bamboo-gateway/src/lib.rs (修改)
pub struct Gateway {
    config: GatewayConfig,
    session_manager: Arc<SessionManager>,
    connection_pool: Arc<ConnectionPool>,
    message_handler: Arc<RwLock<Box<dyn Fn(IncomingMessage) + Send + Sync>>>,
}

impl Gateway {
    pub fn on_message<F>(&self, handler: F)
    where
        F: Fn(IncomingMessage) + Send + Sync + 'static,
    {
        *self.message_handler.write().unwrap() = Box::new(handler);
    }
    
    pub async fn send_to(&self, session_id: &str, event: Event) -> Result<(), GatewayError> {
        if let Some(conn) = self.connection_pool.get(session_id).await {
            let msg = serde_json::to_string(&event)?;
            conn.send(Message::Text(msg)).await?;
            Ok(())
        } else {
            Err(GatewayError::SessionNotFound)
        }
    }
}
```

## 配置更新

```toml
# bamboo.toml
[server]
host = "127.0.0.1"
port = 8081

[gateway]
enabled = true
bind = "127.0.0.1:18790"
auth_token = "optional-secret"
max_connections = 1000
heartbeat_interval_secs = 30

[llm]
default_provider = "openai"
```

## 启动流程

```rust
// bamboo-server/src/main.rs
#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = Config::load().await?;
    
    // 创建共享状态
    let state = Arc::new(AppState::new(config).await);
    
    // 启动 Gateway（如果启用）
    if config.gateway.enabled {
        state.clone().start_gateway().await;
    }
    
    // 启动 AgentRunner
    let runner = AgentRunner::new(state.clone());
    tokio::spawn(async move {
        runner.run().await;
    });
    
    // 启动 HTTP Server
    run_server(state, config.server.port).await
}
```

## 集成步骤

1. **创建 EventBus 模块** - 消息总线
2. **扩展 AppState** - 添加 Gateway 和 EventBus
3. **修改 AgentRunner** - 订阅 EventBus 处理请求
4. **适配 Gateway** - 添加消息处理器和发送方法
5. **修改 HTTP handlers** - 通过 EventBus 接收响应
6. **更新 main.rs** - 启动 Gateway 和 AgentRunner
7. **更新配置** - 添加 gateway 配置段

交给 Codex 实现此集成方案。