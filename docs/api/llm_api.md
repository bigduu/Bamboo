# Bamboo LLM API 文档

Bamboo 提供 OpenAI 兼容的 LLM API 端点，允许其他服务使用 Bamboo 作为 LLM 后端。

## API 端点列表

| 端点 | 方法 | 描述 |
|------|------|------|
| `/v1/chat/completions` | POST | 聊天补全（支持流式和非流式） |
| `/v1/models` | GET | 获取可用模型列表 |
| `/v1/models/{model}` | GET | 获取模型信息 |

## 认证

API 使用 Bearer Token 认证。需要在请求头中提供 `Authorization` 头：

```
Authorization: Bearer YOUR_API_KEY
```

API Key 通过 Bamboo 配置文件中的 `server.admin_token` 设置。如果未设置，则允许所有请求（开发模式）。

## 端点详情

### POST /v1/chat/completions

创建聊天补全请求。

#### 请求体

```json
{
  "model": "copilot/copilot-chat",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false
}
```

#### 参数说明

| 参数 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `model` | string | 是 | 模型 ID |
| `messages` | array | 是 | 消息列表 |
| `messages[].role` | string | 是 | 角色：`system`, `user`, `assistant`, `tool` |
| `messages[].content` | string | 是 | 消息内容 |
| `temperature` | float | 否 | 采样温度 (0-2)，默认 1 |
| `max_tokens` | integer | 否 | 最大生成 token 数 |
| `top_p` | float | 否 | 核采样概率 (0-1) |
| `stream` | boolean | 否 | 是否使用流式响应，默认 false |
| `tools` | array | 否 | 可用工具列表 |

#### 非流式响应

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "copilot/copilot-chat",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 10,
    "total_tokens": 30
  }
}
```

#### 流式响应

设置 `stream: true` 时，返回 SSE (Server-Sent Events) 流：

```
data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"copilot/copilot-chat","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"copilot/copilot-chat","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"copilot/copilot-chat","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1677652288,"model":"copilot/copilot-chat","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

### GET /v1/models

获取可用模型列表。

#### 响应

```json
{
  "object": "list",
  "data": [
    {
      "id": "copilot/copilot-chat",
      "object": "model",
      "created": 1677652288,
      "owned_by": "copilot"
    },
    {
      "id": "openai/gpt-4o-mini",
      "object": "model",
      "created": 1677652288,
      "owned_by": "openai"
    }
  ]
}
```

### GET /v1/models/{model}

获取特定模型的信息。

#### 响应

```json
{
  "id": "copilot/copilot-chat",
  "object": "model",
  "created": 1677652288,
  "owned_by": "copilot"
}
```

## 使用示例

### curl 示例

#### 非流式请求

```bash
curl -X POST http://localhost:8081/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "copilot/copilot-chat",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "temperature": 0.7
  }'
```

#### 流式请求

```bash
curl -X POST http://localhost:8081/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "copilot/copilot-chat",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": true
  }'
```

#### 获取模型列表

```bash
curl -X GET http://localhost:8081/v1/models \
  -H "Authorization: Bearer YOUR_API_KEY"
```

### Python 示例

```python
import openai

# 配置 Bamboo API
client = openai.OpenAI(
    api_key="YOUR_API_KEY",
    base_url="http://localhost:8081/v1"
)

# 非流式请求
response = client.chat.completions.create(
    model="copilot/copilot-chat",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"}
    ],
    temperature=0.7
)
print(response.choices[0].message.content)

# 流式请求
stream = client.chat.completions.create(
    model="copilot/copilot-chat",
    messages=[{"role": "user", "content": "Hello!"}],
    stream=True
)
for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

## 错误处理

API 返回标准的 HTTP 状态码和 OpenAI 兼容的错误格式：

```json
{
  "error": {
    "message": "Invalid authentication token",
    "type": "authentication_error",
    "code": "invalid_api_key"
  }
}
```

### 错误代码

| 状态码 | 错误类型 | 描述 |
|--------|----------|------|
| 401 | `authentication_error` | API Key 无效或缺失 |
| 404 | `invalid_request_error` | 模型不存在 |
| 500 | `api_error` | 内部服务器错误 |

## 测试

### 启动 Bamboo Server

```bash
cd ~/workspace/bamboo
cargo run -p bamboo-server
```

### 运行测试

```bash
# 测试模型列表
curl http://localhost:8081/v1/models

# 测试聊天补全
curl -X POST http://localhost:8081/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "copilot/copilot-chat",
    "messages": [{"role": "user", "content": "Hi"}]
  }'
```

## 配置

在 `~/.bamboo/config.json` 中配置 API 认证：

```json
{
  "server": {
    "port": 8081,
    "host": "127.0.0.1",
    "admin_token": "YOUR_SECURE_API_KEY"
  }
}
```

## 注意事项

1. **认证**：生产环境务必设置 `admin_token` 以启用认证
2. **模型映射**：Bamboo 使用 `provider/model` 格式的模型 ID
3. **流式响应**：流式响应使用 SSE 格式，需要正确处理 `data: [DONE]` 结束标记
4. **工具调用**：支持 OpenAI 格式的工具/函数调用
