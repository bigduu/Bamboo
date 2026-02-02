use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 主配置结构体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: String,
    pub server: ServerConfig,
    pub gateway: GatewayConfig,
    pub llm: LlmConfig,
    pub skills: SkillsConfig,
    pub agent: AgentConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            server: ServerConfig::default(),
            gateway: GatewayConfig::default(),
            llm: LlmConfig::default(),
            skills: SkillsConfig::default(),
            agent: AgentConfig::default(),
            storage: StorageConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// 获取配置值的快捷方法
    pub fn get_value(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["version"] => Some(self.version.clone()),
            ["server", "port"] => Some(self.server.port.to_string()),
            ["server", "host"] => Some(self.server.host.clone()),
            ["server", "cors"] => Some(self.server.cors.to_string()),
            ["gateway", "enabled"] => Some(self.gateway.enabled.to_string()),
            ["gateway", "bind"] => Some(self.gateway.bind.clone()),
            ["gateway", "auth_token"] => self.gateway.auth_token.clone(),
            ["gateway", "max_connections"] => Some(self.gateway.max_connections.to_string()),
            ["gateway", "heartbeat_interval_secs"] => Some(self.gateway.heartbeat_interval_secs.to_string()),
            ["llm", "default_provider"] => Some(self.llm.default_provider.clone()),
            ["skills", "enabled"] => Some(self.skills.enabled.to_string()),
            ["skills", "auto_reload"] => Some(self.skills.auto_reload.to_string()),
            ["agent", "max_rounds"] => Some(self.agent.max_rounds.to_string()),
            ["agent", "timeout_seconds"] => Some(self.agent.timeout_seconds.to_string()),
            ["storage", "type"] => Some(format!("{:?}", self.storage.storage_type)),
            ["storage", "path"] => self.storage.path.clone(),
            ["logging", "level"] => Some(format!("{:?}", self.logging.level)),
            ["logging", "file"] => self.logging.file.clone(),
            _ => None,
        }
    }

    /// 设置配置值
    pub fn set_value(&mut self, key: &str, value: &str) -> ConfigResult<()> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["server", "port"] => {
                self.server.port = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid port number: {}", value))
                })?;
            }
            ["server", "host"] => {
                self.server.host = value.to_string();
            }
            ["server", "cors"] => {
                self.server.cors = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid boolean: {}", value))
                })?;
            }
            ["gateway", "enabled"] => {
                self.gateway.enabled = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid boolean: {}", value))
                })?;
            }
            ["gateway", "bind"] => {
                self.gateway.bind = value.to_string();
            }
            ["gateway", "auth_token"] => {
                self.gateway.auth_token = Some(value.to_string());
            }
            ["gateway", "max_connections"] => {
                self.gateway.max_connections = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid number: {}", value))
                })?;
            }
            ["gateway", "heartbeat_interval_secs"] => {
                self.gateway.heartbeat_interval_secs = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid number: {}", value))
                })?;
            }
            ["llm", "default_provider"] => {
                self.llm.default_provider = value.to_string();
            }
            ["skills", "enabled"] => {
                self.skills.enabled = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid boolean: {}", value))
                })?;
            }
            ["skills", "auto_reload"] => {
                self.skills.auto_reload = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid boolean: {}", value))
                })?;
            }
            ["agent", "max_rounds"] => {
                self.agent.max_rounds = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid number: {}", value))
                })?;
            }
            ["agent", "timeout_seconds"] => {
                self.agent.timeout_seconds = value.parse().map_err(|_| {
                    ConfigError::Validation(format!("Invalid number: {}", value))
                })?;
            }
            ["logging", "level"] => {
                self.logging.level = value.parse()?;
            }
            ["logging", "file"] => {
                self.logging.file = Some(value.to_string());
            }
            _ => return Err(ConfigError::KeyNotFound(key.to_string())),
        }
        Ok(())
    }
}

/// Server 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub cors: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8081,
            host: "127.0.0.1".to_string(),
            cors: true,
        }
    }
}

