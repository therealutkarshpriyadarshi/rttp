//! Hello World example
//!
//! This example demonstrates basic server setup and logging initialization.
//! Run with: cargo run --example hello_world

use pttp::prelude::*;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    info!("🚀 Starting PTTP Hello World example");
    info!("📚 PTTP Version: {}", pttp::VERSION);

    // Create and bind server
    let addr = "127.0.0.1:8080";
    info!("🌐 Binding server to {}", addr);

    let server = Server::bind(addr).await?;
    info!("✅ Server successfully bound to {}", addr);
    info!("🎯 Ready to accept connections!");
    info!("📝 Note: Phase 1 implementation pending - server will accept connections but not handle requests yet");

    // Run server (this will loop indefinitely)
    server.run().await?;

    Ok(())
}
