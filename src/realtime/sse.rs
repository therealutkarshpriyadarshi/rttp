//! Server-Sent Events (SSE) implementation
//!
//! SSE provides a simple way to stream events from server to client over HTTP.
//! The client establishes a connection and receives events as they are sent.

use crate::http::{Response, StatusCode};
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// An SSE event
#[derive(Debug, Clone)]
pub struct Event {
    /// Event type/name
    event: Option<String>,
    /// Event data
    data: Vec<String>,
    /// Event ID
    id: Option<String>,
    /// Retry timeout in milliseconds
    retry: Option<u64>,
    /// Comment
    comment: Option<String>,
}

impl Event {
    /// Create a new event with the given type
    pub fn new(event: impl Into<String>) -> Self {
        Self {
            event: Some(event.into()),
            data: Vec::new(),
            id: None,
            retry: None,
            comment: None,
        }
    }

    /// Create a default event (no type specified)
    pub fn default() -> Self {
        Self {
            event: None,
            data: Vec::new(),
            id: None,
            retry: None,
            comment: None,
        }
    }

    /// Set the event data (can be called multiple times for multi-line data)
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data.push(data.into());
        self
    }

    /// Set the event ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the retry timeout
    pub fn retry(mut self, milliseconds: u64) -> Self {
        self.retry = Some(milliseconds);
        self
    }

    /// Set a comment
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Format the event as SSE text
    fn to_sse_format(&self) -> String {
        let mut output = String::new();

        if let Some(ref comment) = self.comment {
            output.push_str(&format!(": {}\n", comment));
        }

        if let Some(ref event) = self.event {
            output.push_str(&format!("event: {}\n", event));
        }

        for data_line in &self.data {
            output.push_str(&format!("data: {}\n", data_line));
        }

        if let Some(ref id) = self.id {
            output.push_str(&format!("id: {}\n", id));
        }

        if let Some(retry) = self.retry {
            output.push_str(&format!("retry: {}\n", retry));
        }

        output.push('\n'); // Empty line terminates the event
        output
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_sse_format())
    }
}

/// SSE stream for sending events to a client
#[derive(Clone)]
pub struct SseStream {
    sender: Arc<UnboundedSender<Event>>,
}

impl SseStream {
    /// Create a new SSE stream
    pub fn new() -> (Self, SseReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                sender: Arc::new(tx),
            },
            SseReceiver { receiver: rx },
        )
    }

    /// Send an event to the stream
    pub async fn send(&self, event: Event) -> Result<(), String> {
        self.sender
            .send(event)
            .map_err(|e| format!("Failed to send event: {}", e))
    }

    /// Send a simple message event
    pub async fn send_message(&self, data: impl Into<String>) -> Result<(), String> {
        self.send(Event::default().data(data)).await
    }

    /// Send a comment (keep-alive)
    pub async fn send_comment(&self, comment: impl Into<String>) -> Result<(), String> {
        self.send(Event::default().comment(comment)).await
    }
}

impl Default for SseStream {
    fn default() -> Self {
        Self::new().0
    }
}

/// SSE receiver for consuming events
pub struct SseReceiver {
    receiver: UnboundedReceiver<Event>,
}

impl SseReceiver {
    /// Receive the next event
    pub async fn recv(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    /// Convert the receiver into an HTTP response
    pub fn into_response(mut self) -> Response {
        // Create response body as a stream of SSE formatted events
        let mut body = Vec::new();

        // Collect all available events (non-blocking)
        while let Ok(event) = self.receiver.try_recv() {
            body.extend_from_slice(event.to_sse_format().as_bytes());
        }

        Response::new(StatusCode::Ok)
            .with_header("Content-Type".to_string(), "text/event-stream".to_string())
            .with_header("Cache-Control".to_string(), "no-cache".to_string())
            .with_header("Connection".to_string(), "keep-alive".to_string())
            .with_body(body)
    }
}

/// Helper to create an SSE response from a stream
pub async fn create_sse_stream<F, Fut>(handler: F) -> Response
where
    F: FnOnce(SseStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (stream, receiver) = SseStream::new();

    // Spawn the handler to send events
    tokio::spawn(async move {
        handler(stream).await;
    });

    // Give the handler a moment to send initial events
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    receiver.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_formatting() {
        let event = Event::new("message")
            .data("Hello, World!")
            .id("1")
            .retry(5000);

        let formatted = event.to_sse_format();
        assert!(formatted.contains("event: message\n"));
        assert!(formatted.contains("data: Hello, World!\n"));
        assert!(formatted.contains("id: 1\n"));
        assert!(formatted.contains("retry: 5000\n"));
    }

    #[test]
    fn test_multiline_data() {
        let event = Event::default().data("line 1").data("line 2").data("line 3");

        let formatted = event.to_sse_format();
        assert!(formatted.contains("data: line 1\n"));
        assert!(formatted.contains("data: line 2\n"));
        assert!(formatted.contains("data: line 3\n"));
    }

    #[test]
    fn test_comment() {
        let event = Event::default().comment("keep-alive");

        let formatted = event.to_sse_format();
        assert!(formatted.contains(": keep-alive\n"));
    }

    #[test]
    fn test_default_event() {
        let event = Event::default().data("Simple message");

        let formatted = event.to_sse_format();
        assert!(formatted.contains("data: Simple message\n"));
        assert!(!formatted.contains("event:"));
    }

    #[tokio::test]
    async fn test_sse_stream() {
        let (stream, mut receiver) = SseStream::new();

        // Send an event
        stream
            .send(Event::new("test").data("test data"))
            .await
            .unwrap();

        // Receive the event
        let event = receiver.recv().await.unwrap();
        assert_eq!(event.event, Some("test".to_string()));
        assert_eq!(event.data, vec!["test data".to_string()]);
    }

    #[tokio::test]
    async fn test_send_message() {
        let (stream, mut receiver) = SseStream::new();

        stream.send_message("Hello!").await.unwrap();

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.event, None);
        assert_eq!(event.data, vec!["Hello!".to_string()]);
    }

    #[tokio::test]
    async fn test_send_comment() {
        let (stream, mut receiver) = SseStream::new();

        stream.send_comment("ping").await.unwrap();

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.comment, Some("ping".to_string()));
    }
}
