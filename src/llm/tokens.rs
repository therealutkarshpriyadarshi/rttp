//! Token management and context window tracking
//!
//! Provides utilities for counting tokens, managing context windows,
//! and truncating text to fit within token limits.

use std::collections::VecDeque;

use crate::llm::client::Message;

/// Simple token counter using word-based estimation
///
/// Note: This is a simplified approximation. For production use with
/// specific models, consider using tiktoken-rs for accurate tokenization.
pub struct TokenCounter {
    /// Approximate tokens per word (GPT models use ~1.3 tokens/word on average)
    tokens_per_word: f32,
}

impl TokenCounter {
    /// Create a new token counter
    pub fn new() -> Self {
        Self {
            tokens_per_word: 1.3,
        }
    }

    /// Create a token counter with custom ratio
    pub fn with_ratio(tokens_per_word: f32) -> Self {
        Self { tokens_per_word }
    }

    /// Count tokens in text (approximation)
    ///
    /// This uses a simple word-based approximation:
    /// - Splits text by whitespace
    /// - Multiplies word count by average tokens per word
    /// - Adds overhead for special characters
    pub fn count(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        let base_tokens = (words as f32 * self.tokens_per_word) as usize;

        // Add small overhead for special characters and punctuation
        let special_chars = text.chars().filter(|c| c.is_ascii_punctuation()).count();
        let overhead = (special_chars as f32 * 0.3) as usize;

        base_tokens + overhead
    }

    /// Count tokens in a message
    pub fn count_message(&self, message: &Message) -> usize {
        // Account for role prefix (approximately 4 tokens)
        let role_tokens = 4;
        let content_tokens = self.count(&message.content);
        role_tokens + content_tokens
    }

    /// Count total tokens in multiple messages
    pub fn count_messages(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.count_message(m)).sum()
    }

    /// Truncate text to fit within token limit
    pub fn truncate(&self, text: &str, max_tokens: usize) -> String {
        let current_tokens = self.count(text);

        if current_tokens <= max_tokens {
            return text.to_string();
        }

        // Estimate how many words we can keep
        let target_words = (max_tokens as f32 / self.tokens_per_word) as usize;
        let words: Vec<&str> = text.split_whitespace().collect();

        if target_words >= words.len() {
            return text.to_string();
        }

        words[..target_words].join(" ") + "..."
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Context window for managing conversation history
///
/// Automatically manages messages to stay within token limits,
/// evicting oldest messages when necessary.
pub struct ContextWindow {
    /// Messages in the window
    messages: VecDeque<Message>,
    /// Maximum tokens allowed
    max_tokens: usize,
    /// Current token count
    current_tokens: usize,
    /// Token counter
    counter: TokenCounter,
    /// System message (always kept)
    system_message: Option<Message>,
}

impl ContextWindow {
    /// Create a new context window
    pub fn new(max_tokens: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_tokens,
            current_tokens: 0,
            counter: TokenCounter::new(),
            system_message: None,
        }
    }

    /// Set the system message (always kept at the beginning)
    pub fn set_system_message(&mut self, message: Message) {
        let tokens = self.counter.count_message(&message);
        self.system_message = Some(message);
        self.current_tokens += tokens;
        self.ensure_capacity();
    }

    /// Add a message to the context window
    pub fn add_message(&mut self, message: Message) {
        let tokens = self.counter.count_message(&message);
        self.current_tokens += tokens;
        self.messages.push_back(message);
        self.ensure_capacity();
    }

    /// Add multiple messages
    pub fn add_messages(&mut self, messages: Vec<Message>) {
        for message in messages {
            self.add_message(message);
        }
    }

    /// Ensure we're within token capacity by evicting oldest messages
    fn ensure_capacity(&mut self) {
        while self.current_tokens > self.max_tokens && !self.messages.is_empty() {
            if let Some(oldest) = self.messages.pop_front() {
                let tokens = self.counter.count_message(&oldest);
                self.current_tokens = self.current_tokens.saturating_sub(tokens);
            }
        }
    }

    /// Get all messages (including system message if set)
    pub fn messages(&self) -> Vec<Message> {
        let mut result = Vec::new();

        if let Some(sys_msg) = &self.system_message {
            result.push(sys_msg.clone());
        }

        result.extend(self.messages.iter().cloned());
        result
    }

    /// Get current token count
    pub fn token_count(&self) -> usize {
        self.current_tokens
    }

    /// Get remaining token capacity
    pub fn remaining_capacity(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_tokens)
    }

    /// Clear all messages (except system message)
    pub fn clear(&mut self) {
        let system_tokens = self
            .system_message
            .as_ref()
            .map(|m| self.counter.count_message(m))
            .unwrap_or(0);

        self.messages.clear();
        self.current_tokens = system_tokens;
    }

    /// Get the number of messages (excluding system message)
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

