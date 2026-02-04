use actix_web::{web, HttpRequest, HttpResponse};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::state::AppState;
use bamboo_config::{Config, ConfigError, ConfigManager};

const MASKED_VALUE: &str = "***MASKED***";

/// 错误响应结构
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

/// 配置更新响应
#[derive(Debug, Serialize)]
struct ConfigUpdateResponse {
    success: bool,
    message: String,
    sections: Vec<String>,
}

/// 配置重载响应
#[derive(Debug, Serialize)]
struct ConfigReloadResponse {
    success: bool,
    message: String,
    timestamp: String,
}

/// 配置 Schema 响应
#[derive(Debug, Serialize)]
struct ConfigSchemaResponse {
    version: String,
    sections: Map<String, Value>,
}

/// 创建错误响应
fn error_response(status: actix_web::http::StatusCode, message: impl Into<String>) -> HttpResponse {
    HttpResponse::build(status).json(ErrorResponse {
        error: message.into(),
        details: None,
    })
}

/// 创建带详情的错误响应
fn error_response_with_details(
    status: actix_web::http::StatusCode,
    message: impl Into<String>,
    details: impl Into<String>,
) -> HttpResponse {
    HttpResponse::build(status).json(ErrorResponse {
        error: message.into(),
        details: Some(details.into()),
    })
}

/// 将 ConfigError 转换为 HTTP 响应
fn config_error_response(error: ConfigError) -> HttpResponse {
    match error {
        ConfigError::Validation(msg) => {
            error_response(actix_web::http::StatusCode::BAD_REQUEST, msg)
        }
        ConfigError::KeyNotFound(key) => {
            error_response(actix_web::http::StatusCode::NOT_FOUND, format!("Config key not found: {}", key))
        }
        ConfigError::EnvVarNotFound(var) => {
            error_response_with_details(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Environment variable not found",
                format!("Variable: {}", var),
            )
        }
        ConfigError::InvalidPath(path) => {
            error_response(actix_web::http::StatusCode::BAD_REQUEST, format!("Invalid path: {}", path))
        }
        _ => error_response(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        ),
    }
}

/// 检查字段是否为敏感字段
fn is_sensitive_field(key: &str) -> bool {
    let sensitive_patterns = [
        "token", "api_key", "apikey", "secret", "password", "credential",
        "auth_token", "access_token", "refresh_token", "bearer",
    ];
    
    let lower_key = key.to_lowercase();
    sensitive_patterns.iter().any(|pattern| lower_key.contains(pattern))
}

/// 脱敏配置值
fn mask_sensitive_value(key: &str, value: &Value) -> Value {
    if is_sensitive_field(key) {
        if value.is_string() && !value.as_str().unwrap_or("").is_empty() {
            return Value::String(MASKED_VALUE.to_string());
        }
    }
    value.clone()
}

/// 递归脱敏 JSON 对象
fn mask_config_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut masked = Map::new();
            for (key, val) in map {
                if val.is_object() {
                    masked.insert(key.clone(), mask_config_json(val));
                } else {
                    masked.insert(key.clone(), mask_sensitive_value(key, val));
                }
            }
            Value::Object(masked)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(mask_config_json).collect())
        }
        _ => value.clone(),
    }
}

/// 验证认证令牌
async fn verify_auth_token(state: &AppState, req: &HttpRequest) -> Result<(), HttpResponse> {
    let config = state.config.get().read().await.clone();
    
    // 检查是否配置了 admin_token
    let admin_token = match &config.server.admin_token {
        Some(token) if !token.is_empty() => token.clone(),
        _ => {
            // 如果没有配置 admin_token，允许所有请求（开发模式）
            return Ok(());
        }
    };
    
    // 从请求头获取 Authorization
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    // 支持 "Bearer <token>" 或直接使用 token
    let provided_token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        auth_header
    };
    
    if provided_token != admin_token {
        return Err(error_response(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Invalid or missing authentication token",
        ));
    }
    
    Ok(())
}

