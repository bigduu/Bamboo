use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use bamboo_config::{ConfigManager, AuthSettings};
use bamboo_core::{Session, AgentEvent, storage::JsonlStorage};
use bamboo_core::tools::{ToolExecutor, ToolCall, ToolResult, ToolSchema, FunctionSchema};
use bamboo_core::tools::executor::{ToolError, Result as ToolResultResult};
use bamboo_llm::providers::OpenAiProvider;
use bamboo_llm::provider::{ProviderConfig, AuthConfig};
use bamboo_skill::SkillManager;
use bamboo_tool::types::{ToolDef, ArgType};
use bamboo_tool::executor::{ToolRunner, ToolExecutor as BambooToolExecutor};
use async_trait::async_trait;

use crate::event_bus::{EventBus, Event, ReplyChannel, ChatChunk};
use crate::websocket::{Gateway, GatewayConfig};

/// Gateway 集成类型别名
pub type GatewayRef = Arc<Gateway>;

/// ToolExecutor implementation that uses SkillManager to get tools
pub struct SkillManagerToolExecutor {
    skill_manager: Arc<SkillManager>,
    executor: BambooToolExecutor,
}

impl SkillManagerToolExecutor {
    pub fn new(skill_manager: Arc<SkillManager>) -> Self {
        Self {
            skill_manager,
            executor: BambooToolExecutor::new(),
        }
    }
}

#[async_trait]
impl ToolExecutor for SkillManagerToolExecutor {
    async fn execute(&self, call: &ToolCall) -> ToolResultResult<ToolResult> {
        // Get tool definition from skill manager
        let tool_def = self.skill_manager.get_tool(&call.function.name)
            .ok_or_else(|| ToolError::NotFound(format!("Tool not found: {}", call.function.name)))?;
        
        // Parse arguments from JSON string
        let args: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&call.function.arguments)
                .unwrap_or_else(|_| HashMap::new());
        
        // Execute using bamboo_tool's executor
        match self.executor.execute(&tool_def, args).await {
            Ok(result) => Ok(ToolResult {
                success: result.success,
                result: if result.success { result.output } else { result.error.unwrap_or_default() },
                display_preference: None,
            }),
            Err(e) => Err(ToolError::Execution(e.to_string())),
        }
    }
    
    fn list_tools(&self) -> Vec<ToolSchema> {
        self.skill_manager.get_all_tools()
            .into_iter()
            .map(convert_tool_def_to_schema)
            .collect()
    }
}

/// Convert bamboo_tool::ToolDef to bamboo_core::tools::ToolSchema
fn convert_tool_def_to_schema(tool_def: ToolDef) -> ToolSchema {
    // Convert args to JSON schema parameters
    let properties: serde_json::Map<String, serde_json::Value> = tool_def
        .args
        .iter()
        .map(|arg| {
            let prop = serde_json::json!({
                "type": arg_type_to_string(&arg.arg_type),
                "description": arg.description.as_deref().unwrap_or("")
            });
            (arg.name.clone(), prop)
        })
        .collect();
    
    let required: Vec<String> = tool_def
        .args
        .iter()
        .filter(|arg| arg.required)
        .map(|arg| arg.name.clone())
        .collect();
    
    ToolSchema {
        schema_type: "function".to_string(),
        function: FunctionSchema {
            name: tool_def.name,
            description: tool_def.description.unwrap_or_default(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            }),
        },
    }
}

fn arg_type_to_string(arg_type: &ArgType) -> String {
    match arg_type {
        ArgType::String => "string",
        ArgType::Number => "number",
        ArgType::Boolean => "boolean",
        ArgType::Array => "array",
        ArgType::Object => "object",
    }.to_string()
}

pub struct AppState {
    pub sessions: Arc<RwLock<HashMap<String, Session>>>,
    pub storage: JsonlStorage,
    pub llm: Arc<dyn bamboo_llm::LLMProvider>,
    pub skill_manager: Arc<SkillManager>,
    pub cancel_tokens: Arc<RwLock<HashMap<String, tokio_util::sync::CancellationToken>>>,
    pub config: Arc<ConfigManager>,
    // 新增: Gateway 和 EventBus
    pub gateway: Option<GatewayRef>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub async fn new() -> Self {
        let config_manager = ConfigManager::load_default().await
            .expect("Failed to load config");
        
        Self::new_with_config(
            config_manager,
            "copilot",
            "https://api.githubcopilot.com".to_string(),
            "copilot-chat".to_string(),
            "".to_string(),
        ).await
    }

