//! RAG (Retrieval Augmented Generation) pipeline
//!
//! Provides a complete RAG implementation with document chunking,
//! embedding generation, similarity search, and response synthesis.

use crate::llm::{
    client::{CompletionRequest, LlmClient, LlmError, Message},
    tokens::{TokenBudget, TokenCounter},
    vector::{VectorEntry, VectorStore},
};

/// Configuration for the RAG pipeline
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// Chunk size in characters
    pub chunk_size: usize,
    /// Chunk overlap in characters
    pub chunk_overlap: usize,
    /// Number of chunks to retrieve
    pub top_k: usize,
    /// Minimum similarity threshold
    pub similarity_threshold: f32,
    /// Model to use for completion
    pub model: String,
    /// Maximum tokens for completion
    pub max_completion_tokens: u32,
    /// Temperature for sampling
    pub temperature: f32,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            chunk_size: 500,
            chunk_overlap: 50,
            top_k: 3,
            similarity_threshold: 0.7,
            model: "gpt-3.5-turbo".to_string(),
            max_completion_tokens: 500,
            temperature: 0.7,
        }
    }
}

/// Document chunk with metadata
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    /// Unique identifier
    pub id: String,
    /// Chunk content
    pub content: String,
    /// Source document ID
    pub source_id: String,
    /// Chunk index in source document
    pub chunk_index: usize,
    /// Optional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// RAG pipeline for document-based question answering
pub struct RagPipeline {
    /// Configuration
    config: RagConfig,
    /// Vector store for embeddings
    vector_store: VectorStore,
    /// Document chunks
    chunks: std::collections::HashMap<String, DocumentChunk>,
    /// LLM client
    llm_client: LlmClient,
    /// Token counter
    token_counter: TokenCounter,
}

impl RagPipeline {
    /// Create a new RAG pipeline
    pub fn new(llm_client: LlmClient) -> Self {
        Self::with_config(llm_client, RagConfig::default())
    }

    /// Create a new RAG pipeline with custom configuration
    pub fn with_config(llm_client: LlmClient, config: RagConfig) -> Self {
        Self {
            config,
            vector_store: VectorStore::new(),
            chunks: std::collections::HashMap::new(),
            llm_client,
            token_counter: TokenCounter::new(),
        }
    }

    /// Index a document by chunking and generating embeddings
    ///
    /// This is a simplified version that stores chunks without actual embeddings.
    /// In a real implementation, you would:
    /// 1. Call the embeddings API to get vectors for each chunk
    /// 2. Store those vectors in the vector store
    pub async fn index_document(
        &mut self,
        document_id: impl Into<String>,
        content: &str,
    ) -> Result<usize, LlmError> {
        let document_id = document_id.into();
        let chunks = self.chunk_document(content);

        let mut indexed_count = 0;

        for (index, chunk_text) in chunks.iter().enumerate() {
            let chunk_id = format!("{}#{}", document_id, index);

            // Create chunk metadata
            let chunk = DocumentChunk {
                id: chunk_id.clone(),
                content: chunk_text.clone(),
                source_id: document_id.clone(),
                chunk_index: index,
                metadata: std::collections::HashMap::new(),
            };

            // In a real implementation, call embeddings API here
            // For now, we'll use a simple hash-based vector as placeholder
            let embedding = self.generate_placeholder_embedding(chunk_text);

            // Store in vector store
            let entry = VectorEntry::new(chunk_id.clone(), embedding)
                .with_metadata("source_id", document_id.clone())
                .with_metadata("chunk_index", index.to_string());

            self.vector_store.insert(entry);
            self.chunks.insert(chunk_id, chunk);

            indexed_count += 1;
        }

        Ok(indexed_count)
    }

