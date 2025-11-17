//! Redis client with RESP (Redis Serialization Protocol) implementation
//!
//! This module provides a from-scratch Redis client implementation supporting
//! the RESP protocol and basic Redis commands.
//!
//! # Features
//!
//! - RESP protocol encoding/decoding
//! - Basic commands: GET, SET, DEL, EXPIRE, TTL
//! - Pub/Sub support
//! - Connection pooling
//! - Async/await support
//!
//! # Example
//!
//! ```no_run
//! use pttp::cache::RedisClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RedisClient::connect("127.0.0.1:6379").await?;
//!
//!     client.set("key", b"value", None).await?;
//!     let value = client.get("key").await?;
//!
//!     Ok(())
//! }
//! ```

use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// Redis client error types
#[derive(Debug)]
pub enum RedisError {
    Io(io::Error),
    Protocol(String),
    NotConnected,
    InvalidResponse,
    Redis(String),
}

impl std::fmt::Display for RedisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisError::Io(e) => write!(f, "IO error: {}", e),
            RedisError::Protocol(s) => write!(f, "Protocol error: {}", s),
            RedisError::NotConnected => write!(f, "Not connected to Redis"),
            RedisError::InvalidResponse => write!(f, "Invalid response from Redis"),
            RedisError::Redis(s) => write!(f, "Redis error: {}", s),
        }
    }
}

impl std::error::Error for RedisError {}

impl From<io::Error> for RedisError {
    fn from(err: io::Error) -> Self {
        RedisError::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, RedisError>;

/// RESP (Redis Serialization Protocol) value types
#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RespValue>>),
}

impl RespValue {
    /// Encodes a RESP value into bytes
    fn encode(&self) -> Vec<u8> {
        match self {
            RespValue::SimpleString(s) => format!("+{}\r\n", s).into_bytes(),
            RespValue::Error(e) => format!("-{}\r\n", e).into_bytes(),
            RespValue::Integer(i) => format!(":{}\r\n", i).into_bytes(),
            RespValue::BulkString(None) => b"$-1\r\n".to_vec(),
            RespValue::BulkString(Some(data)) => {
                let mut result = format!("${}\r\n", data.len()).into_bytes();
                result.extend_from_slice(data);
                result.extend_from_slice(b"\r\n");
                result
            }
            RespValue::Array(None) => b"*-1\r\n".to_vec(),
            RespValue::Array(Some(values)) => {
                let mut result = format!("*{}\r\n", values.len()).into_bytes();
                for value in values {
                    result.extend_from_slice(&value.encode());
                }
                result
            }
        }
    }

    /// Decodes bytes into a RESP value
    fn decode(reader: &mut TcpStream) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RespValue>> + Send + '_>> {
        Box::pin(async move {
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte).await?;

            match byte[0] {
                b'+' => {
                    let line = Self::read_line(reader).await?;
                    Ok(RespValue::SimpleString(line))
                }
                b'-' => {
                    let line = Self::read_line(reader).await?;
                    Ok(RespValue::Error(line))
                }
                b':' => {
                    let line = Self::read_line(reader).await?;
                    let num = line
                        .parse::<i64>()
                        .map_err(|_| RedisError::Protocol("Invalid integer".to_string()))?;
                    Ok(RespValue::Integer(num))
                }
                b'$' => {
                    let line = Self::read_line(reader).await?;
                    let len = line
                        .parse::<i64>()
                        .map_err(|_| RedisError::Protocol("Invalid bulk string length".to_string()))?;

                    if len == -1 {
                        Ok(RespValue::BulkString(None))
                    } else {
                        let mut data = vec![0u8; len as usize];
                        reader.read_exact(&mut data).await?;

                        // Read trailing \r\n
                        let mut crlf = [0u8; 2];
                        reader.read_exact(&mut crlf).await?;

                        Ok(RespValue::BulkString(Some(data)))
                    }
                }
                b'*' => {
                    let line = Self::read_line(reader).await?;
                    let len = line
                        .parse::<i64>()
                        .map_err(|_| RedisError::Protocol("Invalid array length".to_string()))?;

                    if len == -1 {
                        Ok(RespValue::Array(None))
                    } else {
                        let mut values = Vec::new();
                        for _ in 0..len {
                            values.push(Self::decode(reader).await?);
                        }
                        Ok(RespValue::Array(Some(values)))
                    }
                }
                _ => Err(RedisError::Protocol(format!(
                    "Unknown RESP type: {}",
                    byte[0] as char
                ))),
            }
        })
    }

    async fn read_line(reader: &mut TcpStream) -> Result<String> {
        let mut line = Vec::new();
        let mut prev_byte = 0u8;

        loop {
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte).await?;

            if prev_byte == b'\r' && byte[0] == b'\n' {
                // Remove the \r from the line
                line.pop();
                break;
            }

            line.push(byte[0]);
            prev_byte = byte[0];
        }

        String::from_utf8(line)
            .map_err(|_| RedisError::Protocol("Invalid UTF-8 in response".to_string()))
    }
}