/// Token budget allocator for splitting available tokens
///
/// Useful for RAG applications where you need to allocate tokens
/// between context, retrieved documents, and completion.
pub struct TokenBudget {
    /// Total available tokens
    total: usize,
    /// Tokens allocated to different purposes
    allocations: Vec<(String, usize)>,
}

impl TokenBudget {
    /// Create a new token budget
    pub fn new(total: usize) -> Self {
        Self {
            total,
            allocations: Vec::new(),
        }
    }

    /// Allocate tokens for a specific purpose
    pub fn allocate(&mut self, name: impl Into<String>, tokens: usize) -> Result<(), String> {
        let remaining = self.remaining();
        if tokens > remaining {
            return Err(format!(
                "Cannot allocate {} tokens, only {} remaining",
                tokens, remaining
            ));
        }

        self.allocations.push((name.into(), tokens));
        Ok(())
    }

    /// Get remaining unallocated tokens
    pub fn remaining(&self) -> usize {
        let allocated: usize = self.allocations.iter().map(|(_, t)| t).sum();
        self.total.saturating_sub(allocated)
    }

    /// Get allocation for a specific purpose
    pub fn get_allocation(&self, name: &str) -> Option<usize> {
        self.allocations
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| *t)
    }

    /// Get all allocations
    pub fn allocations(&self) -> &[(String, usize)] {
        &self.allocations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let counter = TokenCounter::new();

        // Simple text
        let tokens = counter.count("Hello world");
        assert!(tokens > 0 && tokens < 10);

        // Longer text
        let text = "The quick brown fox jumps over the lazy dog";
        let tokens = counter.count(text);
        assert!(tokens > 5 && tokens < 20);
    }

    #[test]
    fn test_message_counting() {
        let counter = TokenCounter::new();
        let msg = Message::user("Hello, how are you?");
        let tokens = counter.count_message(&msg);

        // Should include role overhead
        assert!(tokens > counter.count("Hello, how are you?"));
    }

    #[test]
    fn test_truncation() {
        let counter = TokenCounter::new();
        let text = "The quick brown fox jumps over the lazy dog and continues running";

        let truncated = counter.truncate(text, 5);
        assert!(truncated.len() < text.len());
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_context_window_basic() {
        let mut window = ContextWindow::new(100);

        window.add_message(Message::user("Hello"));
        assert_eq!(window.message_count(), 1);

        window.add_message(Message::assistant("Hi there"));
        assert_eq!(window.message_count(), 2);

        let messages = window.messages();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_context_window_eviction() {
        // Very small window to force eviction
        let mut window = ContextWindow::new(20);

        window.add_message(Message::user("First message"));
        window.add_message(Message::user("Second message"));
        window.add_message(Message::user("Third message"));

        // Should have evicted some messages to stay under limit
        assert!(window.token_count() <= 20);
    }

    #[test]
    fn test_context_window_system_message() {
        let mut window = ContextWindow::new(100);

        window.set_system_message(Message::system("You are helpful"));
        window.add_message(Message::user("Hello"));

        let messages = window.messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn test_context_window_clear() {
        let mut window = ContextWindow::new(100);

        window.set_system_message(Message::system("You are helpful"));
        window.add_message(Message::user("Hello"));
        window.add_message(Message::user("Hi"));

        window.clear();

        let messages = window.messages();
        assert_eq!(messages.len(), 1); // Only system message remains
        assert_eq!(messages[0].role, "system");
    }

    #[test]
    fn test_token_budget() {
        let mut budget = TokenBudget::new(1000);

        budget.allocate("context", 300).unwrap();
        budget.allocate("documents", 500).unwrap();

        assert_eq!(budget.remaining(), 200);
        assert_eq!(budget.get_allocation("context"), Some(300));
        assert_eq!(budget.get_allocation("documents"), Some(500));
    }

    #[test]
    fn test_token_budget_overflow() {
        let mut budget = TokenBudget::new(100);

        budget.allocate("first", 60).unwrap();
        let result = budget.allocate("second", 50);

        assert!(result.is_err());
    }
}
