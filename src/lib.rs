//! # PTTP - Pure Rust Web Framework with AI/LLM Integration
//!
//! A production-grade web framework built from near-scratch in Rust to maximize
//! learning about systems programming, async I/O, networking, and AI integration.
//!
//! ## Philosophy
//!
//! Build core components from scratch to understand fundamentals, using only
//! essential libraries where complexity is too high.
//!
//! ## Architecture
//!
//! - **http**: HTTP protocol parsing and types
//! - **server**: TCP server and connection management
//! - **router**: Request routing and path matching
//! - **middleware**: Middleware system and built-in middleware
//! - **context**: Request context and type-safe data storage
//! - **database**: Database layer with connection pooling and ORM
//! - **security**: Authentication, authorization, and security features
//! - **cache**: Caching layer (in-memory and Redis)
//! - **realtime**: WebSocket and Server-Sent Events
//! - **background**: Task queue and scheduler
//! - **llm**: AI/LLM integration and RAG pipeline

// Module declarations
pub mod background;
pub mod cache;
pub mod context;
pub mod database;
pub mod http;
pub mod llm;
pub mod middleware;
pub mod realtime;
pub mod router;
pub mod security;
pub mod server;

// Re-export commonly used types
pub use http::{Request, Response};
pub use server::Server;

/// Current version of PTTP
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::context::Context;
    pub use crate::http::{Method, Request, Response, StatusCode};
    pub use crate::middleware::Middleware;
    pub use crate::router::Router;
    pub use crate::server::Server;
}
