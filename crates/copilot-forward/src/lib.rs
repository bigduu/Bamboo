//! # copilot-forward
//! 
//! GitHub Copilot forwarding client with configurable UI behavior.
//! 
//! ## Features
//! 
//! - **Device Code authentication** - Complete OAuth flow
//! - **Token caching** - Automatic token persistence
//! - **HTTP forwarding** - Direct API access to Copilot
//! - **Configurable UI** - GUI, headless, or silent modes
//! 
//! ## Quick Start
//! 
//! ```no_run
//! use copilot_forward::{CopilotClient, UiConfig};
//! 
//! # async fn example() -> anyhow::Result<()> {
//! // Create client with GUI mode
//! let client = CopilotClient::new(UiConfig::gui()).await?;
//! 
//! // Use chat completions
//! let response = client.chat_completions(&request).await?;
//! # Ok(())
//! # }
//! ```
//! 
//! ## UI Modes
//! 
//! - `UiConfig::gui()` - Full GUI (browser, clipboard, dialogs)
//! - `UiConfig::headless()` - Console output only
//! - `UiConfig::silent()` - No output
//! - `UiConfig::custom()` - Builder pattern

pub mod auth;
pub mod cache;
pub mod forward;
pub mod ui;

pub mod client;

// Re-export main types for convenience
pub use client::{CopilotClient, ClientError};
pub use ui::UiConfig;
pub use auth::{CopilotToken, AuthError};
pub use cache::TokenCache;
pub use forward::{ForwardClient, ChatCompletionRequest, Message, Model, ForwardError};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
