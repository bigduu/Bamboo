use reqwest::Client;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

use crate::error::AuthError;

/// Access token response from GitHub
#[derive(Debug, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "error_description")]
    pub error_description: Option<String>,
}

/// Copilot token response
#[derive(Debug, Deserialize, Clone)]
pub struct CopilotToken {
    pub token: String,
    #[serde(rename = "expires_at")]
    pub expires_at: u64,
    #[serde(rename = "chat_enabled")]
    pub chat_enabled: bool,
    #[serde(rename = "chat_jwt")]
    pub chat_jwt: Option<String>,
}

/// Poll for access token
pub async fn poll_access_token(
    client: &Client,
    client_id: &str,
    access_token_url: &str,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<String, AuthError> {
    let params = [
        ("client_id", client_id),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];
    
    let start = std::time::Instant::now();
    let max_duration = Duration::from_secs(expires_in);
    let poll_interval = Duration::from_secs(interval.max(5)); // Minimum 5 seconds
    
    println!("  🔄 Polling for authorization...");
    
    loop {
        // Check if expired
        if start.elapsed() > max_duration {
            return Err(AuthError::DeviceCodeExpired);
        }
        
        // Request access token
        let response = client
            .post(access_token_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;
        
        let token_response: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| AuthError::Failed(format!("JSON parse error: {}", e)))?;
        
        // Check if we got the token
        if let Some(token) = token_response.access_token {
            println!("  ✅ Access token received!");
            return Ok(token);
        }
        
        // Check for errors
        if let Some(error) = token_response.error {
            match error.as_str() {
                "authorization_pending" => {
                    // Still waiting, continue polling
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
                "slow_down" => {
                    // Server asks us to slow down
                    println!("\n  ⚠️  Server requested slower polling, increasing interval...");
                    sleep(Duration::from_secs(interval + 5)).await;
                    continue;
                }
                "expired_token" => {
                    return Err(AuthError::DeviceCodeExpired);
                }
                "access_denied" => {
                    return Err(AuthError::AccessDenied);
                }
                _ => {
                    let desc = token_response.error_description.unwrap_or_default();
                    return Err(AuthError::Failed(
                        format!("Auth error: {} - {}", error, desc)
                    ));
                }
            }
        }
        
        // Wait before next poll
        sleep(poll_interval).await;
    }
}

/// Get Copilot token from access token
pub async fn get_copilot_token(
    client: &Client,
    copilot_token_url: &str,
    access_token: &str,
) -> Result<CopilotToken, AuthError> {
    let response = client
        .get(copilot_token_url)
        .header("Authorization", format!("token {}", access_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AuthError::Network(e.to_string()))?;
    
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(AuthError::Failed(format!(
            "Copilot token request failed: HTTP {} - {}", status, text
        )));
    }
    
    let copilot_token: CopilotToken = response
        .json()
        .await
        .map_err(|e| AuthError::Failed(format!("JSON parse error: {}", e)))?;
    
    if !copilot_token.chat_enabled {
        return Err(AuthError::Failed(
            "Copilot chat is not enabled for this account.".to_string()
        ));
    }
    
    println!("  ✅ Copilot token received!");
    
    Ok(copilot_token)
}
