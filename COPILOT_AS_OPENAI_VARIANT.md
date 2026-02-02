# Copilot 作为 OpenAI Provider 变种设计方案

## 核心思想
Copilot 是 OpenAI API 的一个变种，差异只在：
1. **认证方式** - Device Code OAuth vs API Key
2. **Headers** - 需要自定义 headers (editor-version, editor-plugin-version)
3. **Base URL** - api.githubcopilot.com vs api.openai.com

其他（请求格式、响应格式、流式处理）完全相同。

## 设计方案

### 1. 扩展 ProviderConfig 支持 Copilot

```rust
// bamboo-llm/src/provider/config.rs
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth: AuthConfig,
    pub headers: Option<HashMap<String, String>>,  // 自定义 headers
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub enum AuthConfig {
    ApiKey { key: String },
    Bearer { token: String },
    DeviceCode {  // Copilot 专用
        client_id: String,
        device_code_url: String,
        access_token_url: String,
    },
}
```

### 2. 扩展 Authenticator 支持 Device Code

```rust
// bamboo-llm/src/auth/mod.rs
#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn get_auth_header(&self) -> Result<Option<(String, String)>, AuthError>;
    async fn needs_refresh(&self) -> bool;
    async fn refresh(&self) -> Result<(), AuthError>;
}

// API Key 认证（OpenAI 标准）
pub struct ApiKeyAuth {
    key: String,
}

// Device Code 认证（Copilot）
pub struct DeviceCodeAuth {
    client_id: String,
    token_cache: TokenCache,
    device_code_flow: DeviceCodeFlow,
}

impl DeviceCodeAuth {
    pub async fn new(client_id: String) -> Result<Self, AuthError> {
        // 尝试从缓存加载 token
        let token_cache = TokenCache::load().await?;
        
        if token_cache.is_valid() {
            Ok(Self { client_id, token_cache, ... })
        } else {
            // 启动 Device Code Flow
            self.authenticate().await
        }
    }
    
    async fn authenticate(&mut self) -> Result<(), AuthError> {
        // 1. 请求 device code
        let device_code = self.request_device_code().await?;
        
        // 2. 显示给用户
        println!("请访问: {}", device_code.verification_uri);
        println!("输入 code: {}", device_code.user_code);
        
        // 3. 轮询 token
        let token = self.poll_for_token(&device_code).await?;
        
        // 4. 保存到缓存
        self.token_cache.save(token).await?;
    }
}

#[async_trait]
impl Authenticator for DeviceCodeAuth {
    async fn get_auth_header(&self) -> Result<Option<(String, String)>, AuthError> {
        let token = self.token_cache.get_token()?;
        Ok(Some(("Authorization".to_string(), format!("Bearer {}", token))))
    }
    
    async fn needs_refresh(&self) -> bool {
        self.token_cache.is_expired() || self.token_cache.is_near_expiry()
    }
    
    async fn refresh(&self) -> Result<(), AuthError> {
        // Copilot 不支持 refresh token，需要重新认证
        self.authenticate().await
    }
}
```

### 3. OpenAiProvider 支持 Copilot

```rust
// bamboo-llm/src/providers/openai.rs
pub struct OpenAiProvider {
    config: ProviderConfig,
    http_client: reqwest::Client,
    authenticator: Arc<dyn Authenticator>,
    transformer: Arc<OpenAiTransformer>,
    custom_headers: HashMap<String, String>,
}

impl OpenAiProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self, LLMError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LLMError::Config(e.to_string()))?;
        
        // 根据 auth 配置创建对应的 Authenticator
        let authenticator: Arc<dyn Authenticator> = match &config.auth {
            AuthConfig::ApiKey { key } => Arc::new(ApiKeyAuth::new(key.clone())),
            AuthConfig::Bearer { token } => Arc::new(ApiKeyAuth::new(token.clone())),
            AuthConfig::DeviceCode { client_id, .. } => {
                Arc::new(DeviceCodeAuth::new(client_id.clone()).await?)
            }
        };
        
        Ok(Self {
            config,
            http_client,
            authenticator,
            transformer: Arc::new(OpenAiTransformer),
            custom_headers: config.headers.clone().unwrap_or_default(),
        })
    }
    
    async fn send_request(&self, body: Value) -> Result<reqwest::Response, LLMError> {
        let mut req = self.http_client
            .post(format!("{}/chat/completions", self.config.base_url))
            .json(&body);
        
        // 添加认证头
        if let Some((header, value)) = self.authenticator.get_auth_header().await? {
            req = req.header(header, value);
        }
        
        // 添加自定义 headers（Copilot 需要）
        for (key, value) in &self.custom_headers {
            req = req.header(key, value);
        }
        
        req.send().await.map_err(|e| LLMError::Network(e.to_string()))
    }
}

#[async_trait]
impl LLMProvider for OpenAiProvider {
    fn provider_id(&self) -> &str {
        &self.config.id  // "openai" 或 "copilot"
    }
    
    async fn chat_stream(&self, request: ChatRequest) -> Result<LLMStream, LLMError> {
        // 检查认证是否需要刷新
        if self.authenticator.needs_refresh().await {
            self.authenticator.refresh().await?;
        }
        
        let body = self.transformer.transform_request(&request)?;
        let response = self.send_request(body).await?;
        
        // 解析流式响应...
    }
}
```

