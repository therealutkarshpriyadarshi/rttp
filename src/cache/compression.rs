//! Compression middleware for HTTP responses
//!
//! This module provides middleware for compressing HTTP responses using
//! Gzip or Brotli compression based on the Accept-Encoding header.
//!
//! # Features
//!
//! - Gzip compression support
//! - Brotli compression support
//! - Accept-Encoding negotiation
//! - Configurable compression levels
//! - Automatic Content-Encoding headers
//!
//! # Example
//!
//! ```no_run
//! use pttp::cache::Compression;
//! use pttp::middleware::MiddlewareStack;
//! use pttp::server::Server;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut stack = MiddlewareStack::new();
//!     stack.add(Compression::new(6)); // Compression level 0-9
//!
//!     // Use with server...
//! }
//! ```

use crate::context::Context;
use crate::http::Response;
use crate::middleware::{Middleware, Next};
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Gzip,
    Brotli,
    Identity, // No compression
}

impl Algorithm {
    /// Parses Accept-Encoding header to determine preferred algorithm
    fn from_accept_encoding(accept_encoding: &str) -> Self {
        let encodings: Vec<&str> = accept_encoding
            .split(',')
            .map(|s| s.trim().split(';').next().unwrap_or(""))
            .collect();

        // Prefer Brotli if available, then Gzip
        if encodings.contains(&"br") {
            Algorithm::Brotli
        } else if encodings.contains(&"gzip") {
            Algorithm::Gzip
        } else {
            Algorithm::Identity
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Gzip => "gzip",
            Algorithm::Brotli => "br",
            Algorithm::Identity => "identity",
        }
    }
}

/// Compression middleware
///
/// Compresses HTTP response bodies based on the Accept-Encoding header.
/// Supports Gzip and Brotli compression.
pub struct Compression {
    level: u32,
    min_size: usize,
}

/// Compresses data using Gzip
fn compress_gzip(data: &[u8], level: u32) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), GzCompression::new(level.min(9) as u32));
    encoder.write_all(data)?;
    encoder.finish()
}

/// Compresses data using Brotli
fn compress_brotli(data: &[u8], level: u32) -> Result<Vec<u8>, std::io::Error> {
    let mut output = Vec::new();
    let mut reader = std::io::Cursor::new(data);

    brotli::BrotliCompress(
        &mut reader,
        &mut output,
        &brotli::enc::BrotliEncoderParams {
            quality: level.min(11) as i32,
            ..Default::default()
        },
    )?;

    Ok(output)
}

/// Gets a header value case-insensitively
fn get_header<'a>(headers: &'a std::collections::HashMap<String, String>, key: &str) -> Option<&'a String> {
    // Try exact match first
    if let Some(value) = headers.get(key) {
        return Some(value);
    }

    // Try case-insensitive match
    let key_lower = key.to_lowercase();
    headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == key_lower)
        .map(|(_, v)| v)
}

/// Checks if the response should be compressed
fn should_compress_response(response: &Response, min_size: usize) -> bool {
    // Don't compress if already compressed
    if get_header(response.headers(), "content-encoding").is_some() {
        return false;
    }

    // Check minimum size first
    if response.body().len() < min_size {
        return false;
    }

    // Check content type (only compress text-based content)
    if let Some(content_type) = get_header(response.headers(), "content-type") {
        let content_type = content_type.to_lowercase();
        let compressible = content_type.starts_with("text/")
            || content_type.contains("json")
            || content_type.contains("xml")
            || content_type.contains("javascript")
            || content_type.contains("css");

        compressible
    } else {
        // No content-type header - don't compress
        false
    }
}

impl Compression {
    /// Creates a new compression middleware with the specified level
    ///
    /// # Arguments
    ///
    /// * `level` - Compression level (0-11 for Brotli, 0-9 for Gzip)
    pub fn new(level: u32) -> Self {
        Self {
            level,
            min_size: 1024, // Only compress responses >= 1KB
        }
    }

    /// Sets the minimum response size to compress
    pub fn with_min_size(mut self, min_size: usize) -> Self {
        self.min_size = min_size;
        self
    }
}

impl Middleware for Compression {
    fn handle(
        &self,
        ctx: Context,
        next: Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let level = self.level;
        let min_size = self.min_size;

        Box::pin(async move {
            // Get Accept-Encoding header
            let algorithm = get_header(ctx.request().headers(), "accept-encoding")
                .map(|s| Algorithm::from_accept_encoding(s))
                .unwrap_or(Algorithm::Identity);

            // Get response from next middleware/handler
            let mut response = next.run(ctx).await;

            // Skip compression if not needed
            if algorithm == Algorithm::Identity || !should_compress_response(&response, min_size) {
                return response;
            }

            // Compress the response body
            let body = response.body();
            let compressed = match algorithm {
                Algorithm::Gzip => compress_gzip(body, level),
                Algorithm::Brotli => compress_brotli(body, level),
                Algorithm::Identity => return response,
            };

            // Handle compression errors
            let compressed = match compressed {
                Ok(data) => data,
                Err(_) => return response, // Return uncompressed on error
            };

            // Update response with compressed body and headers
            response.set_body(compressed);
            response.set_header("Content-Encoding".to_string(), algorithm.as_str().to_string());
            response.remove_header("Content-Length"); // Will be set automatically

            response
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::StatusCode;

    #[test]
    fn test_algorithm_from_accept_encoding() {
        assert_eq!(
            Algorithm::from_accept_encoding("gzip, deflate, br"),
            Algorithm::Brotli
        );
        assert_eq!(
            Algorithm::from_accept_encoding("gzip, deflate"),
            Algorithm::Gzip
        );
        assert_eq!(
            Algorithm::from_accept_encoding("identity"),
            Algorithm::Identity
        );
    }

    #[test]
    fn test_algorithm_as_str() {
        assert_eq!(Algorithm::Gzip.as_str(), "gzip");
        assert_eq!(Algorithm::Brotli.as_str(), "br");
        assert_eq!(Algorithm::Identity.as_str(), "identity");
    }

    #[test]
    fn test_compress_gzip() {
        let data = b"Hello, World! This is a test string that should be compressed.";
        let compressed = compress_gzip(data, 6).unwrap();

        // Compressed data should be different and typically smaller for larger inputs
        assert_ne!(compressed.as_slice(), data);
    }

    #[test]
    fn test_compress_brotli() {
        let data = b"Hello, World! This is a test string that should be compressed.";
        let compressed = compress_brotli(data, 6).unwrap();

        // Compressed data should be different
        assert_ne!(compressed.as_slice(), data);
    }

    #[test]
    fn test_should_compress_text() {
        let mut response = Response::new(StatusCode::Ok);
        response.set_header("Content-Type".to_string(), "text/html".to_string());
        response.set_body(vec![0u8; 2048]); // Large enough

        assert!(should_compress_response(&response, 1024));
    }

    #[test]
    fn test_should_not_compress_small() {
        let mut response = Response::new(StatusCode::Ok);
        response.set_header("Content-Type".to_string(), "text/html".to_string());
        response.set_body(vec![0u8; 100]); // Too small

        assert!(!should_compress_response(&response, 1024));
    }

    #[test]
    fn test_should_not_compress_binary() {
        let mut response = Response::new(StatusCode::Ok);
        response.set_header("Content-Type".to_string(), "image/png".to_string());
        response.set_body(vec![0u8; 2048]);

        assert!(!should_compress_response(&response, 1024));
    }

    #[test]
    fn test_with_min_size() {
        let compression = Compression::new(6).with_min_size(5000);
        assert_eq!(compression.min_size, 5000);
    }
}
