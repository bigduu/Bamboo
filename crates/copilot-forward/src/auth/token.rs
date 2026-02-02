//! Token management for GitHub Copilot

use reqwest::Client;
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use crate::auth::device_code::DeviceCodeResponse;
use crate::auth::device_code::AuthError;

const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

/// Access token response from GitHub
#[derive(Debug, Deserialize)]
pub struct AccessTokenResponse {
    #[serde(rename = "access_token")]
    pub access_token: Option<String>,
    #[serde(rename = "token_type")]
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
    device_code: &DeviceCodeResponse,
) -> Result<String, AuthError> {
    let params = [
        ("client_id", GITHUB_CLIENT_ID),
        ("device_code", device_code.device_code.as_str()),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];
    
    let start = std::time::Instant::now();
    let max_duration = Duration::from_secs(device_code.expires_in);
    let poll_interval = Duration::from_secs(device_code.interval.max(5));
    
    if device_code.interval > 0 {
        println!("  🔄 Polling for authorization...");
    }
    
    loop {
        if start.elapsed() > max_duration {
            return Err(AuthError::Expired);
        }
        
        let response = client
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Http(e.to_string()))?;
        
        let token_response: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| AuthError::Parse(e.to_string()))?;
        
        if let Some(token) = token_response.access_token {
            println!("  ✅ Access token received!");
            return Ok(token);
        }
        
        if let Some(error) = token_response.error {
            match error.as_str() {
                "authorization_pending" => {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
                "slow_down" => {
                    println!("\n  ⚠️  Server requested slower polling");
                    sleep(Duration::from_secs(device_code.interval + 5)).await;
                    continue;
                }
                "expired_token" => return Err(AuthError::Expired),
                "access_denied" => return Err(AuthError::Denied),
                _ => {
                    let desc = token_response.error_description.unwrap_or_default();
                    return Err(AuthError::Api(format!("{}: {}", error, desc)));
                }
            }
        }
        
        sleep(poll_interval).await;
    }
}

/// Get Copilot token from access token
pub async fn get_copilot_token(
    client: &Client,
    access_token: &str,
) -> Result<CopilotToken, AuthError> {
    let response = client
        .get(COPILOT_TOKEN_URL)
        .header("Authorization", format!("token {}", access_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AuthError::Http(e.to_string()))?;
    
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(AuthError::Api(format!("Copilot token request failed: HTTP {} - {}", status, text)));
    }
    
    let copilot_token: CopilotToken = response
        .json()
        .await
        .map_err(|e| AuthError::Parse(e.to_string()))?;
    
    if !copilot_token.chat_enabled {
        return Err(AuthError::Api("Copilot chat is not enabled for this account".to_string()));
    }
    
    println!("  ✅ Copilot token received!");
    
    Ok(copilot_token)
}

/// Full authentication flow
pub async fn authenticate(
    client: &Client,
    device_code: &DeviceCodeResponse,
) -> Result<CopilotToken, AuthError> {
    let access_token = poll_access_token(client, device_code).await?;
    let copilot_token = get_copilot_token(client, &access_token).await?;
    Ok(copilot_token)
}