    /// Generate embeddings for text using the LLM API
    ///
    /// This method actually calls the embeddings API to get real vectors
    pub async fn index_document_with_embeddings(
        &mut self,
        document_id: impl Into<String>,
        content: &str,
    ) -> Result<usize, LlmError> {
        let document_id = document_id.into();
        let chunks = self.chunk_document(content);

        // Get embeddings for all chunks in one batch
        let embeddings_response = self.llm_client.embeddings(chunks.clone()).await?;

        let mut indexed_count = 0;

        for (index, embedding_data) in embeddings_response.data.iter().enumerate() {
            let chunk_text = &chunks[index];
            let chunk_id = format!("{}#{}", document_id, index);

            // Create chunk metadata
            let chunk = DocumentChunk {
                id: chunk_id.clone(),
                content: chunk_text.clone(),
                source_id: document_id.clone(),
                chunk_index: index,
                metadata: std::collections::HashMap::new(),
            };

            // Store in vector store
            let entry = VectorEntry::new(chunk_id.clone(), embedding_data.embedding.clone())
                .with_metadata("source_id", document_id.clone())
                .with_metadata("chunk_index", index.to_string());

            self.vector_store.insert(entry);
            self.chunks.insert(chunk_id, chunk);

            indexed_count += 1;
        }

        Ok(indexed_count)
    }

    /// Chunk a document into smaller pieces
    fn chunk_document(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.config.chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();

            if !chunk.trim().is_empty() {
                chunks.push(chunk);
            }

            // Move forward with overlap
            if end >= chars.len() {
                break;
            }
            start = end - self.config.chunk_overlap;
        }

        chunks
    }

    /// Generate a placeholder embedding based on simple text features
    ///
    /// This is used when embeddings API is not available.
    /// In production, always use real embeddings from the API.
    fn generate_placeholder_embedding(&self, text: &str) -> Vec<f32> {
        // Simple feature extraction (just for demonstration)
        // Real embeddings would come from the API
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut features = vec![0.0; 384]; // Typical embedding dimension

        // Use simple hashing to create pseudo-embeddings
        for (i, word) in words.iter().enumerate() {
            let hash = self.simple_hash(word);
            let index = (hash as usize) % features.len();
            features[index] += 1.0 / (i + 1) as f32;
        }

        // Normalize
        let magnitude: f32 = features.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for x in features.iter_mut() {
                *x /= magnitude;
            }
        }

        features
    }

    /// Simple hash function for placeholder embeddings
    fn simple_hash(&self, text: &str) -> u32 {
        text.bytes().fold(0u32, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(byte as u32)
        })
    }

    /// Query the RAG pipeline
    ///
    /// 1. Generate embedding for the query (or use placeholder)
    /// 2. Retrieve relevant chunks
    /// 3. Inject into prompt
    /// 4. Call LLM
    /// 5. Return response
    pub async fn query(&self, question: &str) -> Result<String, LlmError> {
        // Generate query embedding (using placeholder for now)
        let query_embedding = self.generate_placeholder_embedding(question);

        // Retrieve relevant chunks
        let results = self
            .vector_store
            .search_threshold(&query_embedding, self.config.similarity_threshold);

        let top_results: Vec<_> = results.into_iter().take(self.config.top_k).collect();

        if top_results.is_empty() {
            return Err(LlmError::ApiError(
                "No relevant documents found".to_string(),
            ));
        }

        // Build context from retrieved chunks
        let mut context = String::new();
        for (chunk_id, score) in &top_results {
            if let Some(chunk) = self.chunks.get(chunk_id) {
                context.push_str(&format!(
                    "Document (relevance: {:.2}):\n{}\n\n",
                    score, chunk.content
                ));
            }
        }

        // Create prompt with context
        let prompt = self.build_prompt(question, &context);

        // Call LLM
        let request = CompletionRequest::new(&self.config.model)
            .message(Message::user(prompt))
            .max_tokens(self.config.max_completion_tokens)
            .temperature(self.config.temperature);

        let response = self.llm_client.complete(request).await?;

        // Extract response text
        let answer = response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default();

        Ok(answer)
    }

    /// Query with custom system prompt
    pub async fn query_with_system(
        &self,
        question: &str,
        system_prompt: &str,
    ) -> Result<String, LlmError> {
        let query_embedding = self.generate_placeholder_embedding(question);
        let results = self
            .vector_store
            .search_threshold(&query_embedding, self.config.similarity_threshold);

        let top_results: Vec<_> = results.into_iter().take(self.config.top_k).collect();

        if top_results.is_empty() {
            return Err(LlmError::ApiError(
                "No relevant documents found".to_string(),
            ));
        }

        let mut context = String::new();
        for (chunk_id, score) in &top_results {
            if let Some(chunk) = self.chunks.get(chunk_id) {
                context.push_str(&format!(
                    "Document (relevance: {:.2}):\n{}\n\n",
                    score, chunk.content
                ));
            }
        }

        let user_prompt = self.build_prompt(question, &context);

        let request = CompletionRequest::new(&self.config.model)
            .message(Message::system(system_prompt))
            .message(Message::user(user_prompt))
            .max_tokens(self.config.max_completion_tokens)
            .temperature(self.config.temperature);

        let response = self.llm_client.complete(request).await?;

        let answer = response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default();

        Ok(answer)
    }

    /// Build a prompt with context and question
    fn build_prompt(&self, question: &str, context: &str) -> String {
        format!(
            "Based on the following context, please answer the question.\n\n\
             Context:\n{}\n\n\
             Question: {}\n\n\
             Answer:",
            context, question
        )
    }

    /// Get statistics about the indexed documents
    pub fn stats(&self) -> RagStats {
        RagStats {
            total_chunks: self.chunks.len(),
            total_documents: self
                .chunks
                .values()
                .map(|c| c.source_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            vector_store_size: self.vector_store.len(),
        }
    }

    /// Remove all documents from a source
    pub fn remove_document(&mut self, document_id: &str) -> usize {
        let mut removed_count = 0;

        // Find all chunk IDs for this document
        let chunk_ids: Vec<String> = self
            .chunks
            .values()
            .filter(|c| c.source_id == document_id)
            .map(|c| c.id.clone())
            .collect();

        // Remove each chunk
        for chunk_id in chunk_ids {
            self.vector_store.remove(&chunk_id);
            self.chunks.remove(&chunk_id);
            removed_count += 1;
        }

        removed_count
    }

    /// Clear all indexed documents
    pub fn clear(&mut self) {
        self.vector_store.clear();
        self.chunks.clear();
    }
}

