//! HTTP Server (actix-web)
//!
//! Uses shared AppState so HTTP handlers and AgentRunner share the same EventBus.

use std::io;
use std::sync::Arc;

use actix_web::{middleware::Logger, web, App, HttpServer};

use crate::handlers;
use crate::state::AppState;

/// Run the HTTP server with the provided shared state.
pub async fn run_server(state: Arc<AppState>, port: u16) -> io::Result<()> {
    let host = {
        let config = state.config.get().read().await.clone();
        config.server.host
    };
    let bind_addr = format!("{}:{}", host, port);
    let data = web::Data::new(state.as_ref().clone());

    log::info!("HTTP server listening on http://{}", bind_addr);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(data.clone())
            .route("/health", web::get().to(handlers::health::handler))
            .service(
                web::scope("/api/v1")
                    .route("/health", web::get().to(handlers::health::handler))
                    .route("/chat", web::post().to(handlers::chat::handler))
                    .route("/stream/{session_id}", web::get().to(handlers::stream::handler))
                    .route("/history/{session_id}", web::get().to(handlers::history::handler))
                    .route("/stop/{session_id}", web::post().to(handlers::stop::handler))
                    // Config API
                    .route("/config", web::get().to(handlers::config::get_config))
                    .route("/config", web::post().to(handlers::config::update_config))
                    .route("/config/{section}", web::get().to(handlers::config::get_config_section))
                    .route("/config/{section}", web::post().to(handlers::config::update_config_section))
                    .route("/config/reload", web::post().to(handlers::config::reload_config))
                    .route("/config/schema", web::get().to(handlers::config::get_config_schema))
                    // Masking API
                    .route("/masking/config", web::get().to(handlers::masking::get_config))
                    .route("/masking/config", web::post().to(handlers::masking::update_config))
                    .route("/masking/rules", web::get().to(handlers::masking::list_rules))
                    .route("/masking/rules", web::post().to(handlers::masking::create_rule))
                    .route("/masking/rules/{id}", web::put().to(handlers::masking::update_rule))
                    .route("/masking/rules/{id}", web::delete().to(handlers::masking::delete_rule))
                    .route("/masking/test", web::post().to(handlers::masking::test_masking))
                    // Prompts API
                    .route("/prompts", web::get().to(handlers::prompts::list_prompts))
                    .route("/prompts", web::post().to(handlers::prompts::create_prompt))
                    .route("/prompts/{id}", web::put().to(handlers::prompts::update_prompt))
                    .route("/prompts/{id}", web::delete().to(handlers::prompts::delete_prompt))
                    .route("/prompts/{id}/default", web::post().to(handlers::prompts::set_default_prompt))
                    // Memories API
                    .route("/memories", web::get().to(handlers::memories::list_memories))
                    .route("/sessions/{id}/memory", web::get().to(handlers::memories::get_session_memory))
            )
            // OpenAI-compatible API endpoints
            .service(
                web::scope("/v1")
                    .route("/chat/completions", web::post().to(handlers::llm_api::chat_completions))
                    .route("/models", web::get().to(handlers::llm_api::list_models))
                    .route("/models/{model}", web::get().to(handlers::llm_api::get_model))
            )
    })
    .bind(bind_addr)?
    .run()
    .await
}
