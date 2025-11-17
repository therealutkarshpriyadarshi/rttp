//! HTTP protocol implementation
//!
//! This module provides HTTP/1.1 protocol support including:
//! - Request parsing
//! - Response building
//! - Header management
//! - Method and status code types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP request methods
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
            Method::CONNECT => "CONNECT",
            Method::TRACE => "TRACE",
        }
    }
}

/// HTTP status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusCode {
    // 2xx Success
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,

    // 3xx Redirection
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,

    // 4xx Client Errors
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,

    // 5xx Server Errors
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
}

impl StatusCode {
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }

    pub fn reason_phrase(&self) -> &str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::Accepted => "Accepted",
            StatusCode::NoContent => "No Content",
            StatusCode::MovedPermanently => "Moved Permanently",
            StatusCode::Found => "Found",
            StatusCode::SeeOther => "See Other",
            StatusCode::NotModified => "Not Modified",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::BadGateway => "Bad Gateway",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// HTTP version
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    Http10,
    Http11,
}

impl Version {
    pub fn as_str(&self) -> &str {
        match self {
            Version::Http10 => "HTTP/1.0",
            Version::Http11 => "HTTP/1.1",
        }
    }
}

/// HTTP headers
pub type HeaderMap = HashMap<String, String>;

/// HTTP request
#[derive(Debug)]
pub struct Request {
    method: Method,
    uri: String,
    version: Version,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, uri: String) -> Self {
        Self {
            method,
            uri,
            version: Version::Http11,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }
}

/// HTTP response
#[derive(Debug)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self::new(StatusCode::Ok)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NotFound)
    }

    pub fn internal_error() -> Self {
        Self::new(StatusCode::InternalServerError)
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn json<T: Serialize>(data: &T) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_vec(data)?;
        Ok(Self::ok()
            .with_header("Content-Type".to_string(), "application/json".to_string())
            .with_body(body))
    }

    pub fn html(content: impl Into<String>) -> Self {
        Self::ok()
            .with_header(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            )
            .with_body(content.into().into_bytes())
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self::ok()
            .with_header(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )
            .with_body(content.into().into_bytes())
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Convert response to HTTP/1.1 wire format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Status line
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status.as_u16(),
            self.status.reason_phrase()
        );
        bytes.extend_from_slice(status_line.as_bytes());

        // Headers
        for (name, value) in &self.headers {
            bytes.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
        }

        // Content-Length header if not present
        if !self.headers.contains_key("Content-Length") {
            bytes.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        }

        // Empty line separating headers from body
        bytes.extend_from_slice(b"\r\n");

        // Body
        bytes.extend_from_slice(&self.body);

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_as_str() {
        assert_eq!(Method::GET.as_str(), "GET");
        assert_eq!(Method::POST.as_str(), "POST");
    }

    #[test]
    fn test_status_code() {
        assert_eq!(StatusCode::Ok.as_u16(), 200);
        assert_eq!(StatusCode::Ok.reason_phrase(), "OK");
    }

    #[test]
    fn test_response_builder() {
        let response = Response::ok()
            .with_header("X-Test".to_string(), "value".to_string())
            .with_body(b"Hello".to_vec());

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.body(), b"Hello");
    }

    #[test]
    fn test_response_to_bytes() {
        let response = Response::text("Hello, World!");
        let bytes = response.to_bytes();
        let result = String::from_utf8_lossy(&bytes);

        assert!(result.contains("HTTP/1.1 200 OK"));
        assert!(result.contains("Content-Type: text/plain; charset=utf-8"));
        assert!(result.contains("Hello, World!"));
    }
}
