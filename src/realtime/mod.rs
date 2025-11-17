//! WebSocket and Server-Sent Events
//!
//! This module provides real-time communication support through:
//! - WebSocket protocol (RFC 6455)
//! - Server-Sent Events (SSE)
//!
//! ## WebSocket
//!
//! WebSocket provides full-duplex communication channels over a single TCP connection.
//!
//! ```no_run
//! use pttp::realtime::websocket::{WebSocket, Message};
//!
//! async fn handle_websocket(ws: WebSocket) {
//!     while let Some(msg) = ws.recv().await {
//!         match msg {
//!             Ok(Message::Text(text)) => {
//!                 println!("Received: {}", text);
//!                 ws.send(Message::Text(format!("Echo: {}", text))).await.ok();
//!             }
//!             Ok(Message::Close) => break,
//!             _ => {}
//!         }
//!     }
//! }
//! ```
//!
//! ## Server-Sent Events
//!
//! SSE provides server-to-client event streaming over HTTP.
//!
//! ```no_run
//! use pttp::realtime::sse::{SseStream, Event};
//!
//! async fn handle_sse() -> SseStream {
//!     let stream = SseStream::new();
//!     stream.send(Event::new("message").data("Hello from SSE!")).await.ok();
//!     stream
//! }
//! ```

pub mod sse;
pub mod websocket;

pub use sse::{Event, SseStream};
pub use websocket::{Message, WebSocket, WebSocketUpgrade};
