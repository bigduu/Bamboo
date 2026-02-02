//! Device Code authentication flow for GitHub Copilot

use reqwest::Client;
use serde::Deserialize;
use crate::ui::UiConfig;

const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";

/// Device code response from GitHub
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    #[serde(rename = "verification_uri")]
    pub verification_uri: String,
    #[serde(rename = "expires_in")]
    pub expires_in: u64,
    pub interval: u64,
}

/// Get device code from GitHub
pub async fn get_device_code(client: &Client) -> Result<DeviceCodeResponse, AuthError> {
    let params = [
        ("client_id", GITHUB_CLIENT_ID),
        ("scope", "read:user"),
    ];
    
    let response = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::Http(e.to_string()))?;
    
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(AuthError::Api(format!("Device code request failed: HTTP {} - {}", status, text)));
    }
    
    let device_code: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| AuthError::Parse(e.to_string()))?;
    
    Ok(device_code)
}

/// Present device code to user based on UI configuration
pub async fn present_device_code(
    device_code: &DeviceCodeResponse,
    ui_config: &UiConfig,
) -> Result<(), AuthError> {
    // Print to console if enabled
    if ui_config.print_console {
        print_device_code_console(device_code);
    }
    
    // Copy to clipboard if enabled
    #[cfg(feature = "gui")]
    if ui_config.copy_to_clipboard {
        copy_to_clipboard(&device_code.user_code)?;
        if ui_config.print_console {
            println!("📋 Code copied to clipboard!");
        }
    }
    
    // Open browser if enabled
    #[cfg(feature = "gui")]
    if ui_config.open_browser {
        open_browser(&device_code.verification_uri)?;
        if ui_config.print_console {
            println!("🌐 Browser opened!");
        }
    }
    
    // Show GUI dialog if enabled
    #[cfg(feature = "gui")]
    if ui_config.use_gui_dialog {
        show_gui_dialog(device_code).await?;
    }
    
    Ok(())
}

/// Print device code to console
fn print_device_code_console(device_code: &DeviceCodeResponse) {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║     🔐 GitHub Copilot Authorization Required              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  1. Open your browser and navigate to:");
    println!("     {}", device_code.verification_uri);
    println!();
    println!("  2. Enter the following code:");
    println!();
    println!("     ┌─────────────────────────┐");
    println!("     │  {:^23} │", device_code.user_code);
    println!("     └─────────────────────────┘");
    println!();
    println!("  3. Click 'Authorize' and wait...");
    println!();
    println!("  ⏳ Waiting for authorization (expires in {} seconds)...", device_code.expires_in);
    println!();
}

/// Copy text to clipboard
#[cfg(feature = "gui")]
fn copy_to_clipboard(text: &str) -> Result<(), AuthError> {
    use arboard::Clipboard;
    
    let mut clipboard = Clipboard::new()
        .map_err(|e| AuthError::Ui(format!("Failed to access clipboard: {}", e)))?;
    
    clipboard.set_text(text)
        .map_err(|e| AuthError::Ui(format!("Failed to copy to clipboard: {}", e)))?;
    
    Ok(())
}

/// Open URL in browser
#[cfg(feature = "gui")]
fn open_browser(url: &str) -> Result<(), AuthError> {
    webbrowser::open(url)
        .map_err(|e| AuthError::Ui(format!("Failed to open browser: {}", e)))?;
    Ok(())
}

/// Show GUI dialog
#[cfg(feature = "gui")]
async fn show_gui_dialog(device_code: &DeviceCodeResponse) -> Result<(), AuthError> {
    // Note: rfd is sync, but we can use it in an async context
    let uri = device_code.verification_uri.clone();
    let code = device_code.user_code.clone();
    
    tokio::task::spawn_blocking(move || {
        rfd::MessageDialog::new()
            .set_title("GitHub Copilot Authorization")
            .set_description(&format!(
                "Please authorize Copilot:\n\n1. Open: {}\n2. Enter code: {}\n3. Click Authorize",
                uri, code
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }).await.map_err(|e| AuthError::Ui(format!("GUI dialog error: {}", e)))?;
    
    Ok(())
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("HTTP error: {0}")]
    Http(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("UI error: {0}")]
    Ui(String),
    
    #[error("Device code expired")]
    Expired,
    
    #[error("Authorization denied")]
    Denied,
}
