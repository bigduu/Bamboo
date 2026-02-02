//! WebSocket Gateway module
//!
//! Provides WebSocket server for real-time communication with clients.
//! This module was migrated from the standalone bamboo-gateway crate.

pub mod connection;
pub mod gateway;
pub mod protocol;
pub mod router;
pub mod session;

pub use connection::{ConnectionHandle, ConnectionPool};
pub use gateway::{Gateway, GatewayConfig, GatewayError};
pub use protocol::{ClientMessage, GatewayEvent, TokenUsage};
pub use router::{IncomingMessage, MessageRouter, RouteResult};
pub use session::{Session, SessionHandle, SessionManager, SessionError};