    pub async fn new_with_config(
        config_manager: ConfigManager,
        _provider: &str,
        _llm_base_url: String,
        _model: String,
        _api_key: String,
    ) -> Self {
        // 从配置获取存储路径
        let config = config_manager.get().read().await.clone();
        let storage_path = config.storage.path.as_ref()
            .map(|p| bamboo_config::expand_tilde(p).unwrap_or_else(|| std::path::PathBuf::from(p)))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::env::temp_dir())
                    .join(".bamboo")
            });
        
        let storage = JsonlStorage::new(&storage_path);
        storage.init().await.expect("Failed to init storage");
        
        // 根据配置创建 LLM Provider
        let llm: Arc<dyn bamboo_llm::LLMProvider> = match create_llm_provider_from_config(&config).await {
            Ok(provider) => provider,
            Err(e) => {
                log::error!("Failed to create LLM provider from config: {}. Falling back to default Copilot provider.", e);
                // 向后兼容：如果配置创建失败，使用默认 Copilot provider
                Arc::new(
                    OpenAiProvider::copilot().await
                        .expect("Failed to create default Copilot provider")
                )
            }
        };

        // 从配置获取 skills 目录
        let skills_dirs: Vec<std::path::PathBuf> = config.skills.directories
            .iter()
            .map(|d| bamboo_config::expand_tilde(d).unwrap_or_else(|| std::path::PathBuf::from(d)))
            .collect();
        
        let skills_dir = skills_dirs.first()
            .cloned()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::env::temp_dir())
                    .join(".bamboo")
                    .join("skills")
            });
        
        // 创建 skills 目录（如果不存在）
        if let Err(e) = tokio::fs::create_dir_all(&skills_dir).await {
            log::warn!("Failed to create skills directory: {}", e);
        } else {
            // 创建示例 skill（如果不存在）
            let example_skill_dir = skills_dir.join("example");
            if !example_skill_dir.exists() {
                if let Err(e) = create_example_skill(&example_skill_dir).await {
                    log::warn!("Failed to create example skill: {}", e);
                }
            }
        }
        
        let skill_manager = Arc::new(SkillManager::new(&skills_dir));
        
        // 初始化 Skill Manager（加载所有 skills）
        if config.skills.enabled {
            if let Err(e) = skill_manager.initialize().await {
                log::error!("Failed to initialize skill manager: {}", e);
            } else {
                log::info!("Skill manager initialized with {} skills", skill_manager.skill_count());
            }
            
            // 启动 skill 监听任务（后台处理热重载）
            if config.skills.auto_reload {
                let manager_clone = skill_manager.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        if let Err(e) = manager_clone.process_events().await {
                            log::error!("Skill watcher error: {}", e);
                        }
                    }
                });
            }
        }

        // 初始化 EventBus
        let event_bus = Arc::new(EventBus::new(1000));

        // 初始化 Gateway（如果启用）
        let gateway = if config.gateway.enabled {
            let gateway_config = GatewayConfig {
                bind: config.gateway.bind.clone(),
                auth_token: config.gateway.auth_token.clone(),
                max_connections: config.gateway.max_connections,
                heartbeat_interval_secs: config.gateway.heartbeat_interval_secs,
            };
            Some(Arc::new(Gateway::new(gateway_config)))
        } else {
            log::info!("Gateway is disabled");
            None
        };
        
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage,
            llm,
            skill_manager,
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config_manager),
            gateway,
            event_bus,
        }
    }

    /// 启动 Gateway（如果启用）
    pub async fn start_gateway(self: Arc<Self>) {
        if let Some(ref _gateway) = self.gateway {
            // 启动 Gateway 消息转发任务
            let state_clone = self.clone();
            tokio::spawn(async move {
                state_clone.gateway_message_forwarder().await;
            });
        }
    }

    /// Gateway 消息转发器 - 将 Gateway 消息转发到 EventBus
    async fn gateway_message_forwarder(&self) {
        // 这个任务会定期检查 Gateway 的消息
        // 实际实现需要在 Gateway 中添加消息处理器
        log::debug!("Gateway message forwarder started");
        
        // 这里是一个占位符实现
        // 实际实现需要 Gateway 提供消息订阅机制
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    /// 处理来自 Gateway 的消息
    pub async fn handle_gateway_message(&self, session_id: String, content: String) {
        log::debug!("Handling gateway message for session {}: {}", session_id, content);
        
        // 发布 ChatRequest 事件到 EventBus
        let reply_to = ReplyChannel::Gateway(session_id.clone());
        let event = Event::ChatRequest {
            session_id,
            content,
            reply_to,
        };
        
        if let Err(e) = self.event_bus.publish(event) {
            log::error!("Failed to publish gateway message to event bus: {}", e);
        }
    }

    /// 发送事件到客户端（HTTP SSE 或 WebSocket）
    pub async fn send_event(&self, _session_id: &str, event: Event) {
        match &event {
            Event::ChatResponse { session_id: sid, .. } |
            Event::AgentComplete { session_id: sid, .. } |
            Event::AgentError { session_id: sid, .. } |
            Event::ToolStart { session_id: sid, .. } |
            Event::ToolComplete { session_id: sid, .. } |
            Event::ToolError { session_id: sid, .. } => {
                // 尝试通过 Gateway 发送
                if let Some(ref gateway) = self.gateway {
                    // 将 Event 转换为 GatewayEvent
                    if let Ok(gateway_event) = convert_event_to_gateway_event(&event) {
                        if let Err(e) = gateway.send_to(sid, gateway_event).await {
                            log::debug!("Failed to send via gateway (session {} may be HTTP): {}", sid, e);
                            // 发送到 HTTP EventBus
                            let http_event = Event::HttpResponse {
                                session_id: sid.to_string(),
                                event: Box::new(event),
                            };
                            if let Err(e) = self.event_bus.publish(http_event) {
                                log::error!("Failed to publish HTTP response: {}", e);
                            }
                        }
                    }
                } else {
                    // Gateway 未启用，发送到 HTTP EventBus
                    let http_event = Event::HttpResponse {
                        session_id: sid.to_string(),
                        event: Box::new(event),
                    };
                    if let Err(e) = self.event_bus.publish(http_event) {
                        log::error!("Failed to publish HTTP response: {}", e);
                    }
                }
            }
            _ => {}
        }
    }

    pub async fn save_event(&self, session_id: &str, event: &AgentEvent) {
        let _ = self.storage.append_event(session_id, event).await;
    }

    pub async fn save_session(&self, session: &Session) {
        let _ = self.storage.save_session(session).await;
    }
    
    /// Get all tool schemas from skill manager
    pub fn get_all_tool_schemas(&self) -> Vec<ToolSchema> {
        self.skill_manager.get_all_tools()
            .into_iter()
            .map(convert_tool_def_to_schema)
            .collect()
    }
    
    /// Build system prompt with skills context
    pub fn build_system_prompt(&self, base_prompt: &str) -> String {
        let skills = self.skill_manager.list_skills();
        
        let mut prompt = base_prompt.to_string();
        
        // 尝试获取配置（非阻塞）
        if let Ok(config) = self.config.get().try_read() {
            // 如果配置了 agent system_prompt，使用它
            if !config.agent.system_prompt.is_empty() {
                prompt = config.agent.system_prompt.clone();
            }
        }
        
        if skills.is_empty() {
            return prompt;
        }
        
        prompt.push_str("\n\nYou have access to the following specialized skills:\n\n");
        
        for skill in skills {
            prompt.push_str(&format!(
                "## {}\n{}\n\n",
                skill.name,
                skill.description
            ));
            
            if let Some(ref sp) = skill.system_prompt {
                if !sp.is_empty() {
                    prompt.push_str(&format!("Instructions: {}\n\n", sp));
                }
            }
            
            if !skill.tools.is_empty() {
                let tool_names: Vec<String> = skill.tools.iter()
                    .map(|t| t.name.clone())
                    .collect();
                prompt.push_str(&format!("Available tools: {}\n", tool_names.join(", ")));
            }
            
            prompt.push('\n');
        }
        
        prompt
    }
    
    /// Create a ToolExecutor for use with agent_runner
    pub fn create_tool_executor(&self) -> Arc<dyn ToolExecutor> {
        Arc::new(SkillManagerToolExecutor::new(self.skill_manager.clone()))
    }
}

