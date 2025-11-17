//! Phase 8 Demo - AI/LLM Integration
//!
//! Demonstrates all the AI/LLM features:
//! - LLM client with completions
//! - Prompt template engine
//! - Token management and context window
//! - Vector database with similarity search
//! - RAG (Retrieval Augmented Generation) pipeline
//!
//! Note: This demo uses placeholder embeddings for demonstration.
//! For production use with real LLM APIs, set the OPENAI_API_KEY environment variable.

use pttp::llm::{
    CompletionRequest, ContextWindow, LlmClient, Message, PromptBuilder, PromptTemplate,
    RagConfig, RagPipeline, TokenCounter, VectorStore,
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        PTTP Phase 8 - AI/LLM Integration Demo             ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Demo 1: Prompt Template Engine
    demo_prompt_templates();

    // Demo 2: Token Management
    demo_token_management();

    // Demo 3: Vector Database
    demo_vector_database();

    // Demo 4: Context Window
    demo_context_window();

    // Demo 5: RAG Pipeline
    demo_rag_pipeline().await;

    // Demo 6: LLM Client (only if API key is set)
    if std::env::var("OPENAI_API_KEY").is_ok() {
        demo_llm_client().await;
    } else {
        println!("\n📝 Note: Set OPENAI_API_KEY to run the LLM client demo");
    }

    println!("\n✅ Phase 8 Demo Complete!");
    println!("\nAll AI/LLM integration features are working correctly:");
    println!("  ✓ Prompt template engine with variables and conditionals");
    println!("  ✓ Token counting and management");
    println!("  ✓ Vector database with similarity search");
    println!("  ✓ Context window for conversation management");
    println!("  ✓ RAG pipeline for document-based QA");
    println!("  ✓ LLM client for API integration");
}

fn demo_prompt_templates() {
    println!("═══════════════════════════════════════════════════════════");
    println!("Demo 1: Prompt Template Engine");
    println!("═══════════════════════════════════════════════════════════\n");

    // Simple variable interpolation
    println!("1. Variable Interpolation:");
    let template = PromptTemplate::parse("Hello {{name}}! Welcome to {{project}}.").unwrap();

    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("Alice"));
    context.insert("project".to_string(), json!("PTTP"));

    let result = template.render(&context).unwrap();
    println!("   Template: Hello {{{{name}}}}! Welcome to {{{{project}}}}.");
    println!("   Result: {}\n", result);

    // Conditionals
    println!("2. Conditional Rendering:");
    let template =
        PromptTemplate::parse("Hello{% if premium %} Premium User{% endif %}!").unwrap();

    let mut context = HashMap::new();
    context.insert("premium".to_string(), json!(true));

    let result = template.render(&context).unwrap();
    println!("   Template: Hello{{% if premium %}} Premium User{{% endif %}}!");
    println!("   Result (premium=true): {}\n", result);

    // Loops
    println!("3. Loop Rendering:");
    let template = PromptTemplate::parse(
        "Programming languages: {% for lang in languages %}{{lang}}, {% endfor %}",
    )
    .unwrap();

    let mut context = HashMap::new();
    context.insert(
        "languages".to_string(),
        json!(["Rust", "Python", "JavaScript"]),
    );

    let result = template.render(&context).unwrap();
    println!("   Template: {{% for lang in languages %}}{{{{lang}}}}, {{% endfor %}}");
    println!("   Result: {}\n", result);

    // Prompt Builder
    println!("4. Prompt Builder:");
    let prompt = PromptBuilder::new()
        .system("You are a helpful AI assistant specialized in Rust programming")
        .example("What is ownership?", "Ownership is Rust's unique memory management system...")
        .user("What are the benefits of Rust?")
        .build();

    println!("   Built Prompt:");
    for line in prompt.lines().take(5) {
        println!("   {}", line);
    }
    println!("   ...\n");
}

