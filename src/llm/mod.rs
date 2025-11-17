//! AI/LLM integration and RAG pipeline
//!
//! This module provides comprehensive AI/LLM integration features including:
//!
//! ## Features
//!
//! - **HTTP Client**: Generic client for LLM APIs (OpenAI, Anthropic)
//! - **Prompt Templates**: Jinja-like template engine for dynamic prompts
//! - **Token Management**: Token counting and context window management
//! - **Vector Database**: In-memory vector store with similarity search
//! - **RAG Pipeline**: Complete Retrieval Augmented Generation implementation
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pttp::llm::{LlmClient, RagPipeline, CompletionRequest, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an LLM client
//! let client = LlmClient::openai("your-api-key")?;
//!
//! // Simple completion
//! let request = CompletionRequest::new("gpt-3.5-turbo")
//!     .message(Message::user("What is Rust?"))
//!     .temperature(0.7);
//!
//! let response = client.complete(request).await?;
//! println!("Response: {}", response.choices[0].message.content);
//!
//! // RAG pipeline
//! let mut rag = RagPipeline::new(client);
//! rag.index_document("doc1", "Rust is a systems programming language...").await?;
//!
//! let answer = rag.query("What is Rust?").await?;
//! println!("RAG Answer: {}", answer);
//! # Ok(())
//! # }
//! ```

// Module declarations
pub mod client;
pub mod prompt;
pub mod rag;
pub mod tokens;
pub mod vector;

// Re-export commonly used types
pub use client::{
    Choice, CompletionRequest, CompletionResponse, Delta, EmbeddingData, EmbeddingsResponse,
    LlmClient, LlmError, Message, StreamChunk, StreamChoice, StreamingResponse, Usage,
};
pub use prompt::{Context, PromptBuilder, PromptTemplate, TemplateError};
pub use rag::{DocumentChunk, RagConfig, RagPipeline, RagStats};
pub use tokens::{ContextWindow, TokenBudget, TokenCounter};
pub use vector::{
    cosine_similarity, euclidean_distance, normalize, VectorEntry, VectorStore,
};
