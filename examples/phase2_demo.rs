//! Phase 2 Demo - Router & Middleware
//!
//! This example demonstrates all Phase 2 features:
//! - Pattern matching router
//! - Path parameter extraction
//! - Middleware system (Logger, CORS, custom middleware)
//! - Request context and extensions
//! - Query parameter handling

use pttp::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,pttp=debug")
        .init();

    // Create router
    let mut router = Router::new();

    // Root endpoint
    router.get("/", |_ctx| async {
        Response::html(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>PTTP - Phase 2 Demo</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        h1 { color: #333; }
        .endpoint { background: #f4f4f4; padding: 10px; margin: 10px 0; border-left: 3px solid #007acc; }
        code { background: #eee; padding: 2px 5px; }
    </style>
</head>
<body>
    <h1>🦀 PTTP Phase 2 - Router & Middleware Demo</h1>
    <p>Phase 2 is complete! Try these endpoints:</p>

    <div class="endpoint">
        <strong>GET /</strong> - This page
    </div>

    <div class="endpoint">
        <strong>GET /health</strong> - Health check endpoint
    </div>

    <div class="endpoint">
        <strong>GET /users/:id</strong> - Get user by ID (e.g., /users/123)
    </div>

    <div class="endpoint">
        <strong>POST /users</strong> - Create a new user
    </div>

    <div class="endpoint">
        <strong>GET /users/:user_id/posts/:post_id</strong> - Nested parameters
    </div>

    <div class="endpoint">
        <strong>GET /search?q=rust&limit=10</strong> - Query parameters
    </div>

    <div class="endpoint">
        <strong>GET /files/*</strong> - Wildcard route (matches /files/anything/here)
    </div>

    <h2>✨ Phase 2 Features:</h2>
    <ul>
        <li>✅ Pattern matching router (exact, parameterized, wildcard)</li>
        <li>✅ Path parameter extraction</li>
        <li>✅ Middleware system with chaining (onion model)</li>
        <li>✅ Built-in middleware (Logger, CORS, RequestID)</li>
        <li>✅ Request context with type-safe extensions</li>
        <li>✅ Query parameter handling</li>
    </ul>
</body>
</html>
"#,
        )
    });

    // Health check endpoint
    router.get("/health", |_ctx| async {
        Response::json(&serde_json::json!({
            "status": "healthy",
            "version": pttp::VERSION,
            "phase": "2",
            "features": [
                "router",
                "middleware",
                "context",
                "path_params",
                "query_params"
            ]
        }))
        .unwrap_or_else(|_| Response::internal_error())
    });

    // Get user by ID (path parameter)
    router.get("/users/:id", |ctx| async move {
        let id = ctx.param("id").unwrap_or("unknown");
        Response::json(&serde_json::json!({
            "user_id": id,
            "name": format!("User {}", id),
            "email": format!("user{}@example.com", id)
        }))
        .unwrap_or_else(|_| Response::internal_error())
    });

    // Create user (POST)
    router.post("/users", |ctx| async move {
        // Try to parse JSON body
        match ctx.json::<serde_json::Value>() {
            Ok(user_data) => Response::json(&serde_json::json!({
                "message": "User created successfully",
                "data": user_data
            }))
            .unwrap_or_else(|_| Response::internal_error()),
            Err(_) => Response::new(StatusCode::BadRequest)
                .with_body(b"Invalid JSON".to_vec()),
        }
    });

    // Nested path parameters
    router.get("/users/:user_id/posts/:post_id", |ctx| async move {
        let user_id = ctx.param("user_id").unwrap_or("unknown");
        let post_id = ctx.param("post_id").unwrap_or("unknown");

        Response::json(&serde_json::json!({
            "user_id": user_id,
            "post_id": post_id,
            "title": format!("Post {} by User {}", post_id, user_id)
        }))
        .unwrap_or_else(|_| Response::internal_error())
    });

    // Query parameters
    router.get("/search", |ctx| async move {
        let query = ctx.query("q").unwrap_or("none");
        let limit = ctx.query("limit").unwrap_or("10");

        Response::json(&serde_json::json!({
            "query": query,
            "limit": limit,
            "results": []
        }))
        .unwrap_or_else(|_| Response::internal_error())
    });

    // Wildcard route
    router.get("/files/*", |ctx| async move {
        let path = ctx.request().uri();
        Response::text(format!("File handler for: {}", path))
    });

    // Create middleware stack
    let mut middleware = MiddlewareStack::new();

    // Add Logger middleware
    middleware.add_middleware(Arc::new(Logger));

    // Add CORS middleware
    middleware.add_middleware(Arc::new(
        Cors::new().allow_origin("*").allow_methods("GET, POST, PUT, DELETE, PATCH, OPTIONS"),
    ));

    // Add RequestID middleware
    middleware.add_middleware(Arc::new(RequestId));

    // Add custom middleware that adds timing information
    middleware.add(|mut ctx, next| async move {
        use std::time::Instant;
        let start = Instant::now();

        // Add start time to extensions
        ctx.extensions_mut().insert(start);

        let mut response = next.run(ctx).await;

        // Add timing header
        let duration = start.elapsed();
        response = response.with_header(
            "X-Response-Time".to_string(),
            format!("{:.2}ms", duration.as_secs_f64() * 1000.0),
        );

        response
    });

    // Create server with router and middleware
    let server = Server::with_router_and_middleware("127.0.0.1:3000", router, middleware);

    println!("🚀 PTTP Phase 2 Demo Server");
    println!("📍 Listening on http://127.0.0.1:3000");
    println!("🎯 Visit http://127.0.0.1:3000 to see available endpoints");
    println!();
    println!("Press Ctrl+C to stop");

    server.run().await?;

    Ok(())
}