/// 获取完整配置（敏感信息脱敏）
pub async fn get_config(state: web::Data<AppState>) -> HttpResponse {
    let config = state.config.get().read().await.clone();
    
    // 序列化为 JSON 并脱敏
    match serde_json::to_value(&config) {
        Ok(value) => {
            let masked = mask_config_json(&value);
            HttpResponse::Ok().json(masked)
        }
        Err(e) => error_response(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize config: {}", e),
        ),
    }
}

/// 更新完整配置
pub async fn update_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<Value>,
) -> HttpResponse {
    // 验证权限
    if let Err(resp) = verify_auth_token(&state, &req).await {
        return resp;
    }
    
    let new_config_value = body.into_inner();
    
    // 反序列化为 Config
    let new_config: Config = match serde_json::from_value(new_config_value.clone()) {
        Ok(config) => config,
        Err(e) => {
            return error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                format!("Invalid config format: {}", e),
            );
        }
    };
    
    // 验证配置
    if let Err(e) = ConfigManager::validate(&new_config) {
        return config_error_response(e);
    }
    
    // 更新配置
    if let Err(e) = state.config.update(|config| {
        *config = new_config;
    }).await {
        return config_error_response(e);
    }
    
    // 通知其他组件配置已更新
    state.notify_config_updated(vec![
        "server".to_string(),
        "gateway".to_string(),
        "llm".to_string(),
        "skills".to_string(),
        "agent".to_string(),
        "storage".to_string(),
        "logging".to_string(),
    ]);
    
    HttpResponse::Ok().json(ConfigUpdateResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
        sections: vec![
            "server".to_string(),
            "gateway".to_string(),
            "llm".to_string(),
            "skills".to_string(),
            "agent".to_string(),
            "storage".to_string(),
            "logging".to_string(),
        ],
    })
}

/// 获取特定章节配置
pub async fn get_config_section(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let section = path.into_inner();
    let config = state.config.get().read().await.clone();
    
    // 将配置序列化为 JSON
    let config_json = match serde_json::to_value(&config) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize config: {}", e),
            );
        }
    };
    
    // 获取特定章节
    let section_value = match section.as_str() {
        "server" => config_json.get("server").cloned(),
        "gateway" => config_json.get("gateway").cloned(),
        "llm" => config_json.get("llm").cloned(),
        "skills" => config_json.get("skills").cloned(),
        "agent" => config_json.get("agent").cloned(),
        "storage" => config_json.get("storage").cloned(),
        "logging" => config_json.get("logging").cloned(),
        "version" => Some(Value::String(config.version.clone())),
        _ => None,
    };
    
    match section_value {
        Some(value) => {
            let masked = mask_config_json(&value);
            HttpResponse::Ok().json(masked)
        }
        None => error_response(
            actix_web::http::StatusCode::NOT_FOUND,
            format!("Config section not found: {}", section),
        ),
    }
}

/// 更新特定章节配置
pub async fn update_config_section(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<Value>,
) -> HttpResponse {
    // 验证权限
    if let Err(resp) = verify_auth_token(&state, &req).await {
        return resp;
    }
    
    let section = path.into_inner();
    let section_value = body.into_inner();
    
    // 获取当前配置
    let current_config = state.config.get().read().await.clone();
    
    // 将当前配置序列化为 JSON
    let mut config_json = match serde_json::to_value(&current_config) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize config: {}", e),
            );
        }
    };
    
    // 检查是否为只读字段
    if section == "version" {
        return error_response(
            actix_web::http::StatusCode::FORBIDDEN,
            "Version field is read-only",
        );
    }
    
    // 合并章节配置
    if let Some(obj) = config_json.as_object_mut() {
        // 保留敏感字段的原始值
        if let Some(current_section) = obj.get(&section) {
            let merged = merge_with_sensitive_preservation(
                current_section,
                &section_value,
            );
            obj.insert(section.clone(), merged);
        } else {
            obj.insert(section.clone(), section_value);
        }
    }
    
    // 反序列化为 Config
    let new_config: Config = match serde_json::from_value(config_json) {
        Ok(config) => config,
        Err(e) => {
            return error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                format!("Invalid config format: {}", e),
            );
        }
    };
    
    // 验证配置
    if let Err(e) = ConfigManager::validate(&new_config) {
        return config_error_response(e);
    }
    
    // 更新配置
    if let Err(e) = state.config.update(|config| {
        *config = new_config;
    }).await {
        return config_error_response(e);
    }
    
    // 通知其他组件配置已更新
    state.notify_config_updated(vec![section.clone()]);
    
    HttpResponse::Ok().json(ConfigUpdateResponse {
        success: true,
        message: format!("Section '{}' updated successfully", section),
        sections: vec![section],
    })
}

