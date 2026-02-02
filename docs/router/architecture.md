# Bamboo Message Router Architecture

```mermaid
flowchart TB
    subgraph Clients["Clients"]
        C1[Client 1]
        C2[Client 2]
        C3[Client N]
    end

    subgraph Gateway["Gateway Layer"]
        WS[WebSocket Handler]
        GI[GatewayIntegration]
    end

    subgraph MessageBus["Message Bus (Core)"]
        MB[MessageBus<br/>DashMap + mpsc]
        
        subgraph Topics["Topics"]
            T1[gateway:input]
            T2[agent:input]
            T3[agent:output]
            T4[command:input]
            T5[command:output]
            T6[system:input]
            T7[system:output]
            T8[gateway:output:*]
        end
    end

    subgraph Router["Message Router"]
        MR[MessageRouter]
        SR[SmartRouter]
    end

    subgraph Handlers["Message Handlers"]
        CH[ChatHandler]
        CMH[CommandHandler]
        SH[SystemHandler]
    end

    subgraph Processors["Processors"]
        AL[Agent Loop]
        CP[Command Processor]
        SP[System Processor]
    end

    %% Inbound flow
    C1 -->|WebSocket| WS
    C2 -->|WebSocket| WS
    C3 -->|WebSocket| WS
    WS --> GI
    GI -->|publish| T1
    
    T1 --> MR
    MR -->|Chat| CH
    MR -->|Command| CMH
    MR -->|System| SH
    
    CH -->|publish| T2
    CMH -->|publish| T4
    SH -->|publish| T6
    
    T2 --> AL
    T4 --> CP
    T6 --> SP
    
    %% Outbound flow
    AL -->|publish| T3
    CP -->|publish| T5
    SP -->|publish| T7
    
    T3 --> MR
    T5 --> MR
    T7 --> MR
    
    MR -->|route_response| T8
    T8 --> GI
    GI -->|WebSocket Push| C1
    GI -->|WebSocket Push| C2
    GI -->|WebSocket Push| C3
```

## 消息流详细说明

### 1. 入站消息流 (Client → Agent)

```
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│ Client  │────►│ Gateway │────►│  Router │────►│ Handler │────►│  Agent  │
└─────────┘     └─────────┘     └─────────┘     └─────────┘     └─────────┘
    │               │               │               │               │
    │  WebSocket    │  publish()    │  route()      │  handle()     │
    │  JSON         │  topic:       │  to:          │  to:          │  process()
    │               │  gateway:input│  agent:input  │  agent:input  │
    │               │               │               │               │
    ▼               ▼               ▼               ▼               ▼
  "Hello!"      Message        Message         Message         "Response"
```

### 2. 出站消息流 (Agent → Client)

```
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│  Agent  │────►│ Handler │────►│  Router │────►│ Gateway │────►│ Client  │
└─────────┘     └─────────┘     └─────────┘     └─────────┘     └─────────┘
    │               │               │               │               │
    │  publish()    │  publish()    │  route()      │  WebSocket    │
    │  topic:       │  topic:       │  to:          │  Push         │
    │  agent:output │  agent:output │  gateway:     │               │
    │               │               │  output:{id}  │               │
    │               │               │               │               │
    ▼               ▼               ▼               ▼               ▼
  "Response"     Message        Message         Message         "Response"
```

## 核心组件职责

| 组件 | 职责 | 关键实现 |
|------|------|----------|
| **MessageBus** | 消息通道管理 | `DashMap<String, mpsc::Sender>` |
| **MessageRouter** | 消息分发 | 处理器注册 + 路由逻辑 |
| **MessageHandler** | 消息处理 trait | `async fn handle()` |
| **GatewayIntegration** | WebSocket 管理 | 客户端注册/注销 |
| **AgentIntegration** | Agent 调用 | LLM API 集成 |

## 主题命名规范

```
{component}:{direction}[:{identifier}]

Examples:
- gateway:input          # 来自客户端的消息
- gateway:output:abc123  # 发送给特定客户端
- agent:input            # 发送给 Agent
- agent:output           # Agent 输出
- session:xyz789         # 会话特定消息
```