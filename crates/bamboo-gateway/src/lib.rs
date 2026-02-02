//! Bamboo Gateway - WebSocket control plane for real-time sessions
//!
//! This crate provides a WebSocket server for managing multi-sessions and
//! real-time message push, similar to OpenClaw's control plane.

mod connection;
mod gateway;
mod protocol;
mod router;
mod session;

pub use connection::{ConnectionHandle, ConnectionPool};
pub use gateway::{Gateway, GatewayConfig, GatewayError};
pub use protocol::{ClientMessage, GatewayEvent, TokenUsage};
pub use router::{IncomingMessage, MessageRouter, RouteResult};
pub use session::{Session, SessionHandle, SessionManager, SessionError};