/// 将 Event 转换为 GatewayEvent
fn convert_event_to_gateway_event(event: &Event) -> Result<crate::websocket::GatewayEvent, String> {
    use crate::websocket::{GatewayEvent, TokenUsage};
    
    match event {
        Event::ChatResponse { session_id, chunk } => {
            let token = match chunk {
                ChatChunk::Content { text } => text.clone(),
                _ => String::new(),
            };
            Ok(GatewayEvent::AgentToken {
                session_id: session_id.clone(),
                token,
            })
        }
        Event::AgentComplete { session_id, usage } => {
            Ok(GatewayEvent::AgentComplete {
                session_id: session_id.clone(),
                usage: TokenUsage {
                    prompt_tokens: usage.prompt_tokens,
                    completion_tokens: usage.completion_tokens,
                    total_tokens: usage.total_tokens,
                },
            })
        }
        Event::AgentError { message, .. } => {
            Ok(GatewayEvent::Error {
                code: "AGENT_ERROR".to_string(),
                message: message.clone(),
            })
        }
        Event::ToolStart { session_id, tool_name, .. } => {
            Ok(GatewayEvent::AgentToolStart {
                session_id: session_id.clone(),
                tool: tool_name.clone(),
            })
        }
        Event::ToolComplete { session_id, tool_call_id, result } => {
            Ok(GatewayEvent::AgentToolComplete {
                session_id: session_id.clone(),
                tool: tool_call_id.clone(),
                result: result.result.clone(),
            })
        }
        Event::ToolError { tool_call_id, error, .. } => {
            Ok(GatewayEvent::Error {
                code: "TOOL_ERROR".to_string(),
                message: format!("Tool {} error: {}", tool_call_id, error),
            })
        }
        _ => Err("Event type not supported for Gateway".to_string()),
    }
}

