//! Bamboo Refactor - Usage Examples

// ============================================================================
// Example 1: Basic Chat with bamboo-core
// ============================================================================

use bamboo_core::{Message, ChatRequest, ChatOptions};
use bamboo_core::types::{ToolDefinition, ToolCall};

fn basic_chat_example() {
    // Create messages using the new API
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("What's the weather like?"),
    ];
    
    // Create a chat request
    let request = ChatRequest::new("gpt-4")
        .with_messages(messages)
        .temperature(0.7)
        .max_tokens(500);
    
    println!("Model: {}", request.model);
    println!("Message count: {}", request.messages.len());
}

// ============================================================================
// Example 2: Tool Definitions
// ============================================================================

fn tool_definition_example() {
    use serde_json::json;
    
    let weather_tool = ToolDefinition::new(
        "get_weather",
        "Get the current weather for a location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    );
    
    let request = ChatRequest::new("gpt-4")
        .with_message(Message::user("What's the weather in Beijing?"))
        .with_tool(weather_tool);
    
    println!("Tools: {}", request.tools.len());
}

// ============================================================================
// Example 3: Using bamboo-llm Providers
// ============================================================================

use bamboo_llm::providers::{OpenAiProvider, CopilotProvider};
use bamboo_llm::LLMProvider;

async fn provider_example() -> Result<(), Box<dyn std::error::Error>> {
    // OpenAI Provider
    let openai = OpenAiProvider::new("your-api-key")?;
    
    let request = ChatRequest::new("gpt-4o-mini")
        .with_message(Message::user("Hello!"));
    
    // Non-streaming chat
    let response = openai.chat(request.clone()).await?;
    println!("Response: {}", response.text());
    
    // Streaming chat
    let mut stream = openai.chat_stream(request).await?;
    while let Some(chunk) = stream.next().await {
        match chunk? {
            bamboo_core::ChatChunk::Content { text } => {
                print!("{}", text);
            }
            bamboo_core::ChatChunk::Finish { reason } => {
                println!("\nFinished: {:?}", reason);
            }
            _ => {}
        }
    }
    
    Ok(())
}

// ============================================================================
// Example 4: Copilot Provider with Authentication
// ============================================================================

async fn copilot_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create Copilot provider
    let mut copilot = CopilotProvider::new()?;
    
    // Check if already authenticated
    if !copilot.is_authenticated() {
        // Try silent authentication (cached token)
        if !copilot.try_authenticate_silent().await? {
            // Interactive device code flow
            copilot.authenticate().await?;
        }
    }
    
    let request = ChatRequest::new("copilot-chat")
        .with_message(Message::user("Write a function in Rust"));
    
    let response = copilot.chat(request).await?;
    println!("Response: {}", response.text());
    
    Ok(())
}

// ============================================================================
// Example 5: Multimodal Content
// ============================================================================

use bamboo_core::types::{Content, ContentPart, ImageSource};

fn multimodal_example() {
    // Text-only content
    let text_content = Content::text("Hello!");
    
    // Multimodal content with image
    let multimodal_content = Content::parts(vec![
        ContentPart::text("What's in this image?"),
        ContentPart::image_url("https://example.com/image.png"),
    ]);
    
    // Image from base64
    let base64_image = Content::parts(vec![
        ContentPart::text("Analyze this:"),
        ContentPart::image_base64("iVBORw0KGgo...", "image/png"),
    ]);
    
    let message = Message::from_parts(
        bamboo_core::types::Role::User,
        vec![
            ContentPart::text("Describe this image:"),
            ContentPart::image_url("https://example.com/photo.jpg"),
        ]
    );
}

// ============================================================================
// Example 6: Custom Provider Configuration
// ============================================================================

use bamboo_llm::provider::ProviderConfig;
use std::time::Duration;

fn custom_config_example() {
    let config = ProviderConfig::new("custom", "https://api.custom-ai.com/v1")
        .with_api_key("custom-key")
        .with_model("custom-model")
        .with_timeout(Duration::from_secs(120))
        .with_header("X-Custom-Header", "value");
    
    println!("Base URL: {}", config.base_url);
    println!("Timeout: {:?}", config.timeout);
}

// ============================================================================
// Example 7: Session Management (bamboo-core)
// ============================================================================

use bamboo_core::Session;

fn session_example() {
    let mut session = Session::new("session-123");
    
    // Add messages
    session.add_message(Message::system("You are helpful."));
    session.add_message(Message::user("Hello!"));
    session.add_message(Message::assistant("Hi there!", None));
    
    // Add tool result
    session.add_message(Message::tool_result(
        "call_123",
        "{\"temperature\": 25}"
    ));
    
    println!("Session has {} messages", session.messages.len());
}

// ============================================================================
// Example 8: Streaming with Tool Calls
// ============================================================================

use bamboo_core::ChatChunk;

async fn streaming_with_tools_example(
    provider: &dyn LLMProvider
) -> Result<(), Box<dyn std::error::Error>> {
    let tool = ToolDefinition::new(
        "calculate",
        "Perform a calculation",
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {"type": "string"}
            }
        }),
    );
    
    let request = ChatRequest::new("gpt-4")
        .with_message(Message::user("Calculate 2 + 2"))
        .with_tool(tool)
        .stream();
    
    let mut stream = provider.chat_stream(request).await?;
    
    while let Some(chunk) = stream.next().await {
        match chunk? {
            ChatChunk::Content { text } => {
                print!("{}", text);
            }
            ChatChunk::ToolCallStart { call_id, name } => {
                println!("\n[Tool: {} (id: {})]", name, call_id);
            }
            ChatChunk::ToolCallDelta { call_id, arguments_delta } => {
                print!("{}", arguments_delta);
            }
            ChatChunk::ToolCallEnd { call_id } => {
                println!("\n[Tool call {} completed]", call_id);
            }
            ChatChunk::Finish { reason } => {
                println!("\n[Finished: {:?}]", reason);
            }
            _ => {}
        }
    }
    
    Ok(())
}

fn main() {
    println!("Bamboo Refactor Examples");
    println!("========================\n");
    
    basic_chat_example();
    tool_definition_example();
    multimodal_example();
    custom_config_example();
    session_example();
    
    println!("\nExamples completed!");
}
