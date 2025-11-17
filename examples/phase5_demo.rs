//! Phase 5 Demo: Performance Features
//!
//! This example demonstrates:
//! - In-memory LRU cache with TTL
//! - Redis client with RESP protocol
//! - Compression middleware (Gzip and Brotli)
//!
//! Run with:
//! ```
//! cargo run --example phase5_demo
//! ```

use pttp::cache::{Compression, LruCache};
use pttp::context::{Context, Params};
use pttp::http::{Method, Request, Response, StatusCode};
use pttp::middleware::{from_middleware, Cors, Logger, MiddlewareStack, RequestId};
use pttp::router::Router;
use pttp::server::Server;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 PTTP Phase 5 Demo: Performance Features\n");
    println!("Features showcased:");
    println!("  ✓ In-memory LRU cache with TTL support");
    println!("  ✓ Compression middleware (Gzip and Brotli)");
    println!("  ✓ High-performance caching layer\n");

    // Create an LRU cache for demonstration
    let cache: Arc<LruCache<String, String>> = Arc::new(LruCache::new(100));

    // Pre-populate cache with some data
    cache.insert(
        "greeting".to_string(),
        "Hello from cached data!".to_string(),
        Some(Duration::from_secs(300)),
    );
    cache.insert(
        "version".to_string(),
        "1.0.0".to_string(),
        None, // No expiration
    );

    println!("📦 Cache initialized with {} items\n", cache.len());

    // Create router
    let mut router = Router::new();

    // Clone cache for route handlers
    let cache_clone1 = Arc::clone(&cache);
    let cache_clone2 = Arc::clone(&cache);
    let cache_clone3 = Arc::clone(&cache);
    let cache_clone4 = Arc::clone(&cache);

    // Route 1: Get cached value
    router.get("/cache/:key", move |ctx: Context| {
        let cache = Arc::clone(&cache_clone1);
        Box::pin(async move {
            let key = ctx.param("key").unwrap_or("unknown");

            match cache.get(&key.to_string()) {
                Some(value) => Response::json(&serde_json::json!({
                    "cached": true,
                    "key": key,
                    "value": value
                }))
                .unwrap(),
                None => Response::json(&serde_json::json!({
                    "cached": false,
                    "key": key,
                    "message": "Key not found in cache"
                }))
                .unwrap()
                .with_header("X-Cache".to_string(), "MISS".to_string()),
            }
        })
    });

    // Route 2: Set cache value with optional TTL
    router.post("/cache/:key", move |mut ctx: Context| {
        let cache = Arc::clone(&cache_clone2);
        Box::pin(async move {
            let key = ctx.param("key").unwrap_or("unknown").to_string();

            // Parse JSON body
            let body: serde_json::Value = match ctx.json() {
                Ok(b) => b,
                Err(_) => {
                    return Response::new(StatusCode::BadRequest)
                        .with_body(b"Invalid JSON body".to_vec())
                }
            };

            let value = body["value"].as_str().unwrap_or("").to_string();
            let ttl = body["ttl"]
                .as_u64()
                .map(|secs| Duration::from_secs(secs));

            cache.insert(key.clone(), value.clone(), ttl);

            Response::json(&serde_json::json!({
                "success": true,
                "key": key,
                "value": value,
                "ttl": ttl.map(|d| d.as_secs())
            }))
            .unwrap()
        })
    });

    // Route 3: Delete from cache
    router.delete("/cache/:key", move |ctx: Context| {
        let cache = Arc::clone(&cache_clone3);
        Box::pin(async move {
            let key = ctx.param("key").unwrap_or("unknown");

            let removed = cache.remove(&key.to_string()).is_some();

            Response::json(&serde_json::json!({
                "success": removed,
                "key": key
            }))
            .unwrap()
        })
    });

    // Route 4: Cache statistics
    router.get("/cache", move |_ctx: Context| {
        let cache = Arc::clone(&cache_clone4);
        Box::pin(async move {
            Response::json(&serde_json::json!({
                "size": cache.len(),
                "capacity": cache.capacity(),
                "is_empty": cache.is_empty()
            }))
            .unwrap()
        })
    });

    // Route 5: Large text response (for compression testing)
    router.get("/large-text", |_ctx: Context| {
        Box::pin(async move {
            let large_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
                .repeat(100);

            Response::html(format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>Large Text Response</title>
</head>
<body>
    <h1>Compression Test</h1>
    <p>This is a large text response that should be compressed when Accept-Encoding headers are present.</p>
    <p>Original size: {} bytes</p>
    <div>{}</div>
</body>
</html>"#,
                large_text.len(),
                large_text
            ))
        })
    });

    // Route 6: JSON data (for compression testing)
    router.get("/large-json", |_ctx: Context| {
        Box::pin(async move {
            let items: Vec<_> = (0..1000)
                .map(|i| {
                    serde_json::json!({
                        "id": i,
                        "name": format!("Item {}", i),
                        "description": "This is a sample item for testing compression",
                        "price": i as f64 * 1.99,
                        "in_stock": i % 2 == 0
                    })
                })
                .collect();

            Response::json(&serde_json::json!({
                "total": items.len(),
                "items": items
            }))
            .unwrap()
        })
    });

    // Route 7: Welcome page
    router.get("/", |_ctx: Context| {
        Box::pin(async move {
            Response::html(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>PTTP Phase 5 Demo</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        h1 { color: #333; }
        .endpoint { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
        .method { color: #fff; padding: 3px 8px; border-radius: 3px; font-weight: bold; }
        .get { background: #61affe; }
        .post { background: #49cc90; }
        .delete { background: #f93e3e; }
        code { background: #e8e8e8; padding: 2px 6px; border-radius: 3px; }
    </style>
</head>
<body>
    <h1>🚀 PTTP Phase 5: Performance Features Demo</h1>
    <p>This demo showcases the caching and compression features implemented in Phase 5.</p>

    <h2>📦 Cache Endpoints</h2>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/cache/:key</code>
        <p>Retrieve a value from the cache</p>
    </div>

    <div class="endpoint">
        <span class="method post">POST</span> <code>/cache/:key</code>
        <p>Store a value in the cache with optional TTL</p>
        <p>Body: <code>{"value": "...", "ttl": 60}</code></p>
    </div>

    <div class="endpoint">
        <span class="method delete">DELETE</span> <code>/cache/:key</code>
        <p>Remove a value from the cache</p>
    </div>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/cache</code>
        <p>Get cache statistics</p>
    </div>

    <h2>📊 Compression Test Endpoints</h2>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/large-text</code>
        <p>Large HTML response (for compression testing)</p>
    </div>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/large-json</code>
        <p>Large JSON response (1000 items, for compression testing)</p>
    </div>

    <h2>🧪 Try it out!</h2>
    <pre>
# Get cached greeting
curl http://localhost:8080/cache/greeting

# Set a new cached value
curl -X POST http://localhost:8080/cache/mykey \
  -H "Content-Type: application/json" \
  -d '{"value": "Hello World", "ttl": 60}'

# Get cache stats
curl http://localhost:8080/cache

# Test compression (notice Content-Encoding header)
curl -H "Accept-Encoding: gzip, br" http://localhost:8080/large-json -v
    </pre>
</body>
</html>"#,
            )
        })
    });

    // Build middleware stack
    let mut middlewares = MiddlewareStack::new();

    // Add logging middleware
    middlewares.add(from_middleware(Logger));

    // Add request ID middleware
    middlewares.add(from_middleware(RequestId::new()));

    // Add CORS middleware
    middlewares.add(from_middleware(Cors::permissive()));

    // Add compression middleware (level 6, min size 1KB)
    middlewares.add(from_middleware(Compression::new(6).with_min_size(1024)));

    // Create and start server
    let server = Server::builder()
        .bind("127.0.0.1:8080")
        .router(router)
        .middlewares(middlewares)
        .build()
        .await?;

    println!("🌐 Server running at http://127.0.0.1:8080");
    println!("📝 Visit http://127.0.0.1:8080 for available endpoints\n");

    server.run().await?;

    Ok(())
}