/// Create an example skill for demonstration
async fn create_example_skill(dir: &std::path::Path) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    
    let skill_md = r#"---
name: example
version: 1.0.0
description: An example skill demonstrating the skill system
tools:
  - name: hello
    description: Say hello to someone
    command: tools/hello.sh
    args:
      - name: name
        type: string
        required: true
        description: Name of the person to greet
---

# Example Skill

This is an example skill that demonstrates how to create skills for Bamboo.

## Usage

Use this skill when you want to greet someone by name.
"#;
    
    tokio::fs::write(dir.join("SKILL.md"), skill_md).await?;
    
    // Create tools directory and hello.sh script
    let tools_dir = dir.join("tools");
    tokio::fs::create_dir_all(&tools_dir).await?;
    
    let hello_script = r#"#!/bin/bash
# Hello tool - greets someone by name
NAME="${ARG_NAME:-World}"
echo "Hello, $NAME! Welcome to Bamboo skills."
"#;
    
    let script_path = tools_dir.join("hello.sh");
    tokio::fs::write(&script_path, hello_script).await?;
    
    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&script_path).await?.permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&script_path, perms).await?;
    }
    
    log::info!("Created example skill at {:?}", dir);
    Ok(())
}

/// 启动 SSE 流发送器
pub fn spawn_sse_sender(
    mut rx: mpsc::Receiver<AgentEvent>,
    tx: mpsc::Sender<actix_web::web::Bytes>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let event_json = match serde_json::to_string(&event) {
                Ok(json) => json,
                Err(_) => continue,
            };
            
            let sse_data = format!("data: {}\n\n", event_json);
            let bytes = actix_web::web::Bytes::from(sse_data);
            
            if tx.send(bytes).await.is_err() {
                break;
            }
            
            // 如果是 Complete 或 Error 事件，结束流
            match &event {
                AgentEvent::Complete { .. } | AgentEvent::Error { .. } => {
                    break;
                }
                _ => {}
            }
        }
    })
}

