//! CSRF (Cross-Site Request Forgery) protection
//!
//! Provides token generation and validation to prevent CSRF attacks.
//!
//! # Example
//! ```rust,ignore
//! use pttp::security::csrf::{CsrfProtection, CsrfToken};
//!
//! let csrf = CsrfProtection::new();
//! let token = csrf.generate_token();
//! assert!(csrf.validate_token(&token));
//! ```

use crate::context::Context;
use crate::http::{Method, Response, StatusCode};
use crate::middleware::{Middleware, Next};
use rand::Rng;
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// CSRF token
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CsrfToken {
    value: String,
    expires_at: SystemTime,
}

impl CsrfToken {
    /// Create a new CSRF token
    fn new(value: String, ttl: Duration) -> Self {
        let expires_at = SystemTime::now() + ttl;
        Self { value, expires_at }
    }

    /// Check if the token is expired
    fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    /// Get the token value
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// CSRF protection handler
#[derive(Clone)]
pub struct CsrfProtection {
    tokens: Arc<RwLock<HashSet<String>>>,
    token_ttl: Duration,
    header_name: String,
    form_field: String,
}

impl CsrfProtection {
    /// Create a new CSRF protection handler with default settings
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashSet::new())),
            token_ttl: Duration::from_secs(3600), // 1 hour
            header_name: "X-CSRF-Token".to_string(),
            form_field: "csrf_token".to_string(),
        }
    }

    /// Set custom token TTL (time-to-live)
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.token_ttl = ttl;
        self
    }

    /// Set custom header name for CSRF token
    pub fn with_header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }

    /// Set custom form field name for CSRF token
    pub fn with_form_field(mut self, field: impl Into<String>) -> Self {
        self.form_field = field.into();
        self
    }

    /// Generate a new CSRF token
    pub async fn generate_token(&self) -> CsrfToken {
        let token_value = Self::generate_random_token();
        let token = CsrfToken::new(token_value.clone(), self.token_ttl);

        let mut tokens = self.tokens.write().await;
        tokens.insert(token_value);

        token
    }

    /// Validate a CSRF token
    pub async fn validate_token(&self, token_value: &str) -> bool {
        let tokens = self.tokens.read().await;
        tokens.contains(token_value)
    }

    /// Invalidate a CSRF token (e.g., after use)
    pub async fn invalidate_token(&self, token_value: &str) {
        let mut tokens = self.tokens.write().await;
        tokens.remove(token_value);
    }

    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) {
        // For simplicity, we don't track expiration per token in this implementation
        // In production, you'd store tokens with their expiration times
        // and remove expired ones periodically
    }

    /// Extract CSRF token from request (header or form field)
    fn extract_token(&self, ctx: &Context) -> Option<String> {
        // Try to get token from header first
        if let Some(header_value) = ctx.request().headers().get(&self.header_name)
            .or_else(|| ctx.request().headers().get(&self.header_name.to_lowercase())) {
            return Some(header_value.clone());
        }

        // TODO: In a full implementation, you'd also check form data
        // For now, we only support header-based tokens

        None
    }

    /// Generate a random token string
    fn generate_random_token() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &random_bytes)
    }
}

impl Default for CsrfProtection {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware that enforces CSRF protection
pub struct CsrfMiddleware {
    csrf: CsrfProtection,
    safe_methods: HashSet<Method>,
}

impl CsrfMiddleware {
    /// Create new CSRF middleware
    pub fn new(csrf: CsrfProtection) -> Self {
        let mut safe_methods = HashSet::new();
        safe_methods.insert(Method::GET);
        safe_methods.insert(Method::HEAD);
        safe_methods.insert(Method::OPTIONS);

        Self { csrf, safe_methods }
    }

