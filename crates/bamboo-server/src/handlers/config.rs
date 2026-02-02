use actix_web::{HttpResponse, web};
use crate::state::AppState;
use serde_json::json;

/// 获取当前配置（敏感信息会被隐藏）
pub async fn get_config(data: web::Data<AppState>) -> HttpResponse {
    let config = data.config.get().read().await.clone();
    
    // 构建一个不包含敏感信息的配置响应
    let llm_providers: serde_json::Map<String, serde_json::Value> = config
        .llm
        .providers
        .iter()
        .map(|(name, provider)| {
            let auth_type = match &provider.auth {
                bamboo_config::AuthSettings::ApiKey { .. } => "api_key",
                bamboo_config::AuthSettings::Bearer { .. } => "bearer",
                bamboo_config::AuthSettings::DeviceCode { .. } => "device_code",
                bamboo_config::AuthSettings::None => "none",
            };
            
            let mut provider_json = serde_json::json!({
                "enabled": provider.enabled,
                "base_url": provider.base_url,
                "model": provider.model,
                "auth_type": auth_type,
            });
            
            // 如果有认证，显示已配置但不显示值
            if !matches!(provider.auth, bamboo_config::AuthSettings::None) {
                provider_json["auth_configured"] = serde_json::json!(true);
            }
            
            (name.clone(), provider_json)
        })
        .collect();
    
    let response = json!({
        "version": config.version,
        "server": {
            "port": config.server.port,
            "host": config.server.host,
            "cors": config.server.cors,
        },
        "gateway": {
            "enabled": config.gateway.enabled,
            "bind": config.gateway.bind,
            "max_connections": config.gateway.max_connections,
            "heartbeat_interval_secs": config.gateway.heartbeat_interval_secs,
        },
        "llm": {
            "default_provider": config.llm.default_provider,
            "providers": llm_providers,
        },
        "skills": {
            "enabled": config.skills.enabled,
            "auto_reload": config.skills.auto_reload,
            "directories": config.skills.directories,
        },
        "agent": {
            "max_rounds": config.agent.max_rounds,
            "timeout_seconds": config.agent.timeout_seconds,
            // 不返回 system_prompt 内容
        },
        "storage": {
            "type": config.storage.storage_type,
            "path": config.storage.path,
        },
        "logging": {
            "level": config.logging.level,
            "file": config.logging.file,
            "max_size_mb": config.logging.max_size_mb,
            "max_files": config.logging.max_files,
        },
    });
    
    HttpResponse::Ok().json(response)
}