/// Statistics about the RAG pipeline
#[derive(Debug, Clone)]
pub struct RagStats {
    /// Total number of chunks
    pub total_chunks: usize,
    /// Total number of documents
    pub total_documents: usize,
    /// Size of vector store
    pub vector_store_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> LlmClient {
        // Create a mock client for testing
        LlmClient::new("http://localhost", "test-key").unwrap()
    }

    #[test]
    fn test_document_chunking() {
        let config = RagConfig {
            chunk_size: 10,
            chunk_overlap: 2,
            ..Default::default()
        };

        let client = create_test_client();
        let pipeline = RagPipeline::with_config(client, config);

        let text = "This is a test document with some content";
        let chunks = pipeline.chunk_document(text);

        assert!(chunks.len() > 1);
        assert!(chunks[0].len() <= 10);
    }

    #[tokio::test]
    async fn test_index_document() {
        let client = create_test_client();
        let mut pipeline = RagPipeline::new(client);

        let text = "This is a test document with some content";
        let result = pipeline.index_document("doc1", text).await;

        assert!(result.is_ok());
        let count = result.unwrap();
        assert!(count > 0);

        let stats = pipeline.stats();
        assert_eq!(stats.total_documents, 1);
    }

    #[tokio::test]
    async fn test_remove_document() {
        let client = create_test_client();
        let mut pipeline = RagPipeline::new(client);

        pipeline.index_document("doc1", "Test content").await.unwrap();

        let removed = pipeline.remove_document("doc1");
        assert!(removed > 0);

        let stats = pipeline.stats();
        assert_eq!(stats.total_chunks, 0);
    }

    #[test]
    fn test_placeholder_embedding() {
        let client = create_test_client();
        let pipeline = RagPipeline::new(client);

        let embedding1 = pipeline.generate_placeholder_embedding("hello world");
        let embedding2 = pipeline.generate_placeholder_embedding("hello world");
        let embedding3 = pipeline.generate_placeholder_embedding("different text");

        // Same text should produce same embedding
        assert_eq!(embedding1, embedding2);

        // Different text should produce different embedding
        assert_ne!(embedding1, embedding3);
    }

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        assert_eq!(config.chunk_size, 500);
        assert_eq!(config.chunk_overlap, 50);
        assert_eq!(config.top_k, 3);
    }

    #[tokio::test]
    async fn test_multiple_documents() {
        let client = create_test_client();
        let mut pipeline = RagPipeline::new(client);

        pipeline
            .index_document("doc1", "First document content")
            .await
            .unwrap();
        pipeline
            .index_document("doc2", "Second document content")
            .await
            .unwrap();

        let stats = pipeline.stats();
        assert_eq!(stats.total_documents, 2);
    }
}