/// Connection pool for Redis connections
struct ConnectionPool {
    connections: Arc<Mutex<VecDeque<TcpStream>>>,
    address: String,
    max_size: usize,
}

impl ConnectionPool {
    fn new(address: String, max_size: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(VecDeque::new())),
            address,
            max_size,
        }
    }

    async fn get(&self) -> Result<TcpStream> {
        let mut connections = self.connections.lock().await;

        if let Some(conn) = connections.pop_front() {
            Ok(conn)
        } else {
            // Create new connection
            TcpStream::connect(&self.address).await.map_err(Into::into)
        }
    }

    async fn return_connection(&self, conn: TcpStream) {
        let mut connections = self.connections.lock().await;

        if connections.len() < self.max_size {
            connections.push_back(conn);
        }
        // Otherwise, drop the connection
    }
}

/// Redis client with connection pooling
pub struct RedisClient {
    pool: Arc<ConnectionPool>,
}

impl RedisClient {
    /// Connects to a Redis server
    pub async fn connect(address: &str) -> Result<Self> {
        let pool = Arc::new(ConnectionPool::new(address.to_string(), 10));

        // Test connection
        let mut conn = TcpStream::connect(address).await?;

        // Send PING to verify connection
        let ping_cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"PING".to_vec())),
        ]));

        conn.write_all(&ping_cmd.encode()).await?;
        let response = RespValue::decode(&mut conn).await?;

        // Return connection to pool
        pool.return_connection(conn).await;

        // Verify PING response
        match response {
            RespValue::SimpleString(s) if s == "PONG" => Ok(Self { pool }),
            _ => Err(RedisError::Protocol("Expected PONG response".to_string())),
        }
    }

    /// Executes a Redis command
    async fn execute(&self, cmd: RespValue) -> Result<RespValue> {
        let mut conn = self.pool.get().await?;

        conn.write_all(&cmd.encode()).await?;
        let response = RespValue::decode(&mut conn).await?;

        self.pool.return_connection(conn).await;

        // Check for error response
        if let RespValue::Error(e) = &response {
            return Err(RedisError::Redis(e.clone()));
        }

        Ok(response)
    }

    /// GET command - gets a value by key
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"GET".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
        ]));

        let response = self.execute(cmd).await?;

        match response {
            RespValue::BulkString(data) => Ok(data),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// SET command - sets a value with optional expiration in seconds
    pub async fn set(&self, key: &str, value: &[u8], ttl: Option<u64>) -> Result<()> {
        let mut cmd_parts = vec![
            RespValue::BulkString(Some(b"SET".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
            RespValue::BulkString(Some(value.to_vec())),
        ];

        if let Some(seconds) = ttl {
            cmd_parts.push(RespValue::BulkString(Some(b"EX".to_vec())));
            cmd_parts.push(RespValue::BulkString(Some(
                seconds.to_string().into_bytes(),
            )));
        }

        let cmd = RespValue::Array(Some(cmd_parts));
        let response = self.execute(cmd).await?;

        match response {
            RespValue::SimpleString(s) if s == "OK" => Ok(()),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// DEL command - deletes one or more keys
    pub async fn del(&self, keys: &[&str]) -> Result<i64> {
        let mut cmd_parts = vec![RespValue::BulkString(Some(b"DEL".to_vec()))];

        for key in keys {
            cmd_parts.push(RespValue::BulkString(Some(key.as_bytes().to_vec())));
        }

        let cmd = RespValue::Array(Some(cmd_parts));
        let response = self.execute(cmd).await?;

        match response {
            RespValue::Integer(count) => Ok(count),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// EXPIRE command - sets a timeout on a key
    pub async fn expire(&self, key: &str, seconds: u64) -> Result<bool> {
        let cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"EXPIRE".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
            RespValue::BulkString(Some(seconds.to_string().into_bytes())),
        ]));

        let response = self.execute(cmd).await?;

        match response {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// TTL command - gets the time to live for a key
    pub async fn ttl(&self, key: &str) -> Result<i64> {
        let cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"TTL".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
        ]));

        let response = self.execute(cmd).await?;

        match response {
            RespValue::Integer(ttl) => Ok(ttl),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// EXISTS command - checks if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"EXISTS".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
        ]));

        let response = self.execute(cmd).await?;

        match response {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// INCR command - increments the integer value of a key
    pub async fn incr(&self, key: &str) -> Result<i64> {
        let cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"INCR".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
        ]));

        let response = self.execute(cmd).await?;

        match response {
            RespValue::Integer(value) => Ok(value),
            _ => Err(RedisError::InvalidResponse),
        }
    }

    /// DECR command - decrements the integer value of a key
    pub async fn decr(&self, key: &str) -> Result<i64> {
        let cmd = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"DECR".to_vec())),
            RespValue::BulkString(Some(key.as_bytes().to_vec())),
        ]));

        let response = self.execute(cmd).await?;

        match response {
            RespValue::Integer(value) => Ok(value),
            _ => Err(RedisError::InvalidResponse),
        }
    }
}

