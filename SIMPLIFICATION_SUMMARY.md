# Bamboo 消息系统简化完成总结

## 完成的工作

### 1. 删除 `bamboo-router` crate
- 从 workspace 中完全移除
- 该 crate 没有被任何其他 crate 使用

### 2. 删除 `bamboo-gateway` crate
- 将 WebSocket 功能合并到 `bamboo-server` 中
- 新的位置: `bamboo-server/src/websocket/`

### 3. 新的模块结构

```
bamboo-server/src/
├── websocket/
│   ├── mod.rs          # 模块导出
│   ├── protocol.rs     # WebSocket 协议定义 (ClientMessage, GatewayEvent)
│   ├── connection.rs   # 连接池管理
│   ├── session.rs      # 会话管理
│   ├── router.rs       # 消息路由
│   └── gateway.rs      # Gateway 主结构
├── event_bus.rs        # HTTP ↔ WebSocket 内部事件总线
├── state.rs            # AppState (集成 Gateway)
├── agent_runner.rs     # AgentRunner (基于 EventBus)
└── ...
```

### 4. 更新的依赖

**bamboo-server/Cargo.toml 变更:**
```toml
# 移除
- bamboo-gateway = { path = "../bamboo-gateway" }

# 添加 (从 bamboo-gateway 迁移过来的依赖)
+ tokio-tungstenite = "0.24"
+ dashmap = "6"
+ futures-util = "0.3"
+ uuid = { version = "1", features = ["v4", "serde"] }
+ anyhow = "1"
```

### 5. 代码变更

**state.rs:**
- 更新 `GatewayRef` 类型别名使用本地 `websocket::Gateway`
- 更新 `GatewayConfig` 导入路径
- 更新 `convert_event_to_gateway_event` 函数使用本地类型

**lib.rs:**
- 添加 `pub mod websocket;`
- 导出 `Gateway`, `GatewayConfig`, `GatewayError`

**main.rs:**
- 无需修改，继续使用 `state.gateway`

## 架构简化后的结构

```
┌─────────────────────────────────────────────────────────────┐
│                    Bamboo Server                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐          ┌──────────────────────────────┐ │
│  │  HTTP Server │          │   WebSocket Gateway          │ │
│  │  (Actix-web) │          │   (tokio-tungstenite)        │ │
│  └──────┬───────┘          └──────────────┬───────────────┘ │
│         │                                  │                 │
│         └──────────┬───────────────────────┘                 │
│                    ▼                                         │
│  ┌──────────────────────────────────────────┐               │
│  │           EventBus (内部事件总线)          │               │
│  │   - HTTP ↔ WebSocket 通信                 │               │
│  └──────────────────────────────────────────┘               │
│                    │                                         │
│         ┌─────────┴──────────┐                              │
│         ▼                    ▼                              │
│  ┌──────────────┐    ┌──────────────┐                       │
│  │ AgentRunner  │    │  LLM/Tool    │                       │
│  │ (EventBus驱动)│    │  (直接调用)   │                       │
│  └──────────────┘    └──────────────┘                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 编译状态

✅ `cargo check --package bamboo-server` 通过
- 无错误
- 只有未使用代码的警告（正常）

## 优势

1. **减少 crate 数量**: 从 3 个消息相关 crate 减少到 1 个
2. **简化依赖关系**: 移除了 bamboo-gateway 和 bamboo-router 的外部依赖
3. **代码集中**: WebSocket 功能现在与 HTTP server 在同一 crate 中，更容易维护
4. **保留灵活性**: EventBus 仍然支持 HTTP 和 WebSocket 之间的通信

## 注意事项

1. 一些 WebSocket 模块中的代码当前未被使用（警告），这是预期的，因为它们是完整的实现，但当前只使用了 Gateway 的核心功能
2. `bamboo-observability` crate 有独立的编译错误，与本次修改无关
