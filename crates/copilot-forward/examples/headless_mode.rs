//! Headless Mode Example
//! 
//! Run with: cargo run --example headless_mode --features headless

use copilot_forward::{CopilotClient, UiConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Copilot client in headless mode...\n");
    
    // Create client with console output only (no GUI)
    let client = CopilotClient::new(UiConfig::headless()).await?;
    
    println!("\n✅ Client ready!");
    
    // Check token validity
    match client.check_token().await {
        Ok(_) => println!("Token is valid"),
        Err(e) => println!("Token check failed: {}", e),
    }
    
    Ok(())
}
