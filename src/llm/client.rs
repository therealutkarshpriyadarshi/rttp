//! HTTP client for LLM APIs
//!
//! Provides a generic HTTP client for interacting with LLM APIs like OpenAI and Anthropic.
//! Supports both standard and streaming responses.

use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Errors that can occur during LLM API calls
#[derive(Debug)]
pub enum LlmError {
    /// HTTP request failed
    RequestFailed(String),
    /// Failed to parse response
    ParseError(String),
    /// API returned an error
    ApiError(String),
    /// Invalid API key or configuration
    ConfigError(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            LlmError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            LlmError::ApiError(msg) => write!(f, "API error: {}", msg),
            LlmError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

/// Result type for LLM operations
pub type Result<T> = std::result::Result<T, LlmError>;

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender (user, assistant, system)
    pub role: String,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// Request to LLM for completion
#[derive(Debug, Clone, Serialize)]
pub struct CompletionRequest {
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Temperature for sampling (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl CompletionRequest {
    /// Create a new completion request
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: None,
        }
    }

    /// Add a message to the request
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Enable streaming
    pub fn stream(mut self) -> Self {
        self.stream = Some(true);
        self
    }
}

/// Response from LLM completion
#[derive(Debug, Clone, Deserialize)]
pub struct CompletionResponse {
    /// ID of the completion
    pub id: String,
    /// Model used
    pub model: String,
    /// Choices returned
    pub choices: Vec<Choice>,
    /// Token usage statistics
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// A single choice in the completion response
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    /// Index of the choice
    pub index: u32,
    /// The message content
    pub message: Message,
    /// Reason for finishing
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Streaming chunk from LLM
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    /// ID of the completion
    pub id: String,
    /// Model used
    pub model: String,
    /// Choices in this chunk
    pub choices: Vec<StreamChoice>,
}

/// A single choice in a stream chunk
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
    /// Index of the choice
    pub index: u32,
    /// Delta (partial content)
    pub delta: Delta,
    /// Reason for finishing (if done)
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming response
#[derive(Debug, Clone, Deserialize)]
pub struct Delta {
    /// Role (usually only in first chunk)
    #[serde(default)]
    pub role: Option<String>,
    /// Partial content
    #[serde(default)]
    pub content: Option<String>,
}

/// HTTP client for LLM APIs
pub struct LlmClient {
    /// Base URL for the API
    base_url: String,
    /// API key for authentication
    api_key: String,
    /// HTTP client
    client: Client,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::ConfigError(e.to_string()))?;

        Ok(Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            client,
        })
    }

    /// Create a new OpenAI client
    pub fn openai(api_key: impl Into<String>) -> Result<Self> {
        Self::new("https://api.openai.com/v1", api_key)
    }

    /// Create a new Anthropic client
    pub fn anthropic(api_key: impl Into<String>) -> Result<Self> {
        Self::new("https://api.anthropic.com/v1", api_key)
    }

    /// Send a completion request
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))
    }

    /// Send a streaming completion request
    pub async fn stream(&self, request: CompletionRequest) -> Result<StreamingResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut req = request;
        req.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError(error_text));
        }

        Ok(StreamingResponse::new(response))
    }

    /// Generate embeddings for text
    pub async fn embeddings(&self, input: Vec<String>) -> Result<EmbeddingsResponse> {
        let url = format!("{}/embeddings", self.base_url);

        let request = EmbeddingsRequest {
            model: "text-embedding-ada-002".to_string(),
            input,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))
    }
}

/// Streaming response from LLM
pub struct StreamingResponse {
    stream: std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>
                + Send,
        >,
    >,
    buffer: String,
}

impl StreamingResponse {
    fn new(response: Response) -> Self {
        Self {
            stream: Box::pin(response.bytes_stream()),
            buffer: String::new(),
        }
    }

    /// Read the next chunk from the stream
    pub async fn next_chunk(&mut self) -> Result<Option<StreamChunk>> {
        use futures_util::StreamExt;

        loop {
            // Check if we have a complete line in the buffer
            if let Some(line_end) = self.buffer.find('\n') {
                let line = self.buffer[..line_end].to_string();
                self.buffer = self.buffer[line_end + 1..].to_string();

                if line.starts_with("data: ") {
                    let data = line[6..].trim();
                    if data == "[DONE]" {
                        return Ok(None);
                    }

                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        return Ok(Some(chunk));
                    }
                }
                continue;
            }

            // Need more data
            match self.stream.next().await {
                Some(Ok(bytes)) => {
                    let text = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&text);
                }
                Some(Err(e)) => return Err(LlmError::RequestFailed(e.to_string())),
                None => return Ok(None),
            }
        }
    }

    /// Collect all chunks into a single string
    pub async fn collect(mut self) -> Result<String> {
        let mut result = String::new();

        while let Some(chunk) = self.next_chunk().await? {
            for choice in chunk.choices {
                if let Some(content) = choice.delta.content {
                    result.push_str(&content);
                }
            }
        }

        Ok(result)
    }
}

/// Request for embeddings
#[derive(Debug, Serialize)]
struct EmbeddingsRequest {
    model: String,
    input: Vec<String>,
}

/// Response from embeddings API
#[derive(Debug, Deserialize)]
pub struct EmbeddingsResponse {
    /// Embeddings data
    pub data: Vec<EmbeddingData>,
    /// Model used
    pub model: String,
    /// Token usage
    pub usage: EmbeddingUsage,
}

/// Single embedding data
#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    /// Index of the embedding
    pub index: u32,
    /// The embedding vector
    pub embedding: Vec<f32>,
}

/// Usage statistics for embeddings
#[derive(Debug, Deserialize)]
pub struct EmbeddingUsage {
    /// Total tokens used
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there");

        let msg = Message::system("You are helpful");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are helpful");
    }

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::new("gpt-4")
            .message(Message::user("Hello"))
            .temperature(0.7)
            .max_tokens(100)
            .stream();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.stream, Some(true));
    }

    #[test]
    fn test_client_creation() {
        let client = LlmClient::openai("test-key");
        assert!(client.is_ok());

        let client = LlmClient::anthropic("test-key");
        assert!(client.is_ok());

        let client = LlmClient::new("https://custom.api", "test-key");
        assert!(client.is_ok());
    }
}
