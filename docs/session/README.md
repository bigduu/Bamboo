# Bamboo Session Storage

Bamboo AI Agent 的 Session 持久化存储系统。

## 功能特性

- ✅ **会话元数据存储** - 创建时间、最后活动、用户ID、状态等
- ✅ **消息历史存储** - 完整的对话历史
- ✅ **事件流存储** - AgentEvent 的追加写入（JSONL 格式）
- ✅ **多维度索引** - 按用户、状态、时间快速查询
- ✅ **自动清理** - 过期会话自动清理
- ✅ **断线重连** - 连接断开后 Session 保持，支持重连
- ✅ **内存缓存** - 活跃会话内存缓存，提高性能

## 架构设计

```
┌────────────────────────────────────────────────────────────────┐
│                        Gateway Layer                            │
│                    ┌──────────────────┐                        │
│                    │  SessionManager  │                        │
│                    │  - 内存缓存管理   │                        │
│                    │  - 连接生命周期   │                        │
│                    │  - 断线重连支持   │                        │
│                    └────────┬─────────┘                        │
└─────────────────────────────┼──────────────────────────────────┘
                              │
┌─────────────────────────────┼──────────────────────────────────┐
│                    Storage Layer                                │
│                    ┌────────┴─────────┐                        │
│                    │   JsonlStorage   │                        │
│                    │  - 文件存储      │                        │
│                    │  - 内存索引      │                        │
│                    │  - 自动清理      │                        │
│                    └──────────────────┘                        │
└────────────────────────────────────────────────────────────────┘
```

## 存储结构

```
~/.bamboo/sessions/
├── sessions/
│   ├── <session_id>.json      # 会话元数据和消息
│   └── ...
├── events/
│   ├── <session_id>.jsonl     # 事件流（追加写入）
│   └── ...
└── index/
    └── (预留)
```

## 快速开始

### 添加依赖

```toml
[dependencies]
bamboo-session = { path = "../bamboo/crates/bamboo-session" }
```

### 基本使用

```rust
use bamboo_session::{
    JsonlStorage, JsonlStorageConfig,
    SessionStorage, Session, Message,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建存储
    let config = JsonlStorageConfig::new("~/.bamboo/sessions")
        .with_default_ttl(86400); // 24小时过期
    
    let storage = JsonlStorage::new(config).await?;
    
    // 创建会话
    let session = Session::new("session-001")
        .with_user_id("user-123")
        .with_title("My Chat");
    
    storage.create_session(&session).await?;
    
    // 添加消息
    storage.append_message(
        &session.metadata.id,
        Message::user("Hello!"),
    ).await?;
    
    // 加载会话
    let loaded = storage.load_session("session-001").await?;
    
    Ok(())
}
```

### SessionManager 使用

```rust
use bamboo_session::{
    SessionManager, SessionManagerConfig,
    Message, AgentEvent,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建存储
    let storage = Arc::new(JsonlStorage::new(config).await?);
    
    // 创建 SessionManager
    let manager = SessionManager::new(
        SessionManagerConfig::default(),
        storage,
    ).await?;
    
    // 创建会话
    let session = manager.create_session(
        Some("user-123".to_string()),
        Some("Chat Title".to_string()),
    ).await?;
    
    // 连接会话（返回事件接收通道）
    let mut event_rx = manager.connect_session(
        &session.metadata.id,
        "conn-001".to_string(),
    ).await?;
    
    // 添加消息
    manager.add_message(
        &session.metadata.id,
        Message::user("Hello!"),
    ).await?;
    
    // 发送事件
    manager.append_event(
        &session.metadata.id,
        AgentEvent::token("Hi!"),
    ).await?;
    
    // 接收事件
    while let Some(event) = event_rx.recv().await {
        println!("{:?}", event);
    }
    
    Ok(())
}
```

## API 接口

### SessionStorage

```rust
#[async_trait]
pub trait SessionStorage: Storage {
    async fn create_session(&self, session: &Session) -> StorageResult<()>;
    async fn load_session(&self, session_id: &str) -> StorageResult<Option<Session>>;
    async fn delete_session(&self, session_id: &str) -> StorageResult<()>;
    
    async fn list_sessions(&self, filter: &SessionFilter) -> StorageResult<SessionListResult>;
    async fn list_user_sessions(&self, user_id: &str) -> StorageResult<Vec<SessionMetadata>>;
    
    async fn get_session_history(&self, session_id: &str) -> StorageResult<Vec<Message>>;
    async fn append_message(&self, session_id: &str, message: &Message) -> StorageResult<()>;
    
    async fn update_session_state(&self, session_id: &str, state: SessionState) -> StorageResult<()>;
    async fn cleanup_expired_sessions(&self) -> StorageResult<u64>;
}
```

