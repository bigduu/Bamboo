# Bamboo Phase 1: 彻底重构设计方案

## 目标
一次性重构 bamboo-core 和 bamboo-llm，建立统一的内部格式和 Schema Transformer 架构。

## 1. 重构范围

### 修改的 Crates
- `bamboo-core` - 新的 Message/Tool 类型
- `bamboo-llm` - 新的 Provider 架构 + Transformer
- `bamboo-server` - 适配新类型
- `bamboo-cli` - 适配新类型
- `bamboo-tui` - 适配新类型

### 废弃的文件
- `bamboo-core/src/lib.rs` 中的旧 Message 类型
- `bamboo-llm/src/openai.rs` - 重写
- `bamboo-llm/src/providers/*.rs` - 重写

---

## 2. bamboo-core 新设计

```rust
// src/types/mod.rs
pub mod message;
pub mod tool;
pub mod content;

pub use message::{Message, Role, MessageId};
pub use tool::{ToolCall, ToolDefinition, ToolResult};
pub use content::{Content, ContentPart, ImageSource};

// src/types/message.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub role: Role,
    pub content: Content,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub metadata: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self;
    pub fn user(content: impl Into<String>) -> Self;
    pub fn assistant(content: impl Into<String>) -> Self;
    pub fn tool_result(call_id: impl Into<String>, result: ToolResult) -> Self;
    pub fn text(&self) -> Option<&str>;
}

// src/types/content.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    Parts { parts: Vec<ContentPart> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { data: String, mime_type: String },
    Url { url: String },
}

// src/types/tool.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,  // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
}

// src/chat/mod.rs - 新增模块
pub mod request;
pub mod response;
pub mod chunk;

pub use request::{ChatRequest, ChatOptions};
pub use response::{ChatResponse, ChatUsage};
pub use chunk::{ChatChunk, FinishReason};

// src/chat/request.rs
#[derive(Debug, Clone, Default)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub options: ChatOptions,
}

#[derive(Debug, Clone, Default)]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stream: bool,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone)]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { schema: Value },
}

// src/chat/chunk.rs - 流式响应统一格式
#[derive(Debug, Clone)]
pub enum ChatChunk {
    Start { model: String },
    Content { text: String },
    ToolCallStart { call_id: String, name: String },
    ToolCallDelta { call_id: String, arguments_delta: String },
    ToolCallEnd { call_id: String },
    Usage { input_tokens: u32, output_tokens: u32 },
    Finish { reason: FinishReason },
    Error { message: String },
}

#[derive(Debug, Clone, Copy)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Cancelled,
    Error,
}
```

---

## 3. bamboo-llm 新设计

### 目录结构
```
bamboo-llm/src/
├── lib.rs                      # 公共导出
├── error.rs                    # 统一错误类型
│
├── transformer/                # Schema Transformer 模块
│   ├── mod.rs                  # SchemaTransformer trait
│   ├── openai.rs               # OpenAiTransformer
│   ├── error.rs                # ConversionError
│   └── utils.rs                # 通用转换工具
│
├── provider/                   # Provider 模块
│   ├── mod.rs                  # LLMProvider trait
│   ├── config.rs               # ProviderConfig
│   ├── base.rs                 # BaseProvider (通用实现)
│   └── factory.rs              # ProviderFactory
│
└── providers/                  # 具体 Provider 实现
    ├── openai.rs               # OpenAiProvider (新)
    ├── copilot.rs              # CopilotProvider (新)
    └── forward.rs              # ForwardProvider (新)
```

### 核心代码

