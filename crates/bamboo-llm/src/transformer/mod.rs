pub mod openai;
pub mod error;
pub mod utils;

pub use openai::OpenAiTransformer;
pub use error::ConversionError as TransformError;

use async_trait::async_trait;
use bamboo_core::chat::{ChatRequest, ChatChunk};
use serde_json::Value;
use std::pin::Pin;
use futures::Stream;

use crate::error::ConversionError;

/// Type alias for LLM stream
pub type LLMStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, crate::LLMError>> + Send>>;

/// Schema transformer trait for converting between internal and provider formats
#[async_trait]
pub trait SchemaTransformer: Send + Sync {
    /// Get the provider ID
    fn provider_id(&self) -> &str;
    
    /// Transform request to provider-specific format
    fn transform_request(&self, request: &ChatRequest) -> Result<Value, ConversionError>;
    
    /// Parse a stream chunk from provider
    fn parse_stream_chunk(&self, data: &str) -> Result<Option<ChatChunk>, ConversionError>;
    
    /// Transform tool definitions to provider format
    fn transform_tools(&self, tools: &[bamboo_core::types::ToolDefinition]) -> Result<Value, ConversionError>;
    
    /// Parse a complete response (non-streaming)
    fn parse_response(&self, data: &Value) -> Result<bamboo_core::chat::ChatResponse, ConversionError>;
}

/// Transformer registry for looking up transformers by provider ID
pub struct TransformerRegistry {
    transformers: std::collections::HashMap<String, Box<dyn SchemaTransformer>>,
}

impl TransformerRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            transformers: std::collections::HashMap::new(),
        }
    }
    
    /// Register a transformer
    pub fn register(&mut self, transformer: Box<dyn SchemaTransformer>) {
        self.transformers.insert(transformer.provider_id().to_string(), transformer);
    }
    
    /// Get a transformer by provider ID
    pub fn get(&self, provider_id: &str) -> Option<&dyn SchemaTransformer> {
        self.transformers.get(provider_id).map(|t| t.as_ref())
    }
}

impl Default for TransformerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
