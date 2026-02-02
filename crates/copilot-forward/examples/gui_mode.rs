//! GUI Mode Example
//! 
//! Run with: cargo run --example gui_mode --features gui

use copilot_forward::{CopilotClient, UiConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Copilot client in GUI mode...\n");
    
    // Create client with full GUI support
    let client = CopilotClient::new(UiConfig::gui()).await?;
    
    println!("✅ Client authenticated successfully!");
    println!("Token: {:?}", client.token());
    
    // List available models
    let models = client.models().await?;
    println!("\nAvailable models:");
    for model in models {
        println!("  - {}", model.id);
    }
    
    Ok(())
}
