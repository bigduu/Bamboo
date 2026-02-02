use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::error::{LLMError, AuthError};
use crate::auth::token::{get_copilot_token, poll_access_token};
use crate::auth::cache::TokenCache;

const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";

/// Device code response from GitHub
#[derive(Debug, Deserialize, Clone)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(rename = "expires_in")]
    pub expires_in: u64,
    pub interval: u64,
}

/// Device Code Authenticator for GitHub Copilot
#[derive(Debug)]
pub struct DeviceCodeAuth {
    client_id: String,
    device_code_url: String,
    access_token_url: String,
    copilot_token_url: String,
    http_client: Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl DeviceCodeAuth {
    /// Create a new DeviceCodeAuth with default GitHub settings
    pub fn new() -> Self {
        Self::with_config(
            GITHUB_CLIENT_ID.to_string(),
            DEVICE_CODE_URL.to_string(),
            "https://github.com/login/oauth/access_token".to_string(),
            "https://api.github.com/copilot_internal/v2/token".to_string(),
        )
    }
    
    /// Create a new DeviceCodeAuth with custom configuration
    pub fn with_config(
        client_id: String,
        device_code_url: String,
        access_token_url: String,
        copilot_token_url: String,
    ) -> Self {
        Self {
            client_id,
            device_code_url,
            access_token_url,
            copilot_token_url,
            http_client: Client::new(),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize and try to load cached token
    pub async fn init(&self) -> Result<(), AuthError> {
        if let Some(cache) = TokenCache::load().await {
            let mut token_cache = self.token_cache.write().map_err(|_| {
                AuthError::Failed("Failed to lock token cache".to_string())
            })?;
            *token_cache = Some(cache);
        }
        Ok(())
    }
    
    /// Check if we have a valid cached token
    pub fn is_authenticated(&self) -> bool {
        if let Ok(cache) = self.token_cache.read() {
            if let Some(ref cache) = *cache {
                return cache.is_valid();
            }
        }
        false
    }
    
    /// Get the current token if available
    fn get_token(&self) -> Result<String, AuthError> {
        let cache = self.token_cache.read().map_err(|_| {
            AuthError::Failed("Failed to lock token cache".to_string())
        })?;
        
        if let Some(ref cache) = *cache {
            if cache.is_valid() {
                return Ok(cache.token.clone());
            }
        }
        
        Err(AuthError::TokenExpired)
    }
    
    /// Perform the full device code flow authentication
    pub async fn authenticate(&self) -> Result<(), AuthError> {
        // Step 1: Request device code
        let device_code = self.request_device_code().await?;
        
        // Step 2: Present to user
        present_device_code(&device_code);
        
        // Step 3: Poll for access token
        let access_token = poll_access_token(
            &self.http_client,
            &self.client_id,
            &self.access_token_url,
            &device_code.device_code,
            device_code.interval,
            device_code.expires_in,
        ).await?;
        
        // Step 4: Exchange for Copilot token
        let copilot_token = get_copilot_token(
            &self.http_client,
            &self.copilot_token_url,
            &access_token,
        ).await?;
        
        // Step 5: Save to cache
        let cache = TokenCache {
            token: copilot_token.token,
            expires_at: copilot_token.expires_at,
        };
        cache.save().await?;
        
        // Step 6: Update in-memory cache
        let mut token_cache = self.token_cache.write().map_err(|_| {
            AuthError::Failed("Failed to lock token cache".to_string())
        })?;
        *token_cache = Some(cache);
        
        println!("  ✅ Authentication successful!");
        
        Ok(())
    }
    
    /// Request device code from GitHub
    async fn request_device_code(&self) -> Result<DeviceCodeResponse, AuthError> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("scope", "read:user"),
        ];
        
        let response = self.http_client
            .post(&self.device_code_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;
        
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AuthError::Failed(format!(
                "Device code request failed: HTTP {} - {}",
                status, text
            )));
        }
        
        let device_code: DeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| AuthError::Failed(format!("JSON parse error: {}", e)))?;
        
        Ok(device_code)
    }
    
    /// Logout and clear token cache
    pub async fn logout(&self) -> Result<(), AuthError> {
        TokenCache::delete().await?;
        let mut cache = self.token_cache.write().map_err(|_| {
            AuthError::Failed("Failed to lock token cache".to_string())
        })?;
        *cache = None;
        Ok(())
    }
}

impl Default for DeviceCodeAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::Authenticator for DeviceCodeAuth {
    async fn get_auth_header(&self) -> Result<Option<(String, String)>, LLMError> {
        let token = self.get_token()?;
        Ok(Some((
            "Authorization".to_string(),
            format!("Bearer {}", token),
        )))
    }
    
    async fn needs_refresh(&self) -> bool {
        if let Ok(cache) = self.token_cache.read() {
            if let Some(ref cache) = *cache {
                // Refresh if less than 5 minutes remaining
                return cache.remaining_seconds() < 300;
            }
        }
        true
    }
    
    async fn refresh(&self) -> Result<(), LLMError> {
        self.authenticate().await?;
        Ok(())
    }
}

/// Present device code to user
pub fn present_device_code(device_code: &DeviceCodeResponse) {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║     🔐 GitHub Copilot Authorization Required              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  1. Open your browser and navigate to:");
    println!("     {}", device_code.verification_uri);
    println!();
    println!("  2. Enter the following code:");
    println!();
    println!("     ┌─────────────────────────┐");
    println!("     │  {:^23} │", device_code.user_code);
    println!("     └─────────────────────────┘");
    println!();
    println!("  3. Click 'Authorize' and wait...");
    println!();
    println!("  ⏳ Waiting for authorization (expires in {} seconds)...", device_code.expires_in);
    println!();
}

/// Get device code from GitHub (standalone function)
pub async fn get_device_code(client: &Client) -> Result<DeviceCodeResponse, String> {
    let params = [
        ("client_id", GITHUB_CLIENT_ID),
        ("scope", "read:user"),
    ];
    
    let response = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to request device code: {}", e))?;
    
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Device code request failed: HTTP {} - {}", status, text));
    }
    
    let device_code: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse device code response: {}", e))?;
    
    Ok(device_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_client_id() {
        assert_eq!(GITHUB_CLIENT_ID, "Iv1.b507a08c87ecfe98");
    }
    
    #[test]
    fn test_device_code_auth_new() {
        let auth = DeviceCodeAuth::new();
        assert_eq!(auth.client_id, GITHUB_CLIENT_ID);
    }
}
