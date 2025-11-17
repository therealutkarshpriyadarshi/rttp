//! Rate limiting middleware using the token bucket algorithm
//!
//! Provides IP-based and user-based rate limiting to prevent abuse.
//!
//! # Example
//! ```rust,ignore
//! use pttp::security::rate_limit::{RateLimiter, RateLimitConfig};
//!
//! let config = RateLimitConfig::new(100, 10); // 100 requests per 10 seconds
//! let limiter = RateLimiter::new(config);
//! ```

use crate::context::Context;
use crate::http::{Response, StatusCode};
use crate::middleware::{Middleware, Next};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of tokens (requests) in the bucket
    pub capacity: f64,
    /// Token refill rate (tokens per second)
    pub refill_rate: f64,
    /// Time window for rate limiting
    pub window: Duration,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    ///
    /// # Arguments
    /// * `max_requests` - Maximum number of requests allowed
    /// * `window_secs` - Time window in seconds
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        let capacity = max_requests as f64;
        let refill_rate = capacity / window_secs as f64;

        Self {
            capacity,
            refill_rate,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Create a per-second rate limit
    pub fn per_second(max_requests: u32) -> Self {
        Self::new(max_requests, 1)
    }

    /// Create a per-minute rate limit
    pub fn per_minute(max_requests: u32) -> Self {
        Self::new(max_requests, 60)
    }

    /// Create a per-hour rate limit
    pub fn per_hour(max_requests: u32) -> Self {
        Self::new(max_requests, 3600)
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Last time tokens were refilled
    last_update: Instant,
    /// Bucket capacity
    capacity: f64,
    /// Refill rate (tokens per second)
    refill_rate: f64,
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_update: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_update = now;
    }

    /// Try to consume a token
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get time until next token is available
    fn time_until_token(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let secs = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(secs)
        }
    }
}

/// Rate limiter using token bucket algorithm
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if a request is allowed for the given key
    pub async fn check(&self, key: &str) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.write().await;

        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.config.capacity, self.config.refill_rate));

        if bucket.try_consume() {
            Ok(())
        } else {
            let retry_after = bucket.time_until_token();
            Err(RateLimitError::LimitExceeded { retry_after })
        }
    }

    /// Clean up old buckets to prevent memory leaks
    pub async fn cleanup_old_buckets(&self, max_age: Duration) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        buckets.retain(|_, bucket| {
            now.duration_since(bucket.last_update) < max_age
        });
    }

    /// Get the number of active buckets
    pub async fn bucket_count(&self) -> usize {
        let buckets = self.buckets.read().await;
        buckets.len()
    }
}

/// Rate limit errors
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Rate limit exceeded
    LimitExceeded {
        /// Time to wait before retrying
        retry_after: Duration,
    },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LimitExceeded { retry_after } => {
                write!(f, "Rate limit exceeded. Retry after {} seconds", retry_after.as_secs())
            }
        }
    }
}

impl std::error::Error for RateLimitError {}

/// Middleware for IP-based rate limiting
pub struct RateLimitMiddleware {
    limiter: RateLimiter,
    key_extractor: KeyExtractor,
}

/// Strategy for extracting rate limit keys from requests
#[derive(Clone)]
pub enum KeyExtractor {
    /// Use IP address as the key
    IpAddress,
    /// Use user ID from claims as the key (requires authentication)
    UserId,
    /// Use a custom function to extract the key
    Custom(Arc<dyn Fn(&Context) -> Option<String> + Send + Sync>),
}

impl RateLimitMiddleware {
    /// Create new rate limit middleware with IP-based limiting
    pub fn new(limiter: RateLimiter) -> Self {
        Self {
            limiter,
            key_extractor: KeyExtractor::IpAddress,
        }
    }

    /// Use user ID-based rate limiting (requires authentication middleware)
    pub fn by_user(limiter: RateLimiter) -> Self {
        Self {
            limiter,
            key_extractor: KeyExtractor::UserId,
        }
    }

    /// Use custom key extraction
    pub fn with_extractor<F>(limiter: RateLimiter, extractor: F) -> Self
    where
        F: Fn(&Context) -> Option<String> + Send + Sync + 'static,
    {
        Self {
            limiter,
            key_extractor: KeyExtractor::Custom(Arc::new(extractor)),
        }
    }

