use actix_web::{web, App, HttpServer};
use std::io;
use bamboo_config::ConfigManager;

use crate::handlers;
use crate::state::AppState;

pub async fn run_server(
    config_manager: ConfigManager,
    port: u16,
    provider: &str,
    llm_base_url: String,
    model: String,
    api_key: String,
) -> io::Result<()> {
    log::info!("Initializing server with provider: {}, base URL: {}", provider, llm_base_url);
    
    // 获取 host 配置
    let host = config_manager.get().read().await.server.host.clone();
    
    let state = web::Data::new(
        AppState::new_with_config(config_manager, provider, llm_base_url, model, api_key).await
    );

    let state_clone = state.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(state_clone.clone())
            .service(
                web::scope("/api/v1")
                    .route("/chat", web::post().to(handlers::chat::handler))
                    .route("/stream/{session_id}", web::get().to(handlers::stream::handler))
                    .route("/stop/{session_id}", web::post().to(handlers::stop::handler))
                    .route("/history/{session_id}", web::get().to(handlers::history::handler))
                    .route("/health", web::get().to(handlers::health::handler))
                    .route("/config", web::get().to(handlers::config::get_config))
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

// 保留旧函数以兼容测试
pub async fn run_server_with_config(
    port: u16,
    provider: &str,
    llm_base_url: String,
    model: String,
    api_key: String,
) -> io::Result<()> {
    let config_manager = ConfigManager::load_default().await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    
    run_server(config_manager, port, provider, llm_base_url, model, api_key).await
}
