pub mod handlers;
pub mod message;
pub mod router;

pub use message::{Message, MessageKind, MessageMetadata, MessagePayload};
pub use router::{MessageBus, Topics, MessageRouter, SmartRouter, MessageHandler};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BambooError {
    #[error("Router error: {0}")]
    Router(String),
    
    #[error("Handler error: {0}")]
    Handler(String),
    
    #[error("Gateway error: {0}")]
    Gateway(String),
    
    #[error("Agent error: {0}")]
    Agent(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Channel closed")]
    ChannelClosed,
}

pub type Result<T> = std::result::Result<T, BambooError>;
