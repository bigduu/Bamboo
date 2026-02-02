// New modular structure
pub mod types;
pub mod chat;
pub mod agent;
pub mod tools;
pub mod storage;

// Re-export new types
pub use types::{
    Message as NewMessage,
    Role as NewRole,
    MessageId,
    Content,
    ContentPart,
    ImageSource,
    ToolCall,
    ToolDefinition,
    ToolResult,
};

pub use chat::{
    ChatRequest,
    ChatResponse,
    ChatChunk,
    ChatOptions,
    ChatUsage,
    ResponseFormat,
    FinishReason,
};

// Keep old exports for backward compatibility
pub use agent::{AgentLoop, AgentConfig, AgentError};
pub use agent::events::{AgentEvent, TokenUsage};
pub use tools::{ToolExecutor, ToolError};
pub use storage::{Storage, JsonlStorage};

// Deprecated: Old Message type (alias for compatibility)
#[deprecated(since = "0.2.0", note = "Use types::Message instead")]
pub use agent::types::{Session, Message, Role};

// Deprecated: Old Tool types (alias for compatibility)
#[deprecated(since = "0.2.0", note = "Use types::ToolCall, types::ToolDefinition instead")]
pub use tools::types::{ToolCall as OldToolCall, ToolResult as OldToolResult, ToolSchema, FunctionCall, FunctionSchema};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