// 为 AppState 实现 Clone（用于线程间传递）
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            storage: self.storage.clone(),
            llm: self.llm.clone(),
            skill_manager: self.skill_manager.clone(),
            cancel_tokens: self.cancel_tokens.clone(),
            config: self.config.clone(),
            gateway: self.gateway.clone(),
            event_bus: self.event_bus.clone(),
        }
    }
}

/// 根据配置创建 LLM Provider
/// 
/// 支持以下 provider:
/// - "copilot": 使用 Device Code 认证的 GitHub Copilot
/// - "openai": 使用 API Key 的 OpenAI 或兼容服务
async fn create_llm_provider_from_config(
    config: &bamboo_config::Config,
) -> anyhow::Result<Arc<dyn bamboo_llm::LLMProvider>> {
    let provider_name = &config.llm.default_provider;
    log::info!("Creating LLM provider from config: {}", provider_name);
    
    // 查找 provider 配置
    let provider_settings = config.llm.providers.get(provider_name)
        .ok_or_else(|| anyhow::anyhow!(
            "Provider '{}' not found in config. Available providers: {:?}",
            provider_name,
            config.llm.providers.keys().collect::<Vec<_>>()
        ))?;
    
    if !provider_settings.enabled {
        return Err(anyhow::anyhow!(
            "Provider '{}' is disabled in config",
            provider_name
        ));
    }
    
    // 将 bamboo_config::AuthSettings 转换为 bamboo_llm::provider::AuthConfig
    let auth_config = match &provider_settings.auth {
        AuthSettings::ApiKey { env } => {
            let api_key = std::env::var(env)
                .map_err(|_| anyhow::anyhow!(
                    "API key environment variable '{}' not set for provider '{}'",
                    env, provider_name
                ))?;
            AuthConfig::ApiKey { key: api_key }
        }
        AuthSettings::Bearer { env } => {
            let token = std::env::var(env)
                .map_err(|_| anyhow::anyhow!(
                    "Bearer token environment variable '{}' not set for provider '{}'",
                    env, provider_name
                ))?;
            AuthConfig::Bearer { token }
        }
        AuthSettings::DeviceCode { client_id, device_code_url, access_token_url, copilot_token_url } => {
            AuthConfig::DeviceCode {
                client_id: client_id.clone(),
                device_code_url: device_code_url.clone().unwrap_or_else(|| 
                    "https://github.com/login/device/code".to_string()
                ),
                access_token_url: access_token_url.clone().unwrap_or_else(|| 
                    "https://github.com/login/oauth/access_token".to_string()
                ),
                copilot_token_url: copilot_token_url.clone().unwrap_or_else(|| 
                    "https://api.github.com/copilot_internal/v2/token".to_string()
                ),
            }
        }
        AuthSettings::None => AuthConfig::None,
    };
    
    // 构建 ProviderConfig
    let model = provider_settings.model.clone()
        .unwrap_or_else(|| "gpt-4o-mini".to_string());
    
    let timeout = std::time::Duration::from_secs(
        provider_settings.timeout_seconds.unwrap_or(60)
    );
    
    let headers = provider_settings.headers.clone().unwrap_or_default();
    
    let provider_config = ProviderConfig {
        provider_id: provider_name.clone(),
        base_url: provider_settings.base_url.clone(),
        auth: auth_config,
        model,
        timeout,
        headers,
    };
    
    log::info!(
        "Initializing provider '{}' with base URL: {}, model: {}",
        provider_name,
        provider_config.base_url,
        provider_config.model
    );
    
    // 创建 provider
    match provider_name.as_str() {
        "copilot" => {
            log::info!("Creating GitHub Copilot provider with Device Code authentication");
            let provider = OpenAiProvider::with_config(provider_config).await
                .map_err(|e| anyhow::anyhow!(
                    "Failed to create Copilot provider: {}",
                    e
                ))?;
            Ok(Arc::new(provider))
        }
        "openai" | _ => {
            log::info!("Creating OpenAI-compatible provider");
            let provider = OpenAiProvider::with_config(provider_config).await
                .map_err(|e| anyhow::anyhow!(
                    "Failed to create OpenAI provider: {}",
                    e
                ))?;
            Ok(Arc::new(provider))
        }
    }
}
