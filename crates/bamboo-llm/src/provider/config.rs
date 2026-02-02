use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Authentication configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// API Key authentication (OpenAI style)
    ApiKey { 
        /// The API key
        key: String 
    },
    /// Bearer token authentication
    Bearer { 
        /// The bearer token
        token: String 
    },
    /// Device Code OAuth flow (GitHub Copilot style)
    DeviceCode {
        /// OAuth client ID
        client_id: String,
        /// URL to request device code
        #[serde(default = "default_device_code_url")]
        device_code_url: String,
        /// URL to exchange device code for access token
        #[serde(default = "default_access_token_url")]
        access_token_url: String,
        /// URL to get the Copilot token
        #[serde(default = "default_copilot_token_url")]
        copilot_token_url: String,
    },
    /// No authentication
    None,
}

fn default_device_code_url() -> String {
    "https://github.com/login/device/code".to_string()
}

fn default_access_token_url() -> String {
    "https://github.com/login/oauth/access_token".to_string()
}

fn default_copilot_token_url() -> String {
    "https://api.github.com/copilot_internal/v2/token".to_string()
}

impl AuthConfig {
    /// Create API key auth from environment variable
    pub fn from_env(env_var: &str) -> Option<Self> {
        std::env::var(env_var).ok().map(|key| Self::ApiKey { key })
    }
    
    /// Create a default device code auth for GitHub Copilot
    pub fn copilot_default() -> Self {
        Self::DeviceCode {
            client_id: "Iv1.b507a08c87ecfe98".to_string(),
            device_code_url: default_device_code_url(),
            access_token_url: default_access_token_url(),
            copilot_token_url: default_copilot_token_url(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::None
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub provider_id: String,
    /// Base URL for the API
    pub base_url: String,
    /// Authentication configuration
    #[serde(flatten)]
    pub auth: AuthConfig,
    /// Default model to use
    pub model: String,
    /// Request timeout in seconds
    #[serde(with = "serde_duration", default = "default_timeout")]
    pub timeout: Duration,
    /// Additional headers to include
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

impl ProviderConfig {
    /// Create a new provider config
    pub fn new(provider_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            base_url: base_url.into(),
            auth: AuthConfig::None,
            model: "gpt-4o-mini".to_string(),
            timeout: Duration::from_secs(60),
            headers: HashMap::new(),
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.auth = AuthConfig::ApiKey { key: key.into() };
        self
    }

    /// Set bearer token
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = AuthConfig::Bearer { token: token.into() };
        self
    }

    /// Set device code auth
    pub fn with_device_code(mut self, client_id: impl Into<String>) -> Self {
        self.auth = AuthConfig::DeviceCode {
            client_id: client_id.into(),
            device_code_url: default_device_code_url(),
            access_token_url: default_access_token_url(),
            copilot_token_url: default_copilot_token_url(),
        };
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
    
    /// Set multiple headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_id: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            auth: AuthConfig::None,
            model: "gpt-4o-mini".to_string(),
            timeout: Duration::from_secs(60),
            headers: HashMap::new(),
        }
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(60)
}

// Custom serialization for Duration
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
