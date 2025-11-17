//! Middleware system
//!
//! This module provides:
//! - Middleware trait definition
//! - Middleware chaining (onion model)
//! - Built-in middleware (logging, CORS, etc.)

use crate::context::Context;
use crate::http::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

/// Type alias for middleware handler functions
pub type MiddlewareHandler = Arc<
    dyn Fn(Context, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static,
>;

/// Next middleware in the chain
pub struct Next {
    middlewares: Vec<MiddlewareHandler>,
    index: usize,
}

impl Next {
    /// Create a new Next with a list of middlewares
    pub fn new(middlewares: Vec<MiddlewareHandler>) -> Self {
        Self {
            middlewares,
            index: 0,
        }
    }

    /// Run the next middleware in the chain
    pub async fn run(mut self, ctx: Context) -> Response {
        if self.index < self.middlewares.len() {
            let middleware = self.middlewares[self.index].clone();
            self.index += 1;
            middleware(ctx, self).await
        } else {
            // No more middlewares, return a default response
            // This should not happen if the chain is properly constructed
            Response::not_found()
        }
    }
}

/// Middleware trait for request/response processing
pub trait Middleware: Send + Sync {
    /// Handle the request and call the next middleware
    fn handle(
        &self,
        ctx: Context,
        next: Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

/// Convert a middleware trait object into a handler function
pub fn from_middleware(
    middleware: Arc<dyn Middleware>,
) -> MiddlewareHandler {
    Arc::new(move |ctx: Context, next: Next| middleware.handle(ctx, next))
}

/// Logger middleware - logs request method, path, and response time
pub struct Logger;

impl Middleware for Logger {
    fn handle(
        &self,
        ctx: Context,
        next: Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        Box::pin(async move {
            let start = Instant::now();
            let method = ctx.request().method().as_str().to_string();
            let path = ctx.request().uri().to_string();

            let response = next.run(ctx).await;

            let duration = start.elapsed();
            let status = response.status().as_u16();

            tracing::info!(
                "{} {} - {} ({:?})",
                method,
                path,
                status,
                duration
            );

            response
        })
    }
}

/// CORS middleware - adds CORS headers to responses
pub struct Cors {
    allow_origin: String,
    allow_methods: String,
    allow_headers: String,
}

impl Cors {
    /// Create a new CORS middleware with default settings
    pub fn new() -> Self {
        Self {
            allow_origin: "*".to_string(),
            allow_methods: "GET, POST, PUT, DELETE, PATCH, OPTIONS".to_string(),
            allow_headers: "Content-Type, Authorization".to_string(),
        }
    }

    /// Set the allowed origin
    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.allow_origin = origin.into();
        self
    }

    /// Set the allowed methods
    pub fn allow_methods(mut self, methods: impl Into<String>) -> Self {
        self.allow_methods = methods.into();
        self
    }

    /// Set the allowed headers
    pub fn allow_headers(mut self, headers: impl Into<String>) -> Self {
        self.allow_headers = headers.into();
        self
    }
}

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for Cors {
    fn handle(
        &self,
        ctx: Context,
        next: Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let allow_origin = self.allow_origin.clone();
        let allow_methods = self.allow_methods.clone();
        let allow_headers = self.allow_headers.clone();

        Box::pin(async move {
            let response = next.run(ctx).await;

            // Add CORS headers
            response
                .with_header("Access-Control-Allow-Origin".to_string(), allow_origin)
                .with_header("Access-Control-Allow-Methods".to_string(), allow_methods)
                .with_header("Access-Control-Allow-Headers".to_string(), allow_headers)
        })
    }
}

/// Request ID middleware - adds a unique request ID to the context
pub struct RequestId;

impl Middleware for RequestId {
    fn handle(
        &self,
        mut ctx: Context,
        next: Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        Box::pin(async move {
            let request_id = uuid::Uuid::new_v4().to_string();
            ctx.extensions_mut().insert(request_id.clone());

            let mut response = next.run(ctx).await;

            // Add request ID to response headers
            response = response.with_header("X-Request-ID".to_string(), request_id);

            response
        })
    }
}

/// Middleware stack for composing multiple middlewares
pub struct MiddlewareStack {
    middlewares: Vec<MiddlewareHandler>,
}

impl MiddlewareStack {
    /// Create a new empty middleware stack
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware function to the stack
    pub fn add<F, Fut>(&mut self, middleware: F)
    where
        F: Fn(Context, Next) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = Arc::new(
            move |ctx: Context, next: Next| -> Pin<Box<dyn Future<Output = Response> + Send>> {
                Box::pin(middleware(ctx, next))
            },
        );
        self.middlewares.push(handler);
    }

    /// Add a middleware trait object to the stack
    pub fn add_middleware(&mut self, middleware: Arc<dyn Middleware>) {
        self.middlewares.push(from_middleware(middleware));
    }

    /// Execute the middleware stack with a final handler
    pub async fn execute<F, Fut>(&self, ctx: Context, handler: F) -> Response
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        // Create a copy of middlewares and add the final handler
        let mut all_handlers = self.middlewares.clone();

        // Add the final handler as the last middleware
        let final_handler = Arc::new(
            move |ctx: Context, _next: Next| -> Pin<Box<dyn Future<Output = Response> + Send>> {
                Box::pin(handler(ctx))
            },
        );
        all_handlers.push(final_handler);

        // Start the middleware chain
        let next = Next::new(all_handlers);
        next.run(ctx).await
    }

    /// Get the number of middlewares in the stack
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Method, Request, StatusCode};

    #[tokio::test]
    async fn test_middleware_chain() {
        let mut stack = MiddlewareStack::new();

        // Add middleware that adds a header
        stack.add(|mut ctx, next| async move {
            ctx.extensions_mut().insert("middleware1".to_string());
            let response = next.run(ctx).await;
            response.with_header("X-Middleware-1".to_string(), "true".to_string())
        });

        stack.add(|mut ctx, next| async move {
            ctx.extensions_mut().insert(42i32);
            let response = next.run(ctx).await;
            response.with_header("X-Middleware-2".to_string(), "true".to_string())
        });

        let request = Request::new(Method::GET, "/test".to_string());
        let ctx = Context::new(request);

        let response = stack
            .execute(ctx, |ctx| async move {
                assert!(ctx.extensions().contains::<String>());
                assert!(ctx.extensions().contains::<i32>());
                Response::text("OK")
            })
            .await;

        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(response.header("X-Middleware-1"), Some(&"true".to_string()));
        assert_eq!(response.header("X-Middleware-2"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_logger_middleware() {
        let logger = Arc::new(Logger);
        let mut stack = MiddlewareStack::new();
        stack.add_middleware(logger);

        let request = Request::new(Method::GET, "/test".to_string());
        let ctx = Context::new(request);

        let response = stack
            .execute(ctx, |_ctx| async { Response::text("OK") })
            .await;

        assert_eq!(response.status(), StatusCode::Ok);
    }

    #[tokio::test]
    async fn test_cors_middleware() {
        let cors = Arc::new(Cors::new().allow_origin("https://example.com"));
        let mut stack = MiddlewareStack::new();
        stack.add_middleware(cors);

        let request = Request::new(Method::GET, "/test".to_string());
        let ctx = Context::new(request);

        let response = stack
            .execute(ctx, |_ctx| async { Response::text("OK") })
            .await;

        assert_eq!(
            response.header("Access-Control-Allow-Origin"),
            Some(&"https://example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_middleware_order() {
        let mut stack = MiddlewareStack::new();
        let order = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let order1 = order.clone();
        stack.add(move |ctx, next| {
            let order = order1.clone();
            async move {
                order.lock().await.push("before-1");
                let response = next.run(ctx).await;
                order.lock().await.push("after-1");
                response
            }
        });

        let order2 = order.clone();
        stack.add(move |ctx, next| {
            let order = order2.clone();
            async move {
                order.lock().await.push("before-2");
                let response = next.run(ctx).await;
                order.lock().await.push("after-2");
                response
            }
        });

        let order3 = order.clone();
        let request = Request::new(Method::GET, "/test".to_string());
        let ctx = Context::new(request);

        stack
            .execute(ctx, move |_ctx| {
                let order = order3.clone();
                async move {
                    order.lock().await.push("handler");
                    Response::text("OK")
                }
            })
            .await;

        let execution_order = order.lock().await;
        assert_eq!(
            *execution_order,
            vec!["before-1", "before-2", "handler", "after-2", "after-1"]
        );
    }
}
