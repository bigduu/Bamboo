//! API Authentication Middleware
//! 
//! Provides Bearer Token authentication for OpenAI-compatible API endpoints.
//! Uses the server's admin_token from configuration for authentication.

use actix_web::HttpResponse;
use crate::state::AppState;

/// Validate API key from Authorization header
/// 
/// Returns Ok(()) if valid or if no admin_token is configured (development mode)
/// Returns Err with error message if invalid
pub async fn validate_api_key(state: &AppState, auth_header: Option<&str>) -> Result<(), String> {
    let config = state.config.get().read().await.clone();
    
    // Check if admin_token is configured
    let admin_token = match &config.server.admin_token {
        Some(token) if !token.is_empty() => token.clone(),
        _ => {
            // No token configured, allow all requests (development mode)
            return Ok(());
        }
    };
    
    // Extract token from header
    let provided_token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => header[7..].to_string(),
        Some(header) => header.to_string(),
        None => return Err("Missing Authorization header".to_string()),
    };
    
    if provided_token != admin_token {
        return Err("Invalid authentication token".to_string());
    }
    
    Ok(())
}

/// Create unauthorized response in OpenAI format
pub fn unauthorized_response(message: &str) -> HttpResponse {
    HttpResponse::Unauthorized().json(serde_json::json!({
        "error": {
            "message": message,
            "type": "authentication_error",
            "code": "invalid_api_key"
        }
    }))
}
