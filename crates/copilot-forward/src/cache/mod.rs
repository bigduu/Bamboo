//! Token cache management

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::auth::CopilotToken;
use crate::auth::AuthError;

/// Token cache structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenCache {
    pub token: String,
    #[serde(rename = "expires_at")]
    pub expires_at: u64,
    #[serde(rename = "chat_enabled")]
    pub chat_enabled: bool,
}

impl TokenCache {
    /// Get default cache directory
    fn cache_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".copilot-forward")
    }
    
    /// Get cache file path
    pub fn cache_path() -> PathBuf {
        Self::cache_dir().join("token.json")
    }
    
    /// Load token from cache
    pub fn load() -> Option<Self> {
        let path = Self::cache_path();
        if !path.exists() {
            return None;
        }
        
        let content = std::fs::read_to_string(&path).ok()?;
        let cache: TokenCache = serde_json::from_str(&content).ok()?;
        
        if cache.is_valid() {
            Some(cache)
        } else {
            None
        }
    }
    
    /// Load token from cache (async version)
    pub async fn load_async() -> Option<Self> {
        tokio::task::spawn_blocking(Self::load)
            .await
            .ok()
            .flatten()
    }
    
    /// Save token to cache
    pub fn save(&self) -> Result<(), AuthError> {
        let path = Self::cache_path();
        
        // Create directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AuthError::Ui(format!("Failed to create cache directory: {}", e)))?;
        }
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| AuthError::Ui(format!("Failed to serialize token: {}", e)))?;
        
        std::fs::write(&path, content)
            .map_err(|e| AuthError::Ui(format!("Failed to write token cache: {}", e)))?;
        
        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&path, perms);
        }
        
        Ok(())
    }
    
    /// Save token to cache (async version)
    pub async fn save_async(&self) -> Result<(), AuthError> {
        let cache = self.clone();
        tokio::task::spawn_blocking(move || cache.save())
            .await
            .map_err(|e| AuthError::Ui(format!("Failed to save token: {}", e)))?
    }
    
    /// Delete cache file
    pub fn delete() -> Result<(), AuthError> {
        let path = Self::cache_path();
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| AuthError::Ui(format!("Failed to delete token cache: {}", e)))?;
        }
        Ok(())
    }
    
    /// Check if token is still valid (with 5 minute buffer)
    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Consider expired 5 minutes before actual expiry
        self.expires_at > now + 300
    }
    
    /// Get remaining seconds until expiry
    pub fn remaining_seconds(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        self.expires_at as i64 - now
    }
    
    /// Convert from CopilotToken
    pub fn from_copilot_token(token: &CopilotToken) -> Self {
        Self {
            token: token.token.clone(),
            expires_at: token.expires_at,
            chat_enabled: token.chat_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_validity() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Valid: expires in 1 hour
        let cache = TokenCache {
            token: "test".to_string(),
            expires_at: now + 3600,
            chat_enabled: true,
        };
        assert!(cache.is_valid());
        
        // Invalid: expires in 1 minute
        let cache = TokenCache {
            token: "test".to_string(),
            expires_at: now + 60,
            chat_enabled: true,
        };
        assert!(!cache.is_valid());
    }
}
