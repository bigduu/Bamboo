use clap::{Parser, Subcommand, Args};
use colored::Colorize;
use eventsource_client::Client;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::time::Instant;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bamboo-cli")]
#[command(about = "CLI tool for bamboo")]
#[command(version)]
struct Cli {
    #[arg(long, default_value = "http://localhost:8081")]
    server_url: String,

    #[arg(long)]
    session_id: Option<String>,

    /// Enable debug mode
    #[arg(long, short, default_value = "false")]
    debug: bool,

    /// Config file path
    #[arg(long, env = "BAMBOO_CONFIG", default_value = "~/.bamboo/config.json")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动交互式聊天
    Chat,
    /// 发送单条消息
    Send {
        /// 消息内容
        message: String,
    },
    /// 测试 SSE 流式输出
    Stream {
        /// 消息内容
        message: String,
    },
    /// 查看会话历史
    History,
    /// 配置管理命令
    Config(ConfigArgs),
}

#[derive(Args, Clone)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommands,
}

#[derive(Subcommand, Clone)]
enum ConfigCommands {
    /// 获取配置值
    Get {
        /// 配置键 (如: server.port, llm.default_provider)
        key: String,
    },
    /// 设置配置值
    Set {
        /// 配置键 (如: server.port, llm.default_provider)
        key: String,
        /// 配置值
        value: String,
    },
    /// 初始化默认配置
    Init {
        /// 强制覆盖已有配置
        #[arg(long, default_value = "false")]
        force: bool,
    },
    /// 显示当前配置
    Show,
    /// 查看服务器配置
    Server,
}

