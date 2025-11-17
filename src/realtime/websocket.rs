//! WebSocket implementation (RFC 6455)
//!
//! This module provides a WebSocket implementation built from scratch.

use crate::http::{Request, Response, StatusCode};
use base64::Engine;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// WebSocket message types
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Text message (UTF-8)
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(Vec<u8>),
    /// Close message
    Close,
}

/// WebSocket opcode
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum OpCode {
    Continue = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl OpCode {
    fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x0 => Some(OpCode::Continue),
            0x1 => Some(OpCode::Text),
            0x2 => Some(OpCode::Binary),
            0x8 => Some(OpCode::Close),
            0x9 => Some(OpCode::Ping),
            0xA => Some(OpCode::Pong),
            _ => None,
        }
    }
}

/// WebSocket connection
pub struct WebSocket {
    stream: Arc<Mutex<TcpStream>>,
}

impl WebSocket {
    /// Create a new WebSocket from a TcpStream
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
        }
    }

    /// Receive a message from the WebSocket
    pub async fn recv(&self) -> Option<Result<Message, String>> {
        let mut stream = self.stream.lock().await;

        // Read frame header (first 2 bytes)
        let mut header = [0u8; 2];
        if stream.read_exact(&mut header).await.is_err() {
            return None;
        }

        let _fin = (header[0] & 0b10000000) != 0;
        let opcode = OpCode::from_u8(header[0] & 0b00001111)?;
        let masked = (header[1] & 0b10000000) != 0;
        let mut payload_len = (header[1] & 0b01111111) as u64;

        // Extended payload length
        if payload_len == 126 {
            let mut len_bytes = [0u8; 2];
            if stream.read_exact(&mut len_bytes).await.is_err() {
                return None;
            }
            payload_len = u16::from_be_bytes(len_bytes) as u64;
        } else if payload_len == 127 {
            let mut len_bytes = [0u8; 8];
            if stream.read_exact(&mut len_bytes).await.is_err() {
                return None;
            }
            payload_len = u64::from_be_bytes(len_bytes);
        }

        // Read masking key if present
        let mask_key = if masked {
            let mut key = [0u8; 4];
            if stream.read_exact(&mut key).await.is_err() {
                return None;
            }
            Some(key)
        } else {
            None
        };

        // Read payload
        let mut payload = vec![0u8; payload_len as usize];
        if stream.read_exact(&mut payload).await.is_err() {
            return None;
        }

        // Unmask payload if needed
        if let Some(key) = mask_key {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= key[i % 4];
            }
        }

        // Convert to message
        let message = match opcode {
            OpCode::Text => {
                let text = String::from_utf8(payload)
                    .map_err(|e| format!("Invalid UTF-8: {}", e))
                    .ok()?;
                Message::Text(text)
            }
            OpCode::Binary => Message::Binary(payload),
            OpCode::Ping => Message::Ping(payload),
            OpCode::Pong => Message::Pong(payload),
            OpCode::Close => Message::Close,
            OpCode::Continue => {
                return Some(Err("Continuation frames not yet supported".to_string()))
            }
        };

        Some(Ok(message))
    }

    /// Send a message through the WebSocket
    pub async fn send(&self, message: Message) -> Result<(), String> {
        let mut stream = self.stream.lock().await;

        let (opcode, payload) = match message {
            Message::Text(text) => (OpCode::Text, text.into_bytes()),
            Message::Binary(data) => (OpCode::Binary, data),
            Message::Ping(data) => (OpCode::Ping, data),
            Message::Pong(data) => (OpCode::Pong, data),
            Message::Close => (OpCode::Close, vec![]),
        };

        let mut frame = Vec::new();

        // First byte: FIN + opcode
        frame.push(0b10000000 | (opcode as u8));

        // Second byte: MASK + payload length
        let payload_len = payload.len();
        if payload_len < 126 {
            frame.push(payload_len as u8);
        } else if payload_len <= 65535 {
            frame.push(126);
            frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            frame.push(127);
            frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }

        // Payload (server doesn't mask)
        frame.extend_from_slice(&payload);

        stream
            .write_all(&frame)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(())
    }

    /// Close the WebSocket connection
    pub async fn close(&self) -> Result<(), String> {
        self.send(Message::Close).await
    }
}

/// WebSocket upgrade handler
pub struct WebSocketUpgrade {
    request: Request,
    stream: TcpStream,
}

impl WebSocketUpgrade {
    /// Create a new WebSocket upgrade from a request and stream
    pub fn new(request: Request, stream: TcpStream) -> Self {
        Self { request, stream }
    }

    /// Check if the request is a valid WebSocket upgrade
    pub fn is_upgrade_request(request: &Request) -> bool {
        request
            .headers()
            .get("upgrade")
            .map(|v| v.to_lowercase() == "websocket")
            .unwrap_or(false)
            && request
                .headers()
                .get("connection")
                .map(|v| v.to_lowercase().contains("upgrade"))
                .unwrap_or(false)
            && request.headers().contains_key("sec-websocket-key")
            && request
                .headers()
                .get("sec-websocket-version")
                .map(|v| v == "13")
                .unwrap_or(false)
    }

    /// Perform the WebSocket handshake and upgrade the connection
    pub async fn upgrade<F, Fut>(self, handler: F) -> Result<(), String>
    where
        F: FnOnce(WebSocket) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // Get the WebSocket key
        let key = self
            .request
            .headers()
            .get("sec-websocket-key")
            .ok_or("Missing Sec-WebSocket-Key")?;

        // Compute accept key
        let accept_key = compute_accept_key(key);

        // Create upgrade response
        let response = Response::new(StatusCode::SwitchingProtocols)
            .with_header("Upgrade".to_string(), "websocket".to_string())
            .with_header("Connection".to_string(), "Upgrade".to_string())
            .with_header("Sec-WebSocket-Accept".to_string(), accept_key);

        // Send response
        let mut stream = self.stream;
        let response_bytes = response.to_bytes();
        stream
            .write_all(&response_bytes)
            .await
            .map_err(|e| format!("Failed to send upgrade response: {}", e))?;

        // Create WebSocket and run handler
        let ws = WebSocket::new(stream);
        handler(ws).await;

        Ok(())
    }
}

/// Compute the Sec-WebSocket-Accept value
fn compute_accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let hash = hasher.finalize();

    base64::engine::general_purpose::STANDARD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_accept_key() {
        // Test from RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let expected = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
        assert_eq!(compute_accept_key(key), expected);
    }

    #[test]
    fn test_opcode_conversion() {
        assert_eq!(OpCode::from_u8(0x1), Some(OpCode::Text));
        assert_eq!(OpCode::from_u8(0x2), Some(OpCode::Binary));
        assert_eq!(OpCode::from_u8(0x8), Some(OpCode::Close));
        assert_eq!(OpCode::from_u8(0x9), Some(OpCode::Ping));
        assert_eq!(OpCode::from_u8(0xA), Some(OpCode::Pong));
        assert_eq!(OpCode::from_u8(0xFF), None);
    }

    #[test]
    fn test_message_types() {
        let text = Message::Text("hello".to_string());
        assert_eq!(text, Message::Text("hello".to_string()));

        let binary = Message::Binary(vec![1, 2, 3]);
        assert_eq!(binary, Message::Binary(vec![1, 2, 3]));

        let ping = Message::Ping(vec![]);
        assert_eq!(ping, Message::Ping(vec![]));

        let pong = Message::Pong(vec![]);
        assert_eq!(pong, Message::Pong(vec![]));

        let close = Message::Close;
        assert_eq!(close, Message::Close);
    }
}