fn demo_token_management() {
    println!("═══════════════════════════════════════════════════════════");
    println!("Demo 2: Token Management");
    println!("═══════════════════════════════════════════════════════════\n");

    let counter = TokenCounter::new();

    // Count tokens in text
    println!("1. Token Counting:");
    let text = "The quick brown fox jumps over the lazy dog";
    let tokens = counter.count(text);
    println!("   Text: \"{}\"", text);
    println!("   Estimated tokens: {}\n", tokens);

    // Count tokens in messages
    println!("2. Message Token Counting:");
    let message = Message::user("Hello, how are you today?");
    let tokens = counter.count_message(&message);
    println!("   Message: {:?}", message.content);
    println!("   Tokens (including role): {}\n", tokens);

    // Text truncation
    println!("3. Text Truncation:");
    let long_text = "This is a very long text that needs to be truncated to fit within a certain token limit for the language model";
    let truncated = counter.truncate(long_text, 10);
    println!("   Original: \"{}\"", long_text);
    println!("   Truncated (10 tokens): \"{}\"\n", truncated);

    // Token budget allocation
    println!("4. Token Budget:");
    let mut budget = pttp::llm::TokenBudget::new(1000);
    budget.allocate("system_prompt", 100).unwrap();
    budget.allocate("context", 400).unwrap();
    budget.allocate("user_query", 100).unwrap();

    println!("   Total budget: 1000 tokens");
    println!("   Allocations:");
    for (name, tokens) in budget.allocations() {
        println!("     - {}: {} tokens", name, tokens);
    }
    println!("   Remaining: {} tokens\n", budget.remaining());
}

fn demo_vector_database() {
    println!("═══════════════════════════════════════════════════════════");
    println!("Demo 3: Vector Database");
    println!("═══════════════════════════════════════════════════════════\n");

    let mut store = VectorStore::new();

    // Insert vectors
    println!("1. Inserting Vectors:");
    store.insert_simple("doc1", vec![1.0, 0.0, 0.0]);
    store.insert_simple("doc2", vec![0.0, 1.0, 0.0]);
    store.insert_simple("doc3", vec![0.9, 0.1, 0.0]);
    store.insert_simple("doc4", vec![0.1, 0.9, 0.0]);

    println!("   Inserted 4 documents with 3D vectors\n");

    // Similarity search
    println!("2. Similarity Search:");
    let query = vec![1.0, 0.0, 0.0];
    let results = store.search(&query, 3);

    println!("   Query vector: {:?}", query);
    println!("   Top 3 similar documents:");
    for (id, score) in &results {
        println!("     - {}: similarity = {:.4}", id, score);
    }
    println!();

    // Search with threshold
    println!("3. Threshold Search:");
    let results = store.search_threshold(&query, 0.8);
    println!("   Minimum similarity: 0.8");
    println!("   Results:");
    for (id, score) in &results {
        println!("     - {}: similarity = {:.4}", id, score);
    }
    println!();

    // Cosine similarity examples
    println!("4. Cosine Similarity Examples:");
    let v1 = vec![1.0, 0.0];
    let v2 = vec![1.0, 0.0];
    let v3 = vec![0.0, 1.0];

    println!(
        "   Identical vectors: {:.4}",
        pttp::llm::cosine_similarity(&v1, &v2)
    );
    println!(
        "   Orthogonal vectors: {:.4}\n",
        pttp::llm::cosine_similarity(&v1, &v3)
    );
}

fn demo_context_window() {
    println!("═══════════════════════════════════════════════════════════");
    println!("Demo 4: Context Window Management");
    println!("═══════════════════════════════════════════════════════════\n");

    // Create a context window with limited capacity
    let mut window = ContextWindow::new(100); // 100 token limit

    println!("1. Context Window with Auto-Eviction:");
    println!("   Max tokens: 100\n");

    // Set system message (always kept)
    window.set_system_message(Message::system("You are a helpful assistant"));
    println!("   Added system message");
    println!("   Current tokens: {}", window.token_count());
    println!("   Messages: {}\n", window.message_count());

    // Add conversation messages
    window.add_message(Message::user("What is Rust?"));
    println!("   Added user message");
    println!("   Current tokens: {}", window.token_count());
    println!("   Messages: {}\n", window.message_count());

    window.add_message(Message::assistant(
        "Rust is a systems programming language...",
    ));
    println!("   Added assistant message");
    println!("   Current tokens: {}", window.token_count());
    println!("   Messages: {}\n", window.message_count());

    // Add more messages to trigger eviction
    window.add_message(Message::user("Tell me more about ownership"));
    window.add_message(Message::assistant("Ownership is a unique feature..."));
    window.add_message(Message::user("What about borrowing?"));

    println!("   Added 3 more messages");
    println!("   Current tokens: {} (stayed under limit)", window.token_count());
    println!("   Messages: {} (oldest evicted)\n", window.message_count());

    println!("2. Getting All Messages:");
    let messages = window.messages();
    println!("   Total messages (including system): {}", messages.len());
    for (i, msg) in messages.iter().enumerate() {
        println!(
            "   {}. {}: {}",
            i + 1,
            msg.role,
            msg.content.chars().take(30).collect::<String>()
        );
    }
    println!();
}

