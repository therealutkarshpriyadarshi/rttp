//! TCP server and connection management
//!
//! This module provides the core HTTP server implementation.

use tokio::net::TcpListener;
use tracing::{error, info};

/// HTTP server
pub struct Server {
    addr: String,
}

impl Server {
    /// Create a new server instance
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    /// Bind the server to the configured address
    pub async fn bind(addr: impl Into<String>) -> Result<Self, std::io::Error> {
        Ok(Self::new(addr))
    }

    /// Start the server and listen for connections
    pub async fn run(self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("Server listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((_stream, addr)) => {
                    info!("Accepted connection from {}", addr);
                    tokio::spawn(async move {
                        // TODO: Handle connection
                        // This will be implemented in Phase 1
                    });
                },
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = Server::new("127.0.0.1:8080");
        assert_eq!(server.addr, "127.0.0.1:8080");
    }
}