#[derive(Serialize)]
struct ChatRequest {
    message: String,
    session_id: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    session_id: String,
    stream_url: String,
    #[allow(dead_code)]
    status: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AgentEvent {
    Token { content: String },
    ToolStart { #[allow(dead_code)] tool_call_id: String, tool_name: String, arguments: serde_json::Value },
    ToolComplete { #[allow(dead_code)] tool_call_id: String, result: ToolResult },
    ToolError { #[allow(dead_code)] tool_call_id: String, error: String },
    Complete { usage: TokenUsage },
    Error { message: String },
}

#[derive(Deserialize, Debug)]
struct ToolResult {
    #[allow(dead_code)]
    success: bool,
    result: String,
}

#[derive(Deserialize, Debug)]
struct TokenUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        eprintln!("{}", "[DEBUG] Debug mode enabled".dimmed());
        eprintln!("{}", format!("[DEBUG] Server URL: {}", cli.server_url).dimmed());
    }

    match cli.command {
        Commands::Chat => run_interactive_chat(&cli.server_url, cli.session_id, cli.debug).await,
        Commands::Send { message } => {
            send_message(&cli.server_url, cli.session_id, &message, cli.debug).await
        }
        Commands::Stream { message } => {
            stream_message(&cli.server_url, cli.session_id, &message, cli.debug).await
        }
        Commands::History => get_history(&cli.server_url, cli.session_id, cli.debug).await,
        Commands::Config(args) => handle_config(args, &cli.config, cli.debug).await,
    }
}

async fn handle_config(args: ConfigArgs, config_path: &str, debug: bool) -> anyhow::Result<()> {
    use bamboo_config::{Config, ConfigManager};

    // 展开配置文件路径
    let config_path = bamboo_config::expand_tilde(config_path)
        .unwrap_or_else(|| PathBuf::from(config_path));

    if debug {
        eprintln!("{}", format!("[DEBUG] Config path: {:?}", config_path).dimmed());
    }

    match args.command {
        ConfigCommands::Get { key } => {
            let manager = ConfigManager::load(&config_path).await?;
            let config = manager.get().read().await.clone();
            
            match config.get_value(&key) {
                Some(value) => {
                    println!("{}", format!("{} = {}", key, value).green());
                }
                None => {
                    println!("{}", format!("❌ Key not found: {}", key).red());
                    std::process::exit(1);
                }
            }
        }
        ConfigCommands::Set { key, value } => {
            let manager = ConfigManager::load(&config_path).await?;
            
            manager.update(|config| {
                if let Err(e) = config.set_value(&key, &value) {
                    eprintln!("{}", format!("❌ Failed to set value: {}", e).red());
                    std::process::exit(1);
                }
            }).await?;
            
            manager.save().await?;
            println!("{}", format!("✅ Set {} = {}", key, value).green());
        }
        ConfigCommands::Init { force } => {
            if config_path.exists() && !force {
                println!("{}", format!("⚠️  Config already exists at {:?}", config_path).yellow());
                println!("{}", "Use --force to overwrite".dimmed());
                return Ok(());
            }

            // 初始化目录
            bamboo_config::init_bamboo_dirs().await?;
            
            // 创建默认配置
            let default_config = Config::default();
            let manager = ConfigManager::new(default_config, config_path.clone());
            manager.save().await?;
            
            println!("{}", format!("✅ Config initialized at {:?}", config_path).green());
            println!("{}", "You can edit this file to customize your settings".dimmed());
        }
        ConfigCommands::Show => {
            let manager = ConfigManager::load(&config_path).await?;
            let config = manager.get().read().await.clone();
            
            println!("{}", "📋 Current Configuration:".cyan().bold());
            println!();
            
            // 显示为 JSON
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
        }
        ConfigCommands::Server => {
            // 从服务器获取配置
            let client = reqwest::Client::new();
            
            // 使用 server_url
            let cli = Cli::parse();
            let url = format!("{}/api/v1/config", cli.server_url);
            
            if debug {
                eprintln!("{}", format!("[DEBUG] GET {}", url).dimmed());
            }
            
            let response = client.get(&url).send().await?;
            
            if response.status().is_success() {
                let config: serde_json::Value = response.json().await?;
                println!("{}", "📋 Server Configuration:".cyan().bold());
                println!();
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("{}", format!("❌ Error: {}", response.status()).red());
                let text = response.text().await?;
                if debug {
                    eprintln!("{}", format!("[DEBUG] Error body: {}", text).dimmed());
                }
            }
        }
    }

    Ok(())
}

async fn send_message(
    server_url: &str,
    session_id: Option<String>,
    message: &str,
    debug: bool,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let request = ChatRequest {
        message: message.to_string(),
        session_id: session_id.clone(),
        model: None,
    };

    let url = format!("{}/api/v1/chat", server_url);
    
    if debug {
        eprintln!("{}", format!("[DEBUG] POST {}", url).dimmed());
        eprintln!("{}", format!("[DEBUG] Request body: {}", serde_json::to_string(&request)?).dimmed());
    }

    println!("{}", format!("🚀 Sending message: {}", message).cyan());
    
    let start = Instant::now();
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await?;
    let elapsed = start.elapsed();

    if debug {
        eprintln!("{}", format!("[DEBUG] Response: {} in {:?}", response.status(), elapsed).dimmed());
        eprintln!("{}", format!("[DEBUG] Response headers: {:?}", response.headers()).dimmed());
    }

    if response.status().is_success() {
        let chat_response: ChatResponse = response.json().await?;
        println!("{}", format!("✅ Session ID: {}", chat_response.session_id).green());
        println!("{}", format!("📡 Stream URL: {}", chat_response.stream_url).green());
        
        if debug {
            eprintln!("{}", format!("[DEBUG] Full response: {:?}", chat_response).dimmed());
        }
        
        // 尝试读取流
        let stream_url = format!("{}{}", server_url, chat_response.stream_url);
        if debug {
            eprintln!("{}", format!("[DEBUG] Connecting to stream: {}", stream_url).dimmed());
        }
        
        let stream_response = client
            .get(&stream_url)
            .send()
            .await?;
        
        if debug {
            eprintln!("{}", format!("[DEBUG] Stream response: {}", stream_response.status()).dimmed());
        }
        
        if stream_response.status().is_success() {
            let body = stream_response.text().await?;
            println!("{}", format!("📦 Response: {}", body).yellow());
        }
    } else {
        println!("{}", format!("❌ Error: {}", response.status()).red());
        let text = response.text().await?;
        if debug {
            eprintln!("{}", format!("[DEBUG] Error body: {}", text).dimmed());
        }
        println!("{}", text.red());
    }

    Ok(())
}

async fn stream_message(
    server_url: &str,
    session_id: Option<String>,
    message: &str,
    debug: bool,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let request = ChatRequest {
        message: message.to_string(),
        session_id: Some(session_id.clone()),
        model: None,
    };

    let url = format!("{}/api/v1/chat", server_url);
    
    if debug {
        eprintln!("{}", format!("[DEBUG] POST {}", url).dimmed());
        eprintln!("{}", format!("[DEBUG] Session ID: {}", session_id).dimmed());
        eprintln!("{}", format!("[DEBUG] Message: {}", message).dimmed());
    }

    println!("{}", format!("🚀 Starting stream session: {}", session_id).cyan());
    
    let start = Instant::now();
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await?;

    if debug {
        eprintln!("{}", format!("[DEBUG] Chat response: {} in {:?}", 
            response.status(), start.elapsed()).dimmed());
    }

    if !response.status().is_success() {
        println!("{}", format!("❌ Error: {}", response.status()).red());
        return Ok(());
    }

    let chat_response: ChatResponse = response.json().await?;
    
    if debug {
        eprintln!("{}", format!("[DEBUG] Stream URL: {}", chat_response.stream_url).dimmed());
    }

    println!("{}", "📝 Stream output:".cyan());
    println!("{}", "─".repeat(50).dimmed());

    // 使用 SSE 客户端读取流
    let stream_url = format!("{}{}", server_url, chat_response.stream_url);
    
    if debug {
        eprintln!("{}", format!("[DEBUG] Connecting SSE: {}", stream_url).dimmed());
    }
    
    let sse_client = eventsource_client::ClientBuilder::for_url(&stream_url)?
        .build();

    let mut stream = sse_client.stream();
    let mut content_buffer = String::new();
    let mut event_count = 0;
    let stream_start = Instant::now();

    while let Some(event) = stream.next().await {
        match event {
            Ok(eventsource_client::SSE::Event(event)) => {
                event_count += 1;
                
                if debug {
                    eprintln!("{}", format!("[DEBUG] Raw event {}: {}", 
                        event_count, event.data).dimmed());
                }
                
                if let Ok(agent_event) = serde_json::from_str::<AgentEvent>(&event.data) {
                    match &agent_event {
                        AgentEvent::Token { content } => {
                            print!("{}", content.green());
                            io::stdout().flush()?;
                            content_buffer.push_str(content);
                        }
                        AgentEvent::ToolStart { tool_name, arguments, .. } => {
                            println!();
                            println!("{}", format!("🔧 Executing tool: {}", tool_name).yellow());
                            println!("{}", format!("   Args: {}", arguments).dimmed());
                        }
                        AgentEvent::ToolComplete { result, .. } => {
                            println!("{}", format!("✅ Tool result: {}", result.result).green());
                        }
                        AgentEvent::ToolError { error, .. } => {
                            println!("{}", format!("❌ Tool error: {}", error).red());
                        }
                        AgentEvent::Complete { usage } => {
                            println!();
                            println!(
                                "{}",
                                format!(
                                    "📊 Tokens: prompt={}, completion={}, total={}",
                                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                                )
                                .dimmed()
                            );
                        }
                        AgentEvent::Error { message } => {
                            println!();
                            println!("{}", format!("❌ Error: {}", message).red());
                        }
                    }
                } else if debug {
                    eprintln!("{}", format!("[DEBUG] Failed to parse event: {}", event.data).dimmed());
                }
            }
            Ok(eventsource_client::SSE::Comment(comment)) => {
                if debug {
                    eprintln!("{}", format!("[DEBUG] SSE Comment: {}", comment).dimmed());
                }
            }
            Err(e) => {
                if debug {
                    eprintln!("{}", format!("[DEBUG] SSE Error: {:?}", e).dimmed());
                }
                eprintln!("\n{}: {:?}", "SSE Error".red(), e);
                break;
            }
        }
    }

    let stream_duration = stream_start.elapsed();
    
    if debug {
        eprintln!("{}", format!("[DEBUG] Stream completed: {} events in {:?}", 
            event_count, stream_duration).dimmed());
    }

    println!();
    println!("{}", "─".repeat(50).dimmed());
    println!("{}", "✨ Stream complete".cyan());
    
    if !content_buffer.is_empty() {
        println!();
        println!("{}", "📝 Complete response:".cyan());
        println!("{}", content_buffer);
    }

    Ok(())
}

async fn run_interactive_chat(
    server_url: &str,
    session_id: Option<String>,
    debug: bool,
) -> anyhow::Result<()> {
    let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    println!("{}", "🤖 Bamboo Agent Interactive Chat".cyan().bold());
    println!("{}", format!("Session ID: {}", session_id).dimmed());
    println!("{}", "Type 'exit' or 'quit' to leave".dimmed());
    
    if debug {
        eprintln!("{}", format!("[DEBUG] Server URL: {}", server_url).dimmed());
        eprintln!("{}", "[DEBUG] Debug mode enabled".dimmed());
    }
    
    println!();

    loop {
        print!("{} ", "You:".cyan().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("{}", "👋 Goodbye!".cyan());
            break;
        }

        if input.is_empty() {
            continue;
        }

        println!("{}", "Assistant:".green().bold());
        
        if let Err(e) = stream_message(server_url, Some(session_id.clone()), input, debug).await {
            if debug {
                eprintln!("{}", format!("[DEBUG] Error: {:?}", e).dimmed());
            }
            println!("{}", format!("❌ Error: {}", e).red());
        }
        
        println!();
    }

    Ok(())
}

async fn get_history(server_url: &str, session_id: Option<String>, debug: bool) -> anyhow::Result<()> {
    let session_id = match session_id {
        Some(id) => id,
        None => {
            println!("{}", "❌ Please provide --session-id".red());
            return Ok(());
        }
    };

    let url = format!("{}/api/v1/history/{}", server_url, session_id);
    
    if debug {
        eprintln!("{}", format!("[DEBUG] GET {}", url).dimmed());
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await?;

    if debug {
        eprintln!("{}", format!("[DEBUG] Response: {}", response.status()).dimmed());
    }

    if response.status().is_success() {
        let history: serde_json::Value = response.json().await?;
        
        if debug {
            eprintln!("{}", "[DEBUG] Raw response:".dimmed());
        }
        
        println!("{}", serde_json::to_string_pretty(&history)?);
    } else {
        println!("{}", format!("❌ Error: {}", response.status()).red());
        let text = response.text().await?;
        if debug {
            eprintln!("{}", format!("[DEBUG] Error body: {}", text).dimmed());
        }
    }

    Ok(())
}
