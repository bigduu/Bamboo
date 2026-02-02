//! Core types for tool definitions and requests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;

/// Definition of an argument for a tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArgDef {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: ArgType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Type of argument
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

impl std::fmt::Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::String => write!(f, "string"),
            ArgType::Number => write!(f, "number"),
            ArgType::Boolean => write!(f, "boolean"),
            ArgType::Array => write!(f, "array"),
            ArgType::Object => write!(f, "object"),
        }
    }
}

impl ArgType {
    /// Check if a JSON value matches this type
    pub fn matches(&self, value: &Value) -> bool {
        match self {
            ArgType::String => value.is_string(),
            ArgType::Number => value.is_number(),
            ArgType::Boolean => value.is_boolean(),
            ArgType::Array => value.is_array(),
            ArgType::Object => value.is_object(),
        }
    }
}

/// Definition of a tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<ArgDef>,
}

/// Type of tool based on file extension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Shell,
    Python,
    Node,
}

impl ToolType {
    /// Detect tool type from file extension
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "sh" | "bash" | "zsh" => Some(ToolType::Shell),
                "py" => Some(ToolType::Python),
                "js" | "mjs" | "cjs" => Some(ToolType::Node),
                _ => None,
            })
    }

    /// Get the interpreter command for this tool type
    pub fn interpreter(&self) -> &'static str {
        match self {
            ToolType::Shell => "sh",
            ToolType::Python => "python3",
            ToolType::Node => "node",
        }
    }
}

/// Request to execute a tool
#[derive(Debug, Clone)]
pub struct ToolRequest {
    pub name: String,
    pub arguments: HashMap<String, Value>,
}

/// Result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(output: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
        }
    }

    /// Create a failed result
    pub fn failure(error: String, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
            duration_ms,
        }
    }
}
