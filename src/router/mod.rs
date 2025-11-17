//! Request routing and path matching
//!
//! This module provides:
//! - Pattern matching (exact, prefix, wildcards)
//! - Path parameter extraction
//! - Method-based routing

use crate::context::{Context, Params};
use crate::http::{Method, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for async handler functions
pub type Handler = Arc<
    dyn Fn(Context) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static,
>;

/// Route pattern types
#[derive(Debug, Clone)]
enum Pattern {
    /// Exact match: /users
    Exact(String),
    /// Parameterized: /users/:id
    Parameterized {
        segments: Vec<Segment>,
    },
    /// Wildcard: /files/*
    Wildcard(String),
}

/// Segment in a parameterized route
#[derive(Debug, Clone)]
enum Segment {
    /// Static segment
    Static(String),
    /// Parameter segment (name)
    Param(String),
}

impl Pattern {
    /// Parse a pattern string into a Pattern
    fn parse(pattern: &str) -> Self {
        // Remove trailing slash unless it's the root
        let pattern = if pattern != "/" && pattern.ends_with('/') {
            &pattern[..pattern.len() - 1]
        } else {
            pattern
        };

        // Check for wildcard
        if let Some(prefix) = pattern.strip_suffix("/*") {
            return Pattern::Wildcard(prefix.to_string());
        }

        // Check for parameters
        if pattern.contains(':') {
            let segments: Vec<Segment> = pattern
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    if let Some(param_name) = s.strip_prefix(':') {
                        Segment::Param(param_name.to_string())
                    } else {
                        Segment::Static(s.to_string())
                    }
                })
                .collect();

            return Pattern::Parameterized { segments };
        }

        // Exact match
        Pattern::Exact(pattern.to_string())
    }

    /// Match a path against this pattern and extract parameters
    fn matches(&self, path: &str) -> Option<Params> {
        // Normalize path (remove trailing slash unless root)
        let path = if path != "/" && path.ends_with('/') {
            &path[..path.len() - 1]
        } else {
            path
        };

        match self {
            Pattern::Exact(pattern) => {
                if path == pattern {
                    Some(Params::new())
                } else {
                    None
                }
            }
            Pattern::Parameterized { segments } => {
                let path_segments: Vec<&str> =
                    path.split('/').filter(|s| !s.is_empty()).collect();

                // Must have same number of segments
                if path_segments.len() != segments.len() {
                    return None;
                }

                let mut params = Params::new();

                for (i, segment) in segments.iter().enumerate() {
                    match segment {
                        Segment::Static(s) => {
                            if path_segments[i] != s {
                                return None;
                            }
                        }
                        Segment::Param(name) => {
                            params.insert(name.clone(), path_segments[i].to_string());
                        }
                    }
                }

                Some(params)
            }
            Pattern::Wildcard(prefix) => {
                if prefix.is_empty() {
                    // Matches everything
                    Some(Params::new())
                } else if path == prefix || path.starts_with(&format!("{}/", prefix)) {
                    Some(Params::new())
                } else {
                    None
                }
            }
        }
    }
}

/// A single route
struct Route {
    method: Method,
    pattern: Pattern,
    handler: Handler,
}

impl Route {
    fn new(method: Method, pattern: &str, handler: Handler) -> Self {
        Self {
            method,
            pattern: Pattern::parse(pattern),
            handler,
        }
    }

    /// Check if this route matches the request
    fn matches(&self, method: &Method, path: &str) -> Option<Params> {
        if &self.method == method {
            self.pattern.matches(path)
        } else {
            None
        }
    }
}

