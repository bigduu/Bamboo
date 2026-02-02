//! Unified Copilot client with configurable UI

use reqwest::Client;
use crate::auth::{get_device_code, present_device_code, authenticate, AuthError};
use crate::cache::TokenCache;
use crate::forward::{ForwardClient, ChatCompletionRequest, Model, ForwardError};
use crate::ui::UiConfig;

/// Unified Copilot client
/// 
/// Handles authentication, token caching, and HTTP forwarding
/// with configurable UI behavior.
pub struct CopilotClient {
    http_client: Client,
    forward_client: Option<ForwardClient>,
    ui_config: UiConfig,
    token: Option<String>,
}

impl CopilotClient {
    /// Create new client with UI configuration
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use copilot_forward::{CopilotClient, UiConfig};
    /// 
    /// # async fn example() -> anyhow::Result<()> {
    /// // GUI mode (default)
    /// let client = CopilotClient::new(UiConfig::gui()).await?;
    /// 
    /// // Headless mode
    /// let client = CopilotClient::new(UiConfig::headless()).await?;
    /// 
    /// // Custom configuration
    /// let client = CopilotClient::new(
    ///     UiConfig::none().with_console().with_browser()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(ui_config: UiConfig) -> Result<Self, ClientError> {
        let http_client = Client::new();
        
        let mut client = Self {
            http_client,
            forward_client: None,
            ui_config,
            token: None,
        };
        
        // Try to authenticate
        client.authenticate().await?;
        
        Ok(client)
    }
    
    /// Create client with existing token (skips authentication)
    pub fn with_token(token: impl Into<String>, ui_config: UiConfig) -> Self {
        let token = token.into();
        let forward_client = ForwardClient::new(&token);
        
        Self {
            http_client: Client::new(),
            forward_client: Some(forward_client),
            ui_config,
            token: Some(token),
        }
    }
    
    /// Authenticate (load cache or perform device code flow)
    async fn authenticate(&mut self) -> Result<(), ClientError> {
        // 1. Try to load from cache
        if let Some(cache) = TokenCache::load_async().await {
            let remaining = cache.remaining_seconds();
            if self.ui_config.print_console {
                println!("✅ Using cached Copilot token (expires in {} minutes)", remaining / 60);
            }
            
            self.token = Some(cache.token.clone());
            self.forward_client = Some(ForwardClient::new(&cache.token));
            return Ok(());
        }
        
        // 2. Need to authenticate
        if self.ui_config.print_console {
            println!("\n🔑 Copilot authentication required");
        }
        
        // Get device code
        let device_code = get_device_code(&self.http_client)
            .await
            .map_err(|e| ClientError::Auth(e))?;
        
        // Present to user
        present_device_code(&device_code, &self.ui_config)
            .await
            .map_err(|e| ClientError::Auth(e))?;
        
        // Authenticate
        let copilot_token = authenticate(&self.http_client, &device_code)
            .await
            .map_err(|e| ClientError::Auth(e))?;
        
        // Cache token
        let cache = TokenCache::from_copilot_token(&copilot_token);
        if let Err(e) = cache.save_async().await {
            log::warn!("Failed to cache token: {}", e);
        }
        
        self.token = Some(copilot_token.token.clone());
        self.forward_client = Some(ForwardClient::new(&copilot_token.token));
        
        if self.ui_config.print_console {
            println!("\n✅ Authentication successful!\n");
        }
        
        Ok(())
    }
    
    /// Check if client is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some() && self.forward_client.is_some()
    }
    
    /// Get current token (if authenticated)
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
    
    /// Get UI configuration
    pub fn ui_config(&self) -> &UiConfig {
        &self.ui_config
    }
    
    /// Chat completions API
    pub async fn chat_completions(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<reqwest::Response, ClientError> {
        let forward_client = self.forward_client.as_ref()
            .ok_or(ClientError::NotAuthenticated)?;
        
        forward_client.chat_completions(request)
            .await
            .map_err(|e| ClientError::Forward(e))
    }
    
    /// Get available models
    pub async fn models(&self) -> Result<Vec<Model>, ClientError> {
        let forward_client = self.forward_client.as_ref()
            .ok_or(ClientError::NotAuthenticated)?;
        
        forward_client.models()
            .await
            .map_err(|e| ClientError::Forward(e))
    }
    
    /// Check if token is valid
    pub async fn check_token(&self) -> Result<(), ClientError> {
        let forward_client = self.forward_client.as_ref()
            .ok_or(ClientError::NotAuthenticated)?;
        
        forward_client.check_token()
            .await
            .map_err(|e| ClientError::Forward(e))
    }
    
    /// Logout - delete cached token
    pub fn logout(&mut self) -> Result<(), ClientError> {
        TokenCache::delete()
            .map_err(|e| ClientError::Auth(e))?;
        
        self.token = None;
        self.forward_client = None;
        
        if self.ui_config.print_console {
            println!("Logged out successfully");
        }
        
        Ok(())
    }
}

/// Client errors
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Forwarding error: {0}")]
    Forward(#[from] ForwardError),
    
    #[error("Not authenticated")]
    NotAuthenticated,
    
    #[error("Token expired")]
    TokenExpired,
}

// Types are re-exported from lib.rs
