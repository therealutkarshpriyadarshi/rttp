//! TCP server and connection management
//!
//! This module provides the core HTTP server implementation.

use crate::context::Context;
use crate::http::{Request, Response};
use crate::middleware::MiddlewareStack;
use crate::router::Router;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// Maximum size for request buffer (1MB)
const MAX_REQUEST_SIZE: usize = 1024 * 1024;

/// HTTP server
pub struct Server {
    addr: String,
    router: Arc<Router>,
    middleware: Arc<MiddlewareStack>,
}

impl Server {
    /// Create a new server instance
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            router: Arc::new(Router::new()),
            middleware: Arc::new(MiddlewareStack::new()),
        }
    }

    /// Create a server with a router and middleware stack
    pub fn with_router_and_middleware(
        addr: impl Into<String>,
        router: Router,
        middleware: MiddlewareStack,
    ) -> Self {
        Self {
            addr: addr.into(),
            router: Arc::new(router),
            middleware: Arc::new(middleware),
        }
    }

    /// Bind the server to the configured address
    pub async fn bind(addr: impl Into<String>) -> Result<Self, std::io::Error> {
        Ok(Self::new(addr))
    }

    /// Start the server and listen for connections
    pub async fn run(self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("Server listening on {}", self.addr);

        let router = self.router;
        let middleware = self.middleware;

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("Accepted connection from {}", addr);
                    let router = router.clone();
                    let middleware = middleware.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, router, middleware).await {
                            error!("Error handling connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single TCP connection
async fn handle_connection(
    mut stream: TcpStream,
    router: Arc<Router>,
    middleware: Arc<MiddlewareStack>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; MAX_REQUEST_SIZE];
    let mut total_read = 0;

    // Read data from the stream
    loop {
        let n = stream.read(&mut buffer[total_read..]).await?;
        if n == 0 {
            // Connection closed by client
            if total_read == 0 {
                debug!("Client closed connection before sending data");
                return Ok(());
            }
            break;
        }

        total_read += n;

        // Try to parse the request
        match Request::parse(&buffer[..total_read]) {
            Ok(Some((request, _bytes_consumed))) => {
                debug!(
                    "Parsed request: {} {} {:?}",
                    request.method().as_str(),
                    request.uri(),
                    request.version()
                );

                // Create context
                let ctx = Context::new(request);

                // Handle the request through middleware and router
                let response = if middleware.is_empty() {
                    // No middleware, just route
                    router.route(ctx.into_request()).await
                } else {
                    // Execute middleware stack with router as final handler
                    let router = router.clone();
                    middleware
                        .execute(ctx, move |ctx| {
                            let router = router.clone();
                            async move { router.route(ctx.into_request()).await }
                        })
                        .await
                };

                // Send the response
                let response_bytes = response.to_bytes();
                stream.write_all(&response_bytes).await?;
                stream.flush().await?;

                debug!("Response sent successfully");
                break;
            }
            Ok(None) => {
                // Incomplete request, continue reading
                if total_read >= MAX_REQUEST_SIZE {
                    warn!("Request size exceeded maximum limit");
                    let response = Response::new(crate::http::StatusCode::BadRequest)
                        .with_body(b"Request too large".to_vec());
                    stream.write_all(&response.to_bytes()).await?;
                    return Ok(());
                }
                continue;
            }
            Err(e) => {
                warn!("Failed to parse request: {}", e);
                let response = Response::new(crate::http::StatusCode::BadRequest)
                    .with_body(format!("Bad request: {}", e).into_bytes());
                stream.write_all(&response.to_bytes()).await?;
                return Ok(());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = Server::new("127.0.0.1:8080");
        assert_eq!(server.addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_server_with_router_and_middleware() {
        let router = Router::new();
        let middleware = MiddlewareStack::new();
        let server = Server::with_router_and_middleware("127.0.0.1:8080", router, middleware);
        assert_eq!(server.addr, "127.0.0.1:8080");
    }
}