/// 合并配置，保留敏感字段
fn merge_with_sensitive_preservation(current: &Value, new: &Value) -> Value {
    match (current, new) {
        (Value::Object(current_map), Value::Object(new_map)) => {
            let mut result = current_map.clone();
            
            for (key, new_val) in new_map {
                if let Some(current_val) = current_map.get(key) {
                    // 检查是否为敏感字段
                    if is_sensitive_field(key) {
                        // 如果新值为空或 MASKED_VALUE，保留原值
                        if let Some(s) = new_val.as_str() {
                            if s.is_empty() || s == MASKED_VALUE {
                                continue; // 保留原值
                            }
                        }
                    }
                    
                    // 递归合并
                    if current_val.is_object() && new_val.is_object() {
                        result.insert(key.clone(), merge_with_sensitive_preservation(current_val, new_val));
                    } else {
                        result.insert(key.clone(), new_val.clone());
                    }
                } else {
                    result.insert(key.clone(), new_val.clone());
                }
            }
            
            Value::Object(result)
        }
        _ => new.clone(),
    }
}

/// 重新加载配置文件
pub async fn reload_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    // 验证权限
    if let Err(resp) = verify_auth_token(&state, &req).await {
        return resp;
    }
    
    // 重新加载配置
    match state.config.reload().await {
        Ok(()) => {
            // 通知其他组件配置已更新
            state.notify_config_updated(vec![
                "server".to_string(),
                "gateway".to_string(),
                "llm".to_string(),
                "skills".to_string(),
                "agent".to_string(),
                "storage".to_string(),
                "logging".to_string(),
            ]);
            
            HttpResponse::Ok().json(ConfigReloadResponse {
                success: true,
                message: "Configuration reloaded successfully".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(e) => config_error_response(e),
    }
}

/// 获取配置 Schema
pub async fn get_config_schema() -> HttpResponse {
    let mut sections = Map::new();
    
    // Server 配置 Schema
    let mut server_schema = Map::new();
    server_schema.insert("port".to_string(), json!({
        "type": "integer",
        "description": "Server port number",
        "minimum": 1,
        "maximum": 65535,
        "default": 8081
    }));
    server_schema.insert("host".to_string(), json!({
        "type": "string",
        "description": "Server host address",
        "format": "hostname",
        "default": "127.0.0.1"
    }));
    server_schema.insert("cors".to_string(), json!({
        "type": "boolean",
        "description": "Enable CORS",
        "default": true
    }));
    server_schema.insert("admin_token".to_string(), json!({
        "type": "string",
        "description": "Admin token for config API authentication",
        "sensitive": true,
        "nullable": true
    }));
    sections.insert("server".to_string(), Value::Object(server_schema));
    
    // Gateway 配置 Schema
    let mut gateway_schema = Map::new();
    gateway_schema.insert("enabled".to_string(), json!({
        "type": "boolean",
        "description": "Enable WebSocket Gateway",
        "default": true
    }));
    gateway_schema.insert("bind".to_string(), json!({
        "type": "string",
        "description": "Gateway bind address (host:port)",
        "format": "socket_addr",
        "default": "127.0.0.1:18790"
    }));
    gateway_schema.insert("auth_token".to_string(), json!({
        "type": "string",
        "description": "Gateway authentication token",
        "sensitive": true,
        "nullable": true
    }));
    gateway_schema.insert("max_connections".to_string(), json!({
        "type": "integer",
        "description": "Maximum concurrent connections",
        "minimum": 1,
        "default": 1000
    }));
    gateway_schema.insert("heartbeat_interval_secs".to_string(), json!({
        "type": "integer",
        "description": "Heartbeat interval in seconds",
        "minimum": 1,
        "default": 30
    }));
    sections.insert("gateway".to_string(), Value::Object(gateway_schema));
    
    // LLM 配置 Schema
    let mut llm_schema = Map::new();
    llm_schema.insert("default_provider".to_string(), json!({
        "type": "string",
        "description": "Default LLM provider",
        "enum": ["copilot", "openai"],
        "default": "copilot"
    }));
    llm_schema.insert("providers".to_string(), json!({
        "type": "object",
        "description": "LLM provider configurations",
        "additionalProperties": {
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" },
                "base_url": { "type": "string", "format": "uri" },
                "model": { "type": "string" },
                "auth_type": { 
                    "type": "string", 
                    "enum": ["api_key", "bearer", "device_code", "none"] 
                },
                "timeout_seconds": { "type": "integer", "minimum": 1 }
            }
        }
    }));
    sections.insert("llm".to_string(), Value::Object(llm_schema));
    
    // Skills 配置 Schema
    let mut skills_schema = Map::new();
    skills_schema.insert("enabled".to_string(), json!({
        "type": "boolean",
        "description": "Enable skills system",
        "default": true
    }));
    skills_schema.insert("auto_reload".to_string(), json!({
        "type": "boolean",
        "description": "Auto-reload skills on file changes",
        "default": true
    }));
    skills_schema.insert("directories".to_string(), json!({
        "type": "array",
        "description": "Skill directories",
        "items": { "type": "string" },
        "default": ["~/.bamboo/skills"]
    }));
    sections.insert("skills".to_string(), Value::Object(skills_schema));
    
    // Agent 配置 Schema
    let mut agent_schema = Map::new();
    agent_schema.insert("max_rounds".to_string(), json!({
        "type": "integer",
        "description": "Maximum conversation rounds",
        "minimum": 1,
        "default": 10
    }));
    agent_schema.insert("timeout_seconds".to_string(), json!({
        "type": "integer",
        "description": "Agent timeout in seconds",
        "minimum": 1,
        "default": 300
    }));
    agent_schema.insert("system_prompt".to_string(), json!({
        "type": "string",
        "description": "Default system prompt",
        "default": "You are a helpful assistant"
    }));
    sections.insert("agent".to_string(), Value::Object(agent_schema));
    
    // Storage 配置 Schema
    let mut storage_schema = Map::new();
    storage_schema.insert("type".to_string(), json!({
        "type": "string",
        "description": "Storage type",
        "enum": ["jsonl", "sqlite"],
        "default": "jsonl"
    }));
    storage_schema.insert("path".to_string(), json!({
        "type": "string",
        "description": "Storage path",
        "nullable": true,
        "default": "~/.bamboo/sessions"
    }));
    sections.insert("storage".to_string(), Value::Object(storage_schema));
    
    // Logging 配置 Schema
    let mut logging_schema = Map::new();
    logging_schema.insert("level".to_string(), json!({
        "type": "string",
        "description": "Log level",
        "enum": ["debug", "info", "warn", "error"],
        "default": "info"
    }));
    logging_schema.insert("file".to_string(), json!({
        "type": "string",
        "description": "Log file path",
        "nullable": true,
        "default": "~/.bamboo/logs/bamboo.log"
    }));
    logging_schema.insert("max_size_mb".to_string(), json!({
        "type": "integer",
        "description": "Maximum log file size in MB",
        "minimum": 1,
        "default": 100
    }));
    logging_schema.insert("max_files".to_string(), json!({
        "type": "integer",
        "description": "Maximum number of log files to keep",
        "minimum": 1,
        "default": 5
    }));
    sections.insert("logging".to_string(), Value::Object(logging_schema));
    
    HttpResponse::Ok().json(ConfigSchemaResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        sections,
    })
}
