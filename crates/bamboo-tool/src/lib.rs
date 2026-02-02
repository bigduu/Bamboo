//! bamboo-tool - Tool execution and management system for Bamboo
//!
//! This crate provides:
//! - Tool definition structures
//! - Script execution (Shell, Python, Node)
//! - Tool registry for managing available tools
//! - Parameter validation and injection

pub mod error;
pub mod executor;
pub mod registry;
pub mod types;

pub use error::{ToolError, Result};
pub use executor::{ToolExecutor, ToolRunner};
pub use registry::ToolRegistry;
pub use types::{ArgDef, ToolDef, ToolRequest, ToolResult, ToolType};

/// Re-export async_trait for implementers
pub use async_trait::async_trait;
