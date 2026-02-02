//! Authentication module for GitHub Copilot
//!
//! Provides Device Code flow authentication with configurable UI.

pub mod device_code;
pub mod token;

pub use device_code::{get_device_code, present_device_code, DeviceCodeResponse, AuthError};
pub use token::{authenticate, poll_access_token, get_copilot_token, CopilotToken};