```rust
// src/error.rs
#[derive(Error, Debug)]
pub enum LLMError {
    #[error("network error: {0}")]
    Network(String),
    
    #[error("api error: {status} - {message}")]
    Api { status: u16, message: String },
    
    #[error("authentication error: {0}")]
    Auth(String),
    
    #[error("transform error: {0}")]
    Transform(#[from] ConversionError),
    
    #[error("stream error: {0}")]
    Stream(String),
    
    #[error("config error: {0}")]
    Config(String),
    
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },
}

// src/transformer/mod.rs
#[async_trait]
pub trait SchemaTransformer: Send + Sync {
    fn provider_id(&self) -> &str;
    
    /// 转换请求为 Provider 特定格式
    fn transform_request(&self, request: &ChatRequest) -> Result<Value, ConversionError>;
    
    /// 解析流式响应块
    fn parse_stream_chunk(&self, data: &str) -> Result<Option<ChatChunk>, ConversionError>;
    
    /// 转换工具定义
    fn transform_tools(&self, tools: &[ToolDefinition]) -> Result<Value, ConversionError>;
}

pub type LLMStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, LLMError>> + Send>>;

// src/provider/mod.rs
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    
    fn metadata(&self) -> &ProviderMetadata;
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;
    
    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream, LLMError>;
    
    async fn validate(&self) -> Result<(), LLMError>;
}

// src/provider/base.rs - 通用 Provider 实现
pub struct BaseProvider<T: SchemaTransformer> {
    config: ProviderConfig,
    http_client: reqwest::Client,
    transformer: Arc<T>,
    metadata: ProviderMetadata,
}

impl<T: SchemaTransformer> BaseProvider<T> {
    pub async fn new(
        config: ProviderConfig,
        transformer: T,
        metadata: ProviderMetadata,
    ) -> Result<Self, LLMError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LLMError::Config(e.to_string()))?;
        
        Ok(Self {
            config,
            http_client,
            transformer: Arc::new(transformer),
            metadata,
        })
    }
    
    pub async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream, LLMError> {
        let body = self.transformer.transform_request(&request)?;
        
        let response = self.send_request(body).await?;
        
        let transformer = self.transformer.clone();
        let stream = response.bytes_stream()
            .map_err(|e| LLMError::Network(e.to_string()))
            .and_then(move |bytes| {
                let text = String::from_utf8_lossy(&bytes);
                futures::future::ready(
                    transformer.parse_stream_chunk(&text)
                        .map_err(LLMError::Transform)
                )
            })
            .filter_map(|result| async move {
                match result {
                    Ok(Some(chunk)) => Some(Ok(chunk)),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                }
            });
        
        Ok(Box::pin(stream))
    }
}

// src/providers/openai.rs
pub struct OpenAiProvider {
    base: BaseProvider<OpenAiTransformer>,
}

impl OpenAiProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self, LLMError> {
        let metadata = ProviderMetadata {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: true,
            },
        };
        
        let base = BaseProvider::new(config, OpenAiTransformer, metadata).await?;
        
        Ok(Self { base })
    }
}

#[async_trait]
impl LLMProvider for OpenAiProvider {
    fn provider_id(&self) -> &str {
        "openai"
    }
    
    fn metadata(&self) -> &ProviderMetadata {
        &self.base.metadata
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        // 基于 stream 实现
        let mut stream = self.chat_stream(request).await?;
        // 收集所有 chunks...
    }
    
    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream, LLMError> {
        self.base.chat_stream(request).await
    }
    
    async fn validate(&self) -> Result<(), LLMError> {
        // 发送测试请求
    }
}

// src/transformer/openai.rs
pub struct OpenAiTransformer;

impl SchemaTransformer for OpenAiTransformer {
    fn provider_id(&self) -> &str {
        "openai"
    }
    
    fn transform_request(&self, request: &ChatRequest) -> Result<Value, ConversionError> {
        let messages: Vec<Value> = request.messages.iter()
            .map(|m| self.convert_message(m))
            .collect::<Result<Vec<_>, _>>()?;
        
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": request.options.stream,
        });
        
        if !request.tools.is_empty() {
            body["tools"] = self.transform_tools(&request.tools)?;
        }
        
        if let Some(temp) = request.options.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(max_tokens) = request.options.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        
        Ok(body)
    }
    
    fn parse_stream_chunk(&self, data: &str) -> Result<Option<ChatChunk>, ConversionError> {
        if data == "[DONE]" {
            return Ok(Some(ChatChunk::Finish { 
                reason: FinishReason::Stop 
            }));
        }
        
        let chunk: Value = serde_json::from_str(data)?;
        
        let choice = chunk["choices"].get(0);
        
        // 处理文本增量
        if let Some(content) = choice.and_then(|c| c["delta"]["content"].as_str()) {
            if !content.is_empty() {
                return Ok(Some(ChatChunk::Content { 
                    text: content.to_string() 
                }));
            }
        }
        
        // 处理工具调用
        if let Some(tool_calls) = choice.and_then(|c| c["delta"]["tool_calls"].as_array()) {
            if let Some(tc) = tool_calls.get(0) {
                return Ok(Some(ChatChunk::ToolCallDelta {
                    call_id: tc["id"].as_str().unwrap_or_default().to_string(),
                    arguments_delta: tc["function"]["arguments"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                }));
            }
        }
        
        // 处理完成
        if choice.and_then(|c| c["finish_reason"].as_str()).is_some() {
            return Ok(Some(ChatChunk::Finish { 
                reason: FinishReason::Stop 
            }));
        }
        
        Ok(None)
    }
    
    fn transform_tools(&self, tools: &[ToolDefinition]) -> Result<Value, ConversionError> {
        let tools_json: Vec<Value> = tools.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })
        }).collect();
        
        Ok(json!(tools_json))
    }
}

impl OpenAiTransformer {
    fn convert_message(&self, msg: &Message) -> Result<Value, ConversionError> {
        let mut json = json!({
            "role": msg.role.to_string().to_lowercase(),
        });
        
        match &msg.content {
            Content::Text { text } => {
                json["content"] = json!(text);
            }
            Content::Parts { parts } => {
                // 多模态内容转换
                json["content"] = json!(parts.iter()
                    .map(|p| self.convert_content_part(p))
                    .collect::<Result<Vec<_>, _>>()?);
            }
        }
        
        if let Some(tool_calls) = &msg.tool_calls {
            json["tool_calls"] = json!(tool_calls.iter()
                .map(|tc| json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments.to_string(),
                    }
                }))
                .collect::<Vec<_>>());
        }
        
        if let Some(tool_call_id) = &msg.tool_call_id {
            json["tool_call_id"] = json!(tool_call_id);
        }
        
        Ok(json)
    }
}
```

