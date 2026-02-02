//! Example: Using OpenAI Provider with different authentication methods
//! 
//! This example shows how to use the unified OpenAiProvider with:
//! - API Key authentication (standard OpenAI)
//! - Bearer token authentication
//! - Device Code authentication (GitHub Copilot)

use bamboo_llm::{OpenAiProvider, ProviderConfig, AuthConfig, LLMProvider};
use bamboo_core::chat::{ChatRequest, ChatMessage, MessageRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // =========================================================================
    // Example 1: Standard OpenAI with API Key
    // =========================================================================
    println!("=== Example 1: OpenAI with API Key ===");
    
    let openai_config = ProviderConfig::new("openai", "https://api.openai.com/v1")
        .with_api_key(std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "your-api-key".to_string()))
        .with_model("gpt-4o-mini");
    
    let openai = OpenAiProvider::with_config(openai_config).await?;
    println!("OpenAI provider created: {}", openai.provider_id());
    
    // =========================================================================
    // Example 2: GitHub Copilot with Device Code authentication
    // =========================================================================
    println!("\n=== Example 2: GitHub Copilot with Device Code ===");
    
    // Method 1: Use the convenient copilot() constructor
    let copilot = OpenAiProvider::copilot().await?;
    println!("Copilot provider created: {}", copilot.provider_id());
    
    // Method 2: Use custom configuration
    let custom_headers = [
        ("editor-version".to_string(), "vscode/1.99.2".to_string()),
        ("editor-plugin-version".to_string(), "copilot-chat/0.20.3".to_string()),
        ("user-agent".to_string(), "GitHubCopilotChat/0.20.3".to_string()),
    ].into_iter().collect();
    
    let copilot_config = ProviderConfig::new("copilot", "https://api.githubcopilot.com")
        .with_model("copilot-chat")
        .with_device_code("Iv1.b507a08c87ecfe98")
        .with_headers(custom_headers);
    
    let copilot_custom = OpenAiProvider::with_config(copilot_config).await?;
    println!("Custom Copilot provider created: {}", copilot_custom.provider_id());
    
    // =========================================================================
    // Example 3: Moonshot or other OpenAI-compatible providers
    // =========================================================================
    println!("\n=== Example 3: Moonshot (OpenAI-compatible) ===");
    
    let moonshot_config = ProviderConfig::new("moonshot", "https://api.moonshot.cn/v1")
        .with_api_key(std::env::var("MOONSHOT_API_KEY").unwrap_or_else(|_| "your-api-key".to_string()))
        .with_model("moonshot-v1-8k");
    
    let moonshot = OpenAiProvider::with_config(moonshot_config).await?;
    println!("Moonshot provider created: {}", moonshot.provider_id());
    
    // =========================================================================
    // Example 4: Using AuthConfig enum directly
    // =========================================================================
    println!("\n=== Example 4: Using AuthConfig enum ===");
    
    // API Key auth
    let api_key_auth = AuthConfig::ApiKey {
        key: "sk-...".to_string(),
    };
    println!("API Key auth config: {:?}", api_key_auth);
    
    // Bearer auth
    let bearer_auth = AuthConfig::Bearer {
        token: "token-...".to_string(),
    };
    println!("Bearer auth config: {:?}", bearer_auth);
    
    // Device Code auth (for Copilot)
    let device_code_auth = AuthConfig::DeviceCode {
        client_id: "Iv1.b507a08c87ecfe98".to_string(),
        device_code_url: "https://github.com/login/device/code".to_string(),
        access_token_url: "https://github.com/login/oauth/access_token".to_string(),
        copilot_token_url: "https://api.github.com/copilot_internal/v2/token".to_string(),
    };
    println!("Device Code auth config: {:?}", device_code_auth);
    
    // =========================================================================
    // Example 5: Creating a chat request
    // =========================================================================
    println!("\n=== Example 5: Creating a chat request ===");
    
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant.".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Hello, how are you?".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        options: bamboo_core::chat::ChatOptions {
            model: "gpt-4o-mini".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: false,
            tools: None,
            tool_choice: None,
        },
    };
    
    println!("Chat request created with {} messages", request.messages.len());
    
    // Example of making a request (commented out as it requires valid auth)
    // let response = openai.chat(request).await?;
    // println!("Response: {:?}", response);
    
    println!("\n✅ All examples completed successfully!");
    
    Ok(())
}
