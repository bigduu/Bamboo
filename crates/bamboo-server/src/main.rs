use std::io;
use std::sync::Arc;
use clap::Parser;
use bamboo_config::ConfigManager;

mod handlers;
mod server;
mod state;
mod agent_runner;
mod logging;
mod skill_loader;
mod event_bus;

use server::run_server;
use logging::init_logging;
use state::AppState;
use agent_runner::AgentRunner;

#[derive(Parser, Debug, Clone)]
#[command(name = "bamboo-server")]
#[command(about = "Bamboo Agent HTTP Server")]
#[command(version)]
struct Cli {
    /// Enable debug mode
    #[arg(long, env = "DEBUG", default_value = "false")]
    debug: bool,
    
    /// Server port (overrides config)
    #[arg(long, env = "PORT")]
    port: Option<u16>,
    
    /// LLM provider (overrides config)
    #[arg(long, env = "LLM_PROVIDER")]
    provider: Option<String>,
    
    /// LLM API base URL (overrides config)
    #[arg(long, env = "LLM_BASE_URL")]
    llm_base_url: Option<String>,
    
    /// LLM model name (overrides config)
    #[arg(long, env = "LLM_MODEL")]
    model: Option<String>,
    
    /// LLM API key (overrides config)
    #[arg(long, env = "LLM_API_KEY")]
    api_key: Option<String>,
    
    /// Log level (overrides config)
    #[arg(long, env = "RUST_LOG")]
    log_level: Option<String>,
    
    /// Config file path
    #[arg(long, env = "BAMBOO_CONFIG", default_value = "~/.bamboo/config.json")]
    config: String,
    
    /// Disable hot-reload
    #[arg(long, default_value = "false")]
    no_watch: bool,
    
    /// Disable Gateway
    #[arg(long, default_value = "false")]
    no_gateway: bool,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();
    
    // 展开配置文件路径
    let config_path = bamboo_config::expand_tilde(&cli.config)
        .unwrap_or_else(|| std::path::PathBuf::from(&cli.config));
    
    // 初始化 Bamboo 目录结构
    if let Err(e) = bamboo_config::init_bamboo_dirs().await {
        eprintln!("Warning: Failed to init bamboo directories: {}", e);
    }
    
    // 加载配置
    let config_manager = match ConfigManager::load(&config_path).await {
        Ok(cm) => {
            log::info!("Config loaded from {:?}", config_path);
            cm
        }
        Err(e) => {
            eprintln!("Failed to load config from {:?}: {}", config_path, e);
            std::process::exit(1);
        }
    };
    
    // 启动热重载（如果未禁用）
    #[cfg(feature = "hot-reload")]
    if !cli.no_watch {
        let mut manager = config_manager.clone();
        if let Err(e) = manager.watch(|| {
            log::info!("Config hot-reloaded");
        }) {
            log::warn!("Failed to start config watcher: {}", e);
        }
    }
    
    let config = config_manager.get().read().await.clone();
    
    // 确定最终配置值（CLI 参数覆盖配置文件）
    let port = cli.port.unwrap_or(config.server.port);
    let provider = cli.provider.as_ref().unwrap_or(&config.llm.default_provider).clone();
    
    // 获取 provider 配置
    let provider_config = config.llm.providers.get(&provider)
        .cloned()
        .unwrap_or_else(|| {
            log::warn!("Provider '{}' not found in config, using defaults", provider);
            bamboo_config::ProviderSettings {
                enabled: true,
                base_url: "http://localhost:12123/v1".to_string(),
                model: Some("kimi-for-coding".to_string()),
                auth: bamboo_config::AuthSettings::None,
                headers: None,
                timeout_seconds: Some(60),
            }
        });
    
    let llm_base_url = cli.llm_base_url.unwrap_or(provider_config.base_url);
    let model = cli.model.or(provider_config.model).unwrap_or_else(|| "kimi-for-coding".to_string());
    let api_key = cli.api_key.or_else(|| provider_config.auth.get_api_key()).unwrap_or_else(|| "sk-test".to_string());
    
    // 初始化日志
    let log_level = cli.log_level.or_else(|| {
        if cli.debug {
            Some("debug".to_string())
        } else {
            Some(format!("{:?}", config.logging.level).to_lowercase())
        }
    });
    
    if let Some(level) = log_level {
        std::env::set_var("RUST_LOG", &level);
        env_logger::init();
        log::info!("Log level set to: {}", level);
    } else {
        init_logging(cli.debug);
    }
    
    log::info!("Starting Bamboo Server on port {}", port);
    log::info!("LLM Configuration:");
    log::info!("  Provider: {}", provider);
    log::info!("  Base URL: {}", llm_base_url);
    log::info!("  Model: {}", model);
    log::info!("  Skills enabled: {}", config.skills.enabled);
    log::info!("  Agent max rounds: {}", config.agent.max_rounds);
    log::info!("  Gateway enabled: {}", config.gateway.enabled && !cli.no_gateway);
    
    if cli.debug {
        log::debug!("Debug mode enabled");
        log::debug!("Server configuration:");
        log::debug!("  Port: {}", port);
        log::debug!("  Host: {}", config.server.host);
        log::debug!("  CORS: {}", config.server.cors);
        log::debug!("  Storage: {:?}", config.storage.storage_type);
        log::debug!("  Gateway bind: {}", config.gateway.bind);
    }
    
    // 创建共享状态（克隆字符串以避免移动问题）
    let state = Arc::new(AppState::new_with_config(
        config_manager.clone(),
        &provider,
        llm_base_url.clone(),
        model.clone(),
        api_key.clone(),
    ).await);
    
    // 启动 Gateway（如果启用且未被 CLI 禁用）
    if config.gateway.enabled && !cli.no_gateway {
        log::info!("Starting Gateway...");
        
        // 设置 Gateway 消息处理器
        if let Some(ref gateway) = state.gateway {
            let state_for_handler = state.clone();
            gateway.on_message(move |session_id: String, content: String| {
                let state = state_for_handler.clone();
                tokio::spawn(async move {
                    state.handle_gateway_message(session_id, content).await;
                });
            }).await;
            
            // 克隆 gateway 用于启动
            let gateway_clone = gateway.clone();
            let gateway_bind = config.gateway.bind.clone();
            tokio::spawn(async move {
                log::info!("Gateway listening on ws://{}", gateway_bind);
                if let Err(e) = gateway_clone.run().await {
                    log::error!("Gateway error: {}", e);
                }
            });
        }
    }
    
    // 启动 AgentRunner
    log::info!("Starting AgentRunner...");
    let agent_runner = AgentRunner::new(state.clone());
    tokio::spawn(async move {
        agent_runner.run().await;
    });
    
    // 启动 HTTP Server
    log::info!("Starting HTTP Server on {}:{}", config.server.host, port);
    run_server(
        config_manager,
        port,
        &provider,
        llm_base_url,
        model,
        api_key,
    ).await
}