### SessionManager

```rust
impl SessionManager {
    // 会话生命周期
    pub async fn create_session(&self, user_id: Option<String>, title: Option<String>) -> StorageResult<Session>;
    pub async fn get_session(&self, session_id: &str) -> StorageResult<Option<Session>>;
    pub async fn close_session(&self, session_id: &str) -> StorageResult<()>;
    pub async fn delete_session(&self, session_id: &str) -> StorageResult<()>;
    
    // 连接管理（支持断线重连）
    pub async fn connect_session(&self, session_id: &str, connection_id: String) 
        -> StorageResult<mpsc::UnboundedReceiver<AgentEvent>>;
    pub async fn disconnect_session(&self, session_id: &str) -> StorageResult<()>;
    pub async fn reconnect_session(&self, session_id: &str, new_connection_id: String) 
        -> StorageResult<mpsc::UnboundedReceiver<AgentEvent>>;
    
    // 数据和事件
    pub async fn add_message(&self, session_id: &str, message: Message) -> StorageResult<()>;
    pub async fn append_event(&self, session_id: &str, event: AgentEvent) -> StorageResult<()>;
    
    // 查询
    pub async fn list_sessions(&self, filter: &SessionFilter) -> StorageResult<SessionListResult>;
    pub async fn get_session_history(&self, session_id: &str) -> StorageResult<Vec<Message>>;
}
```

## 运行示例

```bash
# 基本用法示例
cargo run --example basic_usage

# SessionManager 示例
cargo run --example session_manager

# Gateway 集成示例
cargo run --example gateway_integration
```

## 配置选项

### JsonlStorageConfig

```rust
let config = JsonlStorageConfig::new("~/.bamboo/sessions")
    .with_default_ttl(86400)        // 默认 24 小时过期
    .with_max_sessions(10000)        // 最大 10000 个会话
    .with_cleanup_interval(3600)     // 每小时清理一次
    .disable_index();                // 禁用内存索引（节省内存）
```

### SessionManagerConfig

```rust
let config = SessionManagerConfig::default()
    .with_idle_timeout(300)          // 5分钟空闲超时
    .with_disconnect_retention(3600) // 1小时断开保留（支持重连）
    .with_max_sessions(1000);        // 最大 1000 个活跃会话
```

## 与 Bamboo Gateway 集成

SessionManager 设计用于与 Bamboo Gateway 无缝集成：

```rust
pub struct Gateway {
    session_manager: Arc<SessionManager>,
}

impl Gateway {
    async fn handle_chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let session = self.session_manager
            .get_or_create_session(req.session_id, req.user_id)
            .await?;
        
        // 添加消息...
        
        Ok(ChatResponse {
            session_id: session.metadata.id,
            stream_url: format!("/api/v1/stream/{}", session.metadata.id),
        })
    }
    
    async fn handle_stream(&self, session_id: String, conn_id: String) -> Result<EventStream> {
        let event_rx = self.session_manager
            .connect_session(&session_id, conn_id)
            .await?;
        
        Ok(EventStream::from(event_rx))
    }
    
    async fn handle_reconnect(&self, session_id: String, conn_id: String) -> Result<EventStream> {
        let event_rx = self.session_manager
            .reconnect_session(&session_id, conn_id)
            .await?;
        
        Ok(EventStream::from(event_rx))
    }
}
```

## 特性

### 已实现的特性

- ✅ 基础存储 trait (Storage)
- ✅ 增强 Session 存储 (SessionStorage)
- ✅ 事件存储 (EventStorage)
- ✅ 索引存储 (IndexStorage)
- ✅ JSONL 文件存储实现 (JsonlStorage)
- ✅ 内存索引（按用户、状态、时间）
- ✅ SessionManager（连接管理、断线重连）
- ✅ 自动清理过期会话
- ✅ 完整的示例代码
- ✅ 单元测试

### 待实现的特性

- ⏳ SQLite 后端
- ⏳ Redis 缓存层
- ⏳ 分布式锁（多实例支持）
- ⏳ 数据迁移工具
- ⏳ 指标和监控

## 许可证

MIT
