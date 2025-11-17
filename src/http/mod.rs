//! HTTP protocol implementation
//!
//! This module provides HTTP/1.1 protocol support including:
//! - Request parsing
//! - Response building
//! - Header management
//! - Method and status code types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::{self, FromStr};

/// HTTP parsing errors
#[derive(Debug)]
pub enum ParseError {
    InvalidRequestLine,
    InvalidMethod,
    InvalidVersion,
    InvalidHeader,
    InvalidBody,
    IncompleteRequest,
    Utf8Error(str::Utf8Error),
    IoError(std::io::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidRequestLine => write!(f, "Invalid HTTP request line"),
            ParseError::InvalidMethod => write!(f, "Invalid HTTP method"),
            ParseError::InvalidVersion => write!(f, "Invalid HTTP version"),
            ParseError::InvalidHeader => write!(f, "Invalid HTTP header"),
            ParseError::InvalidBody => write!(f, "Invalid HTTP body"),
            ParseError::IncompleteRequest => write!(f, "Incomplete HTTP request"),
            ParseError::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            ParseError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<str::Utf8Error> for ParseError {
    fn from(err: str::Utf8Error) -> Self {
        ParseError::Utf8Error(err)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

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

impl FromStr for Method {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<Self> {
        match s {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "CONNECT" => Ok(Method::CONNECT),
            "TRACE" => Ok(Method::TRACE),
            _ => Err(ParseError::InvalidMethod),
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

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<Self> {
        match s {
            "HTTP/1.0" => Ok(Version::Http10),
            "HTTP/1.1" => Ok(Version::Http11),
            _ => Err(ParseError::InvalidVersion),
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

    /// Parse an HTTP request from raw bytes
    pub fn parse(buffer: &[u8]) -> ParseResult<Option<(Self, usize)>> {
        // Convert buffer to string for parsing headers
        let buffer_str = str::from_utf8(buffer)?;

        // Find the end of headers (double CRLF)
        let headers_end = match buffer_str.find("\r\n\r\n") {
            Some(pos) => pos,
            None => return Ok(None), // Incomplete request
        };

        let headers_section = &buffer_str[..headers_end];
        let lines: Vec<&str> = headers_section.split("\r\n").collect();

        if lines.is_empty() {
            return Err(ParseError::InvalidRequestLine);
        }

        // Parse request line
        let (method, uri, version) = Self::parse_request_line(lines[0])?;

        // Parse headers
        let mut headers = HashMap::new();
        for line in &lines[1..] {
            if line.is_empty() {
                continue;
            }
            let (name, value) = Self::parse_header(line)?;
            headers.insert(name, value);
        }

        // Calculate body start position
        let body_start = headers_end + 4; // Skip \r\n\r\n

        // Determine body length
        let content_length = headers
            .get("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        // Check if we have the complete body
        if buffer.len() < body_start + content_length {
            return Ok(None); // Incomplete request
        }

        // Extract body
        let body = buffer[body_start..body_start + content_length].to_vec();

        let request = Self {
            method,
            uri,
            version,
            headers,
            body,
        };

        let total_bytes = body_start + content_length;
        Ok(Some((request, total_bytes)))
    }

    /// Parse request line: METHOD URI VERSION
    fn parse_request_line(line: &str) -> ParseResult<(Method, String, Version)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(ParseError::InvalidRequestLine);
        }

        let method = Method::from_str(parts[0])?;
        let uri = parts[1].to_string();
        let version = Version::from_str(parts[2])?;

        Ok((method, uri, version))
    }

    /// Parse a header line: Name: Value
    fn parse_header(line: &str) -> ParseResult<(String, String)> {
        let colon_pos = line.find(':').ok_or(ParseError::InvalidHeader)?;
        let name = line[..colon_pos].trim().to_string();
        let value = line[colon_pos + 1..].trim().to_string();
        Ok((name, value))
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

    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
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
    fn test_method_from_str() {
        assert_eq!(Method::from_str("GET").unwrap(), Method::GET);
        assert_eq!(Method::from_str("POST").unwrap(), Method::POST);
        assert_eq!(Method::from_str("PUT").unwrap(), Method::PUT);
        assert_eq!(Method::from_str("DELETE").unwrap(), Method::DELETE);
        assert!(Method::from_str("INVALID").is_err());
    }

    #[test]
    fn test_version_from_str() {
        assert_eq!(Version::from_str("HTTP/1.0").unwrap(), Version::Http10);
        assert_eq!(Version::from_str("HTTP/1.1").unwrap(), Version::Http11);
        assert!(Version::from_str("HTTP/2.0").is_err());
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

    #[test]
    fn test_parse_simple_get_request() {
        let raw_request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let result = Request::parse(raw_request).unwrap();

        assert!(result.is_some());
        let (request, bytes_consumed) = result.unwrap();

        assert_eq!(request.method(), &Method::GET);
        assert_eq!(request.uri(), "/");
        assert_eq!(request.version(), &Version::Http11);
        assert_eq!(request.header("Host"), Some(&"localhost".to_string()));
        assert_eq!(bytes_consumed, raw_request.len());
    }

    #[test]
    fn test_parse_request_with_multiple_headers() {
        let raw_request = b"GET /api/users HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Test\r\nAccept: application/json\r\n\r\n";
        let result = Request::parse(raw_request).unwrap();

        assert!(result.is_some());
        let (request, _) = result.unwrap();

        assert_eq!(request.method(), &Method::GET);
        assert_eq!(request.uri(), "/api/users");
        assert_eq!(request.header("Host"), Some(&"example.com".to_string()));
        assert_eq!(request.header("User-Agent"), Some(&"Test".to_string()));
        assert_eq!(request.header("Accept"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_parse_post_request_with_body() {
        let raw_request = b"POST /api/data HTTP/1.1\r\nHost: example.com\r\nContent-Length: 13\r\n\r\nHello, World!";
        let result = Request::parse(raw_request).unwrap();

        assert!(result.is_some());
        let (request, bytes_consumed) = result.unwrap();

        assert_eq!(request.method(), &Method::POST);
        assert_eq!(request.uri(), "/api/data");
        assert_eq!(request.body(), b"Hello, World!");
        assert_eq!(bytes_consumed, raw_request.len());
    }

    #[test]
    fn test_parse_incomplete_request() {
        let raw_request = b"GET / HTTP/1.1\r\nHost: localhost\r\n";
        let result = Request::parse(raw_request).unwrap();

        // Should return None for incomplete request
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_incomplete_body() {
        let raw_request = b"POST /data HTTP/1.1\r\nContent-Length: 20\r\n\r\nOnly 10";
        let result = Request::parse(raw_request).unwrap();

        // Should return None when body is incomplete
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_request_line() {
        let raw_request = b"INVALID\r\n\r\n";
        let result = Request::parse(raw_request);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_method() {
        let raw_request = b"INVALID / HTTP/1.1\r\n\r\n";
        let result = Request::parse(raw_request);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_version() {
        let raw_request = b"GET / HTTP/2.0\r\n\r\n";
        let result = Request::parse(raw_request);

        assert!(result.is_err());
    }

    #[test]
    fn test_response_json() {
        #[derive(Serialize)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let response = Response::json(&data).unwrap();
        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.header("Content-Type"),
            Some(&"application/json".to_string())
        );

        let body_str = String::from_utf8_lossy(response.body());
        assert!(body_str.contains("test"));
        assert!(body_str.contains("42"));
    }

    #[test]
    fn test_response_html() {
        let response = Response::html("<h1>Hello</h1>");
        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.header("Content-Type"),
            Some(&"text/html; charset=utf-8".to_string())
        );
        assert_eq!(response.body(), b"<h1>Hello</h1>");
    }
}
