# OpenAI ↔ Anthropic API 转换设计

## 需求分析

### 功能需求
1. **转发原生 OpenAI API**：接收 OpenAI 格式的请求
2. **转换为 Anthropic API**：将请求转换为 Anthropic 格式并转发
3. **响应转换**：将 Anthropic 响应转换回 OpenAI 格式
4. **流式支持**：支持 SSE 流式响应转换

### 使用场景
- 用户有 Anthropic API Key，但应用只支持 OpenAI 格式
- 统一 API 接口，后端自动路由到不同 Provider
- 支持 Claude 模型，但保持 OpenAI 兼容性

## 技术方案

### 1. 数据结构映射

#### OpenAI Request → Anthropic Request
```rust
// OpenAI 格式
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "stream": true,
  "temperature": 0.7,
  "max_tokens": 1000
}

// Anthropic 格式
{
  "model": "claude-3-sonnet-20240229",
  "system": "You are a helpful assistant.",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "stream": true,
  "temperature": 0.7,
  "max_tokens": 1000
}
```

#### Anthropic Response → OpenAI Response
```rust
// Anthropic 格式
{
  "id": "msg_01Xxx...",
  "type": "message",
  "role": "assistant",
  "content": [{"type": "text", "text": "Hello! How can I help you today?"}],
  "model": "claude-3-sonnet-20240229",
  "usage": {"input_tokens": 10, "output_tokens": 20}
}

// OpenAI 格式
{
  "id": "chatcmpl-xxx",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "gpt-4",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! How can I help you today?"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 20,
    "total_tokens": 30
  }
}
```

### 2. 实现方案

#### 方案 A：独立转换层（推荐）
在 bamboo-llm crate 中添加转换模块：

```rust
// bamboo-llm/src/adapters/mod.rs
pub mod openai;
pub mod anthropic;
pub mod converter;

// 转换器 trait
pub trait ApiConverter {
    type Input;
    type Output;
    type StreamChunk;
    
    fn convert_request(&self, input: Self::Input) -> Result<impl Serialize>;
    fn convert_response(&self, output: impl Deserialize) -> Result<Self::Output>;
    fn convert_stream_chunk(&self, chunk: impl Deserialize) -> Result<Self::StreamChunk>;
}
```

#### 方案 B：代理模式
创建 bamboo-proxy crate，作为独立服务：

```rust
// 接收 OpenAI 格式请求
// 转换为 Anthropic 格式
// 转发到 Anthropic API
// 转换响应为 OpenAI 格式
// 返回给客户端
```

### 3. 推荐实现

采用 **方案 A**，在 bamboo-llm 中添加：

```
crates/bamboo-llm/src/
├── adapters/
│   ├── mod.rs           # 适配器模块
│   ├── openai.rs        # OpenAI 类型定义
│   ├── anthropic.rs     # Anthropic 类型定义
│   └── converter.rs     # 转换逻辑
├── providers/
│   ├── mod.rs
│   ├── openai_provider.rs
│   └── anthropic_provider.rs  # 新增 Anthropic Provider
```

### 4. API 端点

```
# 通过 Bamboo 代理访问 Anthropic（OpenAI 格式）
POST /v1/chat/completions
Headers:
  Authorization: Bearer <anthropic-api-key>
  X-Provider: anthropic  # 指定使用 Anthropic

# 或配置文件中指定默认 Provider
```

### 5. 配置支持

```json
// ~/.bamboo/config.json
{
  "llm": {
    "default_provider": "anthropic",
    "providers": {
      "anthropic": {
        "enabled": true,
        "base_url": "https://api.anthropic.com",
        "model": "claude-3-sonnet-20240229",
        "auth": {
          "type": "bearer",
          "token": "sk-ant-..."
        }
      }
    }
  }
}
```

### 6. 前端支持

在 Provider 配置中添加 Anthropic 选项：
- Provider 类型选择（OpenAI / Anthropic / Copilot）
- API Key 输入
- 模型选择（claude-3-opus / claude-3-sonnet / claude-3-haiku）

## 实现步骤

1. **定义 Anthropic API 类型**（anthropic.rs）
2. **实现转换逻辑**（converter.rs）
3. **创建 Anthropic Provider**（anthropic_provider.rs）
4. **更新配置支持**（config.rs）
5. **添加前端配置界面**
6. **测试转换正确性**

## 注意事项

1. **消息格式差异**：
   - OpenAI: `messages` 包含 system/user/assistant/tool
   - Anthropic: `system` 是独立字段，messages 只有 user/assistant

2. **工具调用差异**：
   - OpenAI: `tool_calls` / `tool_call_id`
   - Anthropic: 使用 XML 标签或特定格式

3. **流式响应差异**：
   - OpenAI: `data: {"choices": [{"delta": {"content": "..."}}]}`
   - Anthropic: `data: {"type": "content_block_delta", "delta": {"text": "..."}}`

4. **错误处理**：
   - 统一错误格式
   - 保留原始错误信息