/// Router for handling HTTP requests
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a GET route
    pub fn get<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.add_route(Method::GET, path, handler);
    }

    /// Add a POST route
    pub fn post<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.add_route(Method::POST, path, handler);
    }

    /// Add a PUT route
    pub fn put<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.add_route(Method::PUT, path, handler);
    }

    /// Add a DELETE route
    pub fn delete<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.add_route(Method::DELETE, path, handler);
    }

    /// Add a PATCH route
    pub fn patch<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.add_route(Method::PATCH, path, handler);
    }

    /// Generic route addition
    fn add_route<F, Fut>(&mut self, method: Method, path: &str, handler: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = Arc::new(move |ctx: Context| -> Pin<Box<dyn Future<Output = Response> + Send>> {
            Box::pin(handler(ctx))
        });

        self.routes.push(Route::new(method, path, handler));
    }

    /// Route a request to the appropriate handler
    pub async fn route(&self, request: Request) -> Response {
        // Extract path without query string
        let path = request
            .uri()
            .split('?')
            .next()
            .unwrap_or(request.uri());

        // Find matching route
        for route in &self.routes {
            if let Some(params) = route.matches(request.method(), path) {
                let ctx = Context::with_params(request, params);
                return (route.handler)(ctx).await;
            }
        }

        // No route found
        Response::not_found().with_body(b"404 - Not Found".to_vec())
    }

    /// Get the number of routes
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if the router has no routes
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::StatusCode;

    #[test]
    fn test_pattern_parse_exact() {
        let pattern = Pattern::parse("/users");
        match pattern {
            Pattern::Exact(s) => assert_eq!(s, "/users"),
            _ => panic!("Expected Exact pattern"),
        }
    }

    #[test]
    fn test_pattern_parse_parameterized() {
        let pattern = Pattern::parse("/users/:id");
        match pattern {
            Pattern::Parameterized { segments } => {
                assert_eq!(segments.len(), 2);
            }
            _ => panic!("Expected Parameterized pattern"),
        }
    }

    #[test]
    fn test_pattern_parse_wildcard() {
        let pattern = Pattern::parse("/files/*");
        match pattern {
            Pattern::Wildcard(s) => assert_eq!(s, "/files"),
            _ => panic!("Expected Wildcard pattern"),
        }
    }

    #[test]
    fn test_pattern_exact_match() {
        let pattern = Pattern::parse("/users");
        assert!(pattern.matches("/users").is_some());
        assert!(pattern.matches("/users/").is_some()); // Trailing slash normalized
        assert!(pattern.matches("/users/123").is_none());
    }

    #[test]
    fn test_pattern_parameterized_match() {
        let pattern = Pattern::parse("/users/:id");
        let params = pattern.matches("/users/123").unwrap();
        assert_eq!(params.get("id"), Some("123"));

        assert!(pattern.matches("/users").is_none());
        assert!(pattern.matches("/users/123/posts").is_none());
    }

    #[test]
    fn test_pattern_multiple_params() {
        let pattern = Pattern::parse("/users/:user_id/posts/:post_id");
        let params = pattern.matches("/users/42/posts/100").unwrap();
        assert_eq!(params.get("user_id"), Some("42"));
        assert_eq!(params.get("post_id"), Some("100"));
    }

    #[test]
    fn test_pattern_wildcard_match() {
        let pattern = Pattern::parse("/files/*");
        assert!(pattern.matches("/files/").is_some());
        assert!(pattern.matches("/files/doc.txt").is_some());
        assert!(pattern.matches("/files/images/photo.jpg").is_some());
        assert!(pattern.matches("/other").is_none());
    }

    #[tokio::test]
    async fn test_router_get() {
        let mut router = Router::new();

        router.get("/test", |_ctx| async { Response::text("Hello") });

        let request = Request::new(Method::GET, "/test".to_string());
        let response = router.route(request).await;

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.body(), b"Hello");
    }

    #[tokio::test]
    async fn test_router_with_params() {
        let mut router = Router::new();

        router.get("/users/:id", |ctx| async move {
            let id = ctx.param("id").unwrap_or("unknown");
            Response::text(format!("User ID: {}", id))
        });

        let request = Request::new(Method::GET, "/users/123".to_string());
        let response = router.route(request).await;

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.body(), b"User ID: 123");
    }

    #[tokio::test]
    async fn test_router_method_mismatch() {
        let mut router = Router::new();

        router.get("/test", |_ctx| async { Response::text("Hello") });

        let request = Request::new(Method::POST, "/test".to_string());
        let response = router.route(request).await;

        assert_eq!(response.status(), StatusCode::NotFound);
    }

    #[tokio::test]
    async fn test_router_not_found() {
        let router = Router::new();

        let request = Request::new(Method::GET, "/nonexistent".to_string());
        let response = router.route(request).await;

        assert_eq!(response.status(), StatusCode::NotFound);
    }

    #[tokio::test]
    async fn test_router_wildcard() {
        let mut router = Router::new();

        router.get("/files/*", |_ctx| async { Response::text("File handler") });

        let request = Request::new(Method::GET, "/files/images/photo.jpg".to_string());
        let response = router.route(request).await;

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.body(), b"File handler");
    }

    #[tokio::test]
    async fn test_router_multiple_methods() {
        let mut router = Router::new();

        router.get("/resource", |_ctx| async { Response::text("GET") });
        router.post("/resource", |_ctx| async { Response::text("POST") });

        let get_req = Request::new(Method::GET, "/resource".to_string());
        let get_resp = router.route(get_req).await;
        assert_eq!(get_resp.body(), b"GET");

        let post_req = Request::new(Method::POST, "/resource".to_string());
        let post_resp = router.route(post_req).await;
        assert_eq!(post_resp.body(), b"POST");
    }
}