---

## 4. 迁移步骤

### 步骤 1: 重构 bamboo-core
1. 创建新的 types/ 目录
2. 实现 Message, Content, Tool 新类型
3. 创建 chat/ 模块 (Request, Response, Chunk)
4. 保留旧类型作为 deprecated 别名（临时兼容）
5. 更新 lib.rs 导出

### 步骤 2: 重构 bamboo-llm
1. 创建 transformer/ 模块
2. 实现 OpenAiTransformer
3. 创建 provider/ 模块
4. 实现 BaseProvider
5. 重写 OpenAiProvider
6. 重写 CopilotProvider（复用 OpenAiTransformer）
7. 重写 ForwardProvider

### 步骤 3: 更新依赖
1. bamboo-server - 更新使用新类型
2. bamboo-cli - 更新使用新类型
3. bamboo-tui - 更新使用新类型

### 步骤 4: 清理
1. 移除旧类型别名
2. 移除废弃代码
3. 运行 cargo check 确保编译通过

---

## 5. 验证清单

- [ ] bamboo-core 编译通过
- [ ] bamboo-llm 编译通过
- [ ] bamboo-server 编译通过
- [ ] bamboo-cli 编译通过
- [ ] bamboo-tui 编译通过
- [ ] 示例运行正常
- [ ] OpenAI Provider 测试通过
- [ ] Copilot Provider 测试通过

---

交给 Codex 实现此重构方案。