/// Gateway 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayConfig {
    /// 是否启用 Gateway
    pub enabled: bool,
    /// 绑定地址 (e.g., "127.0.0.1:18790")
    pub bind: String,
    /// 可选的认证 token
    pub auth_token: Option<String>,
    /// 最大并发连接数
    pub max_connections: usize,
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind: "127.0.0.1:18790".to_string(),
            auth_token: None,
            max_connections: 1000,
            heartbeat_interval_secs: 30,
        }
    }
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmConfig {
    pub default_provider: String,
    pub providers: HashMap<String, ProviderSettings>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();
        
        // Default Copilot provider with device code auth
        providers.insert(
            "copilot".to_string(),
            ProviderSettings {
                enabled: true,
                base_url: "https://api.githubcopilot.com".to_string(),
                model: Some("copilot-chat".to_string()),
                auth: AuthSettings::DeviceCode {
                    client_id: "Iv1.b507a08c87ecfe98".to_string(),
                    device_code_url: Some("https://github.com/login/device/code".to_string()),
                    access_token_url: Some("https://github.com/login/oauth/access_token".to_string()),
                    copilot_token_url: Some("https://api.github.com/copilot_internal/v2/token".to_string()),
                },
                headers: Some(HashMap::from([
                    ("editor-version".to_string(), "vscode/1.99.2".to_string()),
                    ("editor-plugin-version".to_string(), "copilot-chat/0.20.3".to_string()),
                    ("user-agent".to_string(), "GitHubCopilotChat/0.20.3".to_string()),
                ])),
                timeout_seconds: Some(60),
            },
        );
        
        // Default OpenAI provider with API key auth
        providers.insert(
            "openai".to_string(),
            ProviderSettings {
                enabled: false,
                base_url: "https://api.openai.com/v1".to_string(),
                model: Some("gpt-4o-mini".to_string()),
                auth: AuthSettings::ApiKey {
                    env: "OPENAI_API_KEY".to_string(),
                },
                headers: None,
                timeout_seconds: Some(60),
            },
        );

        Self {
            default_provider: "copilot".to_string(),
            providers,
        }
    }
}

/// Provider 配置（用于 bamboo.toml）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderSettings {
    pub enabled: bool,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(flatten)]
    pub auth: AuthSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

/// Authentication settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "auth_type", rename_all = "snake_case")]
pub enum AuthSettings {
    /// API Key authentication - reads from environment variable
    ApiKey {
        env: String,
    },
    /// Bearer token authentication - reads from environment variable
    Bearer {
        env: String,
    },
    /// Device Code OAuth flow (GitHub Copilot)
    DeviceCode {
        client_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        device_code_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        access_token_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        copilot_token_url: Option<String>,
    },
    /// No authentication
    None,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self::None
    }
}

impl AuthSettings {
    /// Get API key from environment if applicable
    pub fn get_api_key(&self) -> Option<String> {
        match self {
            Self::ApiKey { env } => std::env::var(env).ok(),
            _ => None,
        }
    }
    
    /// Get bearer token from environment if applicable
    pub fn get_bearer_token(&self) -> Option<String> {
        match self {
            Self::Bearer { env } => std::env::var(env).ok(),
            _ => None,
        }
    }
}

/// Skills 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillsConfig {
    pub enabled: bool,
    pub auto_reload: bool,
    pub directories: Vec<String>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_reload: true,
            directories: vec!["~/.bamboo/skills".to_string()],
        }
    }
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub max_rounds: u32,
    pub system_prompt: String,
    pub timeout_seconds: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_rounds: 10,
            system_prompt: "You are a helpful assistant".to_string(),
            timeout_seconds: 300,
        }
    }
}

/// Storage 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Jsonl,
    Sqlite,
}

impl Default for StorageType {
    fn default() -> Self {
        Self::Jsonl
    }
}

/// Storage 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageConfig {
    #[serde(rename = "type")]
    pub storage_type: StorageType,
    pub path: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::Jsonl,
            path: Some("~/.bamboo/sessions".to_string()),
        }
    }
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl std::str::FromStr for LogLevel {
    type Err = ConfigError;

    fn from_str(s: &str) -> ConfigResult<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(ConfigError::Validation(format!("Invalid log level: {}", s))),
        }
    }
}

/// Logging 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file: Option<String>,
    #[serde(default = "default_max_size")]
    pub max_size_mb: u64,
    #[serde(default = "default_max_files")]
    pub max_files: u32,
}

fn default_max_size() -> u64 {
    100
}

fn default_max_files() -> u32 {
    5
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file: Some("~/.bamboo/logs/bamboo.log".to_string()),
            max_size_mb: 100,
            max_files: 5,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Watch error: {0}")]
    #[cfg(feature = "hot-reload")]
    Watch(#[from] notify::Error),
}

pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8081);
        assert_eq!(config.llm.default_provider, "copilot");
    }

    #[test]
    fn test_provider_settings_serialization() {
        let settings = ProviderSettings {
            enabled: true,
            base_url: "https://api.githubcopilot.com".to_string(),
            model: Some("copilot-chat".to_string()),
            auth: AuthSettings::DeviceCode {
                client_id: "test-client-id".to_string(),
                device_code_url: None,
                access_token_url: None,
                copilot_token_url: None,
            },
            headers: None,
            timeout_seconds: None,
        };
        
        let toml = toml::to_string(&settings).unwrap();
        assert!(toml.contains("auth_type"));
        assert!(toml.contains("device_code"));
    }
}