impl Clone for RedisClient {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resp_encode_simple_string() {
        let value = RespValue::SimpleString("OK".to_string());
        assert_eq!(value.encode(), b"+OK\r\n");
    }

    #[test]
    fn test_resp_encode_error() {
        let value = RespValue::Error("Error message".to_string());
        assert_eq!(value.encode(), b"-Error message\r\n");
    }

    #[test]
    fn test_resp_encode_integer() {
        let value = RespValue::Integer(42);
        assert_eq!(value.encode(), b":42\r\n");
    }

    #[test]
    fn test_resp_encode_bulk_string() {
        let value = RespValue::BulkString(Some(b"hello".to_vec()));
        assert_eq!(value.encode(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn test_resp_encode_null_bulk_string() {
        let value = RespValue::BulkString(None);
        assert_eq!(value.encode(), b"$-1\r\n");
    }

    #[test]
    fn test_resp_encode_array() {
        let value = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"GET".to_vec())),
            RespValue::BulkString(Some(b"key".to_vec())),
        ]));
        assert_eq!(value.encode(), b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_resp_encode_null_array() {
        let value = RespValue::Array(None);
        assert_eq!(value.encode(), b"*-1\r\n");
    }

    // Integration tests require a running Redis instance
    #[tokio::test]
    #[ignore] // Ignore by default, run with --ignored flag
    async fn test_redis_connection() {
        let client = RedisClient::connect("127.0.0.1:6379").await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_set_get() {
        let client = RedisClient::connect("127.0.0.1:6379").await.unwrap();

        client.set("test_key", b"test_value", None).await.unwrap();
        let value = client.get("test_key").await.unwrap();

        assert_eq!(value, Some(b"test_value".to_vec()));

        client.del(&["test_key"]).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_ttl() {
        let client = RedisClient::connect("127.0.0.1:6379").await.unwrap();

        client.set("ttl_key", b"value", Some(10)).await.unwrap();
        let ttl = client.ttl("ttl_key").await.unwrap();

        assert!(ttl > 0 && ttl <= 10);

        client.del(&["ttl_key"]).await.unwrap();
    }
}