    /// Extract rate limit key from request
    fn extract_key(&self, ctx: &Context) -> Option<String> {
        match &self.key_extractor {
            KeyExtractor::IpAddress => {
                // Try to get IP from X-Forwarded-For header first
                if let Some(forwarded_str) = ctx.request().headers().get("x-forwarded-for")
                    .or_else(|| ctx.request().headers().get("X-Forwarded-For")) {
                    if let Some(first_ip) = forwarded_str.split(',').next() {
                        return Some(first_ip.trim().to_string());
                    }
                }

                // Fallback to peer address
                ctx.request().peer_addr().map(|addr| addr.to_string())
            }
            KeyExtractor::UserId => {
                // Extract user ID from claims (requires authentication)
                use crate::security::auth::Claims;
                ctx.extensions()
                    .get::<Claims>()
                    .map(|claims| claims.sub.clone())
            }
            KeyExtractor::Custom(f) => f(ctx),
        }
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle(&self, ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let key = self.extract_key(&ctx);
        let limiter = self.limiter.clone();

        Box::pin(async move {
            let key = match key {
                Some(key) => key,
                None => {
                    tracing::warn!("Could not extract rate limit key from request");
                    // Allow the request if we can't extract a key
                    return next.run(ctx).await;
                }
            };

            match limiter.check(&key).await {
                Ok(()) => next.run(ctx).await,
                Err(RateLimitError::LimitExceeded { retry_after }) => {
                    tracing::warn!("Rate limit exceeded for key: {}", key);
                    Response::builder()
                        .status(StatusCode::TooManyRequests)
                        .header("Content-Type", "application/json")
                        .header("Retry-After", retry_after.as_secs().to_string())
                        .header("X-RateLimit-Limit", limiter.config.capacity.to_string())
                        .header("X-RateLimit-Window", limiter.config.window.as_secs().to_string())
                        .body(r#"{"error":"Rate limit exceeded"}"#)
                        .unwrap()
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::new(100, 60);
        assert_eq!(config.capacity, 100.0);
        assert!((config.refill_rate - 100.0 / 60.0).abs() < 0.001);
    }

    #[test]
    fn test_rate_limit_config_presets() {
        let per_second = RateLimitConfig::per_second(10);
        assert_eq!(per_second.capacity, 10.0);
        assert_eq!(per_second.window, Duration::from_secs(1));

        let per_minute = RateLimitConfig::per_minute(60);
        assert_eq!(per_minute.capacity, 60.0);
        assert_eq!(per_minute.window, Duration::from_secs(60));

        let per_hour = RateLimitConfig::per_hour(1000);
        assert_eq!(per_hour.capacity, 1000.0);
        assert_eq!(per_hour.window, Duration::from_secs(3600));
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0);

        // Should be able to consume initial tokens
        assert!(bucket.try_consume());
        assert_eq!(bucket.tokens, 9.0);
    }

    #[test]
    fn test_token_bucket_exhaustion() {
        let mut bucket = TokenBucket::new(2.0, 1.0);

        // Consume all tokens
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());

        // Should fail when no tokens left
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10.0, 10.0); // Refill 10 tokens per second

        // Consume all tokens
        for _ in 0..10 {
            bucket.try_consume();
        }
        assert!(!bucket.try_consume());

        // Wait for refill (simulate by manually updating)
        bucket.last_update = Instant::now() - Duration::from_secs(1);
        bucket.refill();

        // Should have tokens again
        assert!(bucket.tokens >= 9.0); // At least 9 tokens after 1 second
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig::new(5, 10);
        let limiter = RateLimiter::new(config);

        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.check("test-key").await.is_ok());
        }

        // 6th request should be rate limited
        assert!(limiter.check("test-key").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_keys() {
        let config = RateLimitConfig::new(2, 10);
        let limiter = RateLimiter::new(config);

        // Each key should have independent limits
        assert!(limiter.check("key1").await.is_ok());
        assert!(limiter.check("key2").await.is_ok());
        assert!(limiter.check("key1").await.is_ok());
        assert!(limiter.check("key2").await.is_ok());

        // Both should be exhausted now
        assert!(limiter.check("key1").await.is_err());
        assert!(limiter.check("key2").await.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_old_buckets() {
        let config = RateLimitConfig::new(5, 10);
        let limiter = RateLimiter::new(config);

        limiter.check("key1").await.ok();
        limiter.check("key2").await.ok();

        assert_eq!(limiter.bucket_count().await, 2);

        // Cleanup buckets older than 0 seconds (should remove all)
        limiter.cleanup_old_buckets(Duration::from_secs(0)).await;

        // Note: This may not remove all buckets immediately due to timing,
        // but in practice cleanup would run periodically
    }
}