### 4. 配置示例

```rust
// 标准 OpenAI
let openai_config = ProviderConfig {
    id: "openai".to_string(),
    name: "OpenAI".to_string(),
    base_url: "https://api.openai.com".to_string(),
    auth: AuthConfig::ApiKey { 
        key: std::env::var("OPENAI_API_KEY").unwrap() 
    },
    headers: None,
    timeout_seconds: 60,
};

// Copilot（OpenAI 变种）
let copilot_config = ProviderConfig {
    id: "copilot".to_string(),
    name: "GitHub Copilot".to_string(),
    base_url: "https://api.githubcopilot.com".to_string(),
    auth: AuthConfig::DeviceCode {
        client_id: "Iv23lixxx...".to_string(),
        device_code_url: "https://github.com/login/device/code".to_string(),
        access_token_url: "https://github.com/login/oauth/access_token".to_string(),
    },
    headers: Some(HashMap::from([
        ("editor-version".to_string(), "vscode/1.99.2".to_string()),
        ("editor-plugin-version".to_string(), "copilot-chat/0.20.3".to_string()),
        ("user-agent".to_string(), "GitHubCopilotChat/0.20.3".to_string()),
    ])),
    timeout_seconds: 60,
};

// 创建 Provider
let openai = OpenAiProvider::new(openai_config).await?;
let copilot = OpenAiProvider::new(copilot_config).await?;

// 使用方式完全相同
let response = copilot.chat_stream(request).await?;
```

### 5. 配置文件示例 (bamboo.toml)

```toml
[[providers]]
id = "openai"
name = "OpenAI"
base_url = "https://api.openai.com"
auth = { type = "api_key", env = "OPENAI_API_KEY" }

[[providers]]
id = "copilot"
name = "GitHub Copilot"
base_url = "https://api.githubcopilot.com"
auth = { 
    type = "device_code",
    client_id = "Iv23lixxx...",
    device_code_url = "https://github.com/login/device/code",
    access_token_url = "https://github.com/login/oauth/access_token"
}
headers = [
    { key = "editor-version", value = "vscode/1.99.2" },
    { key = "editor-plugin-version", value = "copilot-chat/0.20.3" },
]

[[providers]]
id = "moonshot"
name = "Moonshot"
base_url = "https://api.moonshot.cn"
auth = { type = "api_key", env = "MOONSHOT_API_KEY" }
# Moonshot 也是 OpenAI-compatible，复用同一个 Provider
```

### 6. 文件结构

```
bamboo-llm/src/
├── auth/
│   ├── mod.rs              # Authenticator trait
│   ├── api_key.rs          # ApiKeyAuth
│   └── device_code.rs      # DeviceCodeAuth (Copilot)
├── provider/
│   ├── config.rs           # ProviderConfig, AuthConfig
│   └── base.rs             # BaseProvider (通用 HTTP 逻辑)
├── transformer/
│   └── openai.rs           # OpenAiTransformer
└── providers/
    └── openai.rs           # OpenAiProvider (支持所有 OpenAI-compatible API)
```

## 优势

1. **代码复用** - Copilot 和 OpenAI 共享 90% 代码
2. **易于扩展** - 添加 Moonshot、DeepSeek 等只需配置
3. **统一接口** - 所有 Provider 使用相同的 LLMProvider trait
4. **配置驱动** - 无需重新编译即可添加新 Provider

## 实现任务

1. 创建 `auth/device_code.rs` - Device Code OAuth 实现
2. 修改 `auth/mod.rs` - 统一 Authenticator trait
3. 修改 `provider/config.rs` - 支持 AuthConfig 变体
4. 优化 `providers/openai.rs` - 支持自定义 headers 和 Device Code
5. 删除独立的 `providers/copilot.rs` - 不再 needed
6. 更新 `lib.rs` 导出

交给 Codex 实现。