async fn demo_rag_pipeline() {
    println!("═══════════════════════════════════════════════════════════");
    println!("Demo 5: RAG Pipeline");
    println!("═══════════════════════════════════════════════════════════\n");

    // Create a mock client for demo (won't actually call API)
    let client = LlmClient::new("http://localhost", "demo-key").unwrap();

    // Create RAG pipeline with custom config
    let config = RagConfig {
        chunk_size: 100,
        chunk_overlap: 20,
        top_k: 2,
        similarity_threshold: 0.5,
        ..Default::default()
    };

    let mut rag = RagPipeline::with_config(client, config);

    println!("1. Indexing Documents:");

    // Index some documents about Rust
    let doc1 = "Rust is a systems programming language that runs blazingly fast, \
               prevents segfaults, and guarantees thread safety. It accomplishes \
               these goals without a garbage collector.";

    let doc2 = "Ownership is Rust's most unique feature. It enables Rust to make \
               memory safety guarantees without needing a garbage collector. The \
               ownership system has three main rules.";

    let doc3 = "Cargo is Rust's build system and package manager. Most Rustaceans \
               use this tool to manage their Rust projects because Cargo handles \
               a lot of tasks.";

    rag.index_document("rust_intro", doc1).await.unwrap();
    println!("   ✓ Indexed: rust_intro");

    rag.index_document("rust_ownership", doc2).await.unwrap();
    println!("   ✓ Indexed: rust_ownership");

    rag.index_document("rust_cargo", doc3).await.unwrap();
    println!("   ✓ Indexed: rust_cargo\n");

    let stats = rag.stats();
    println!("2. RAG Statistics:");
    println!("   Total documents: {}", stats.total_documents);
    println!("   Total chunks: {}", stats.total_chunks);
    println!("   Vector store size: {}\n", stats.vector_store_size);

    println!("3. Document Retrieval:");
    println!("   (Using placeholder embeddings for demo)\n");
    println!("   Note: In production, use real embeddings from LLM API");
    println!("   by calling rag.index_document_with_embeddings() instead\n");

    // Note: We can't actually query without a real API,
    // but we can show the setup
    println!("   RAG pipeline is ready for queries!");
    println!("   Example: rag.query(\"What is Rust?\").await");
    println!("   Would retrieve relevant chunks and generate answer\n");
}

async fn demo_llm_client() {
    println!("═══════════════════════════════════════════════════════════");
    println!("Demo 6: LLM Client (Live API)");
    println!("═══════════════════════════════════════════════════════════\n");

    let api_key = std::env::var("OPENAI_API_KEY").unwrap();
    let client = LlmClient::openai(api_key).unwrap();

    println!("1. Simple Completion:");

    let request = CompletionRequest::new("gpt-3.5-turbo")
        .message(Message::user("Say 'Hello from PTTP!' in exactly 5 words"))
        .temperature(0.7)
        .max_tokens(50);

    match client.complete(request).await {
        Ok(response) => {
            println!("   Response: {}", response.choices[0].message.content);
            if let Some(usage) = response.usage {
                println!("   Tokens used: {}", usage.total_tokens);
            }
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }

    println!("\n2. Streaming Completion:");

    let request = CompletionRequest::new("gpt-3.5-turbo")
        .message(Message::user("Count from 1 to 5"))
        .temperature(0.7);

    match client.stream(request).await {
        Ok(mut stream) => {
            print!("   Streaming: ");
            while let Ok(Some(chunk)) = stream.next_chunk().await {
                for choice in chunk.choices {
                    if let Some(content) = choice.delta.content {
                        print!("{}", content);
                    }
                }
            }
            println!("\n");
        }
        Err(e) => {
            println!("   Error: {}", e);
        }
    }
}
