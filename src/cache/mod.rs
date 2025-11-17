//! Caching layer (in-memory and Redis)
//!
//! This module provides comprehensive caching solutions for the PTTP framework:
//!
//! - **LRU Cache**: In-memory cache with LRU eviction and TTL support
//! - **Redis Client**: Full-featured Redis client with RESP protocol implementation
//! - **Compression**: HTTP response compression middleware (Gzip and Brotli)
//!
//! # Examples
//!
//! ## Using LRU Cache
//!
//! ```
//! use pttp::cache::LruCache;
//! use std::time::Duration;
//!
//! let cache = LruCache::new(100);
//! cache.insert("key".to_string(), "value".to_string(), Some(Duration::from_secs(60)));
//!
//! if let Some(value) = cache.get(&"key".to_string()) {
//!     println!("Found: {}", value);
//! }
//! ```
//!
//! ## Using Redis Client
//!
//! ```no_run
//! use pttp::cache::RedisClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RedisClient::connect("127.0.0.1:6379").await?;
//!     client.set("key", b"value", None).await?;
//!     let value = client.get("key").await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Using Compression Middleware
//!
//! ```no_run
//! use pttp::cache::Compression;
//! use pttp::middleware::MiddlewareStack;
//!
//! let mut stack = MiddlewareStack::new();
//! stack.add(Compression::new(6).with_min_size(1024));
//! ```

mod compression;
mod memory;
mod redis;

pub use compression::{Algorithm, Compression};
pub use memory::LruCache;
pub use redis::{RedisClient, RedisError, RespValue};