    /// Check if the request method is safe (doesn't require CSRF protection)
    fn is_safe_method(&self, method: &Method) -> bool {
        self.safe_methods.contains(method)
    }
}

impl Middleware for CsrfMiddleware {
    fn handle(&self, ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let csrf = self.csrf.clone();
        let is_safe = self.is_safe_method(ctx.request().method());

        Box::pin(async move {
            // Safe methods (GET, HEAD, OPTIONS) don't need CSRF protection
            if is_safe {
                return next.run(ctx).await;
            }

            // Extract and validate CSRF token
            let token = match csrf.extract_token(&ctx) {
                Some(token) => token,
                None => {
                    tracing::warn!("CSRF token missing from request");
                    return Response::builder()
                        .status(StatusCode::Forbidden)
                        .header("Content-Type", "application/json")
                        .body(r#"{"error":"CSRF token missing"}"#)
                        .unwrap();
                }
            };

            if !csrf.validate_token(&token).await {
                tracing::warn!("Invalid CSRF token: {}", token);
                return Response::builder()
                    .status(StatusCode::Forbidden)
                    .header("Content-Type", "application/json")
                    .body(r#"{"error":"Invalid CSRF token"}"#)
                    .unwrap();
            }

            // Optionally invalidate single-use tokens
            // csrf.invalidate_token(&token).await;

            next.run(ctx).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_and_validate_token() {
        let csrf = CsrfProtection::new();
        let token = csrf.generate_token().await;

        assert!(csrf.validate_token(token.value()).await);
    }

    #[tokio::test]
    async fn test_validate_invalid_token() {
        let csrf = CsrfProtection::new();

        assert!(!csrf.validate_token("invalid-token").await);
    }

    #[tokio::test]
    async fn test_invalidate_token() {
        let csrf = CsrfProtection::new();
        let token = csrf.generate_token().await;

        assert!(csrf.validate_token(token.value()).await);

        csrf.invalidate_token(token.value()).await;

        assert!(!csrf.validate_token(token.value()).await);
    }

    #[tokio::test]
    async fn test_multiple_tokens() {
        let csrf = CsrfProtection::new();

        let token1 = csrf.generate_token().await;
        let token2 = csrf.generate_token().await;

        assert_ne!(token1.value(), token2.value());
        assert!(csrf.validate_token(token1.value()).await);
        assert!(csrf.validate_token(token2.value()).await);
    }

    #[test]
    fn test_custom_ttl() {
        let csrf = CsrfProtection::new().with_ttl(Duration::from_secs(7200));

        assert_eq!(csrf.token_ttl, Duration::from_secs(7200));
    }

    #[test]
    fn test_custom_header_name() {
        let csrf = CsrfProtection::new().with_header_name("X-Custom-CSRF");

        assert_eq!(csrf.header_name, "X-Custom-CSRF");
    }

    #[test]
    fn test_custom_form_field() {
        let csrf = CsrfProtection::new().with_form_field("_csrf");

        assert_eq!(csrf.form_field, "_csrf");
    }

    #[tokio::test]
    async fn test_extract_token_from_header() {
        use crate::http::{Method, Request};
        let csrf = CsrfProtection::new();
        let mut req = Request::new(Method::POST, "/test".to_string());

        let token = csrf.generate_token().await;
        req.headers_mut()
            .insert("x-csrf-token".to_string(), token.value().to_string());

        let ctx = Context::new(req);
        let extracted = csrf.extract_token(&ctx);
        assert_eq!(extracted.as_deref(), Some(token.value()));
    }

    #[test]
    fn test_extract_token_missing() {
        use crate::http::{Method, Request};
        let csrf = CsrfProtection::new();
        let req = Request::new(Method::POST, "/test".to_string());

        let ctx = Context::new(req);
        let extracted = csrf.extract_token(&ctx);
        assert!(extracted.is_none());
    }

    #[test]
    fn test_is_safe_method() {
        let csrf = CsrfProtection::new();
        let middleware = CsrfMiddleware::new(csrf);

        assert!(middleware.is_safe_method(&Method::GET));
        assert!(middleware.is_safe_method(&Method::HEAD));
        assert!(middleware.is_safe_method(&Method::OPTIONS));
        assert!(!middleware.is_safe_method(&Method::POST));
        assert!(!middleware.is_safe_method(&Method::PUT));
        assert!(!middleware.is_safe_method(&Method::DELETE));
    }

    #[test]
    fn test_token_is_expired() {
        let expired_token = CsrfToken::new("test".to_string(), Duration::from_secs(0));
        std::thread::sleep(Duration::from_millis(10));
        assert!(expired_token.is_expired());

        let valid_token = CsrfToken::new("test".to_string(), Duration::from_secs(3600));
        assert!(!valid_token.is_expired());
    }
}
