//! TCP server and connection management
//!
//! This module provides the core HTTP server implementation.

use crate::http::{Request, Response};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// Maximum size for request buffer (1MB)
const MAX_REQUEST_SIZE: usize = 1024 * 1024;

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
                Ok((stream, addr)) => {
                    debug!("Accepted connection from {}", addr);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream).await {
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
async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
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

                // Handle the request and generate a response
                let response = handle_request(request);

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

/// Handle a parsed HTTP request and generate a response
fn handle_request(request: Request) -> Response {
    // For now, just return a simple response based on the path
    match request.uri() {
        "/" => Response::html(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>PTTP - Pure Rust Web Framework</title>
</head>
<body>
    <h1>🦀 Welcome to PTTP!</h1>
    <p>A Pure Rust Web Framework with AI/LLM Integration</p>
    <p><strong>Phase 1: HTTP Server Core - COMPLETE!</strong></p>
    <ul>
        <li>✅ TCP listener and connection handling</li>
        <li>✅ HTTP/1.1 request parsing</li>
        <li>✅ Request/Response abstractions</li>
        <li>✅ Basic request handling</li>
    </ul>
</body>
</html>
"#,
        ),
        "/health" => Response::json(&serde_json::json!({
            "status": "ok",
            "version": crate::VERSION,
            "phase": "1"
        }))
        .unwrap_or_else(|_| Response::internal_error()),
        "/echo" => {
            let body_text = String::from_utf8_lossy(request.body());
            Response::text(format!(
                "Method: {}\nURI: {}\nBody: {}",
                request.method().as_str(),
                request.uri(),
                body_text
            ))
        }
        _ => Response::not_found().with_body(b"404 - Not Found".to_vec()),
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
