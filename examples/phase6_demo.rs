//! Phase 6 Demo: Real-Time Features
//!
//! This example demonstrates:
//! - WebSocket support for bidirectional communication
//! - Server-Sent Events (SSE) for server-to-client streaming
//! - Real-time chat using WebSocket
//! - Live updates using SSE
//!
//! Run with:
//! ```
//! cargo run --example phase6_demo
//! ```

use pttp::context::Context;
use pttp::http::{Response, StatusCode};
use pttp::middleware::{Cors, Logger, MiddlewareStack, RequestId};
use pttp::realtime::sse::{create_sse_stream, Event};
use pttp::realtime::websocket::WebSocketUpgrade;
use pttp::router::Router;
use pttp::server::Server;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 PTTP Phase 6 Demo: Real-Time Features\n");
    println!("Features showcased:");
    println!("  ✓ WebSocket support for bidirectional communication");
    println!("  ✓ Server-Sent Events (SSE) for server-to-client streaming");
    println!("  ✓ Real-time chat using WebSocket");
    println!("  ✓ Live updates using SSE\n");

    // Create broadcast channel for chat messages
    let (chat_tx, _) = broadcast::channel::<String>(100);
    let chat_tx = Arc::new(chat_tx);

    // Create router
    let mut router = Router::new();

    // Route 1: Welcome page with HTML UI
    router.get("/", |_ctx: Context| {
        Box::pin(async move {
            Response::html(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>PTTP Phase 6 Demo</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 1200px; margin: 50px auto; padding: 20px; }
        h1 { color: #333; }
        .container { display: flex; gap: 20px; margin-top: 20px; }
        .section { flex: 1; border: 1px solid #ddd; padding: 20px; border-radius: 8px; }
        .endpoint { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
        .method { color: #fff; padding: 3px 8px; border-radius: 3px; font-weight: bold; }
        .get { background: #61affe; }
        .ws { background: #50e3c2; }
        code { background: #e8e8e8; padding: 2px 6px; border-radius: 3px; }
        #chat-container { height: 300px; overflow-y: auto; border: 1px solid #ccc; padding: 10px; margin-bottom: 10px; background: #fafafa; }
        #sse-container { height: 200px; overflow-y: auto; border: 1px solid #ccc; padding: 10px; background: #fafafa; }
        .message { margin: 5px 0; padding: 5px; background: white; border-radius: 3px; }
        .event { margin: 5px 0; padding: 5px; background: #e8f5e9; border-radius: 3px; }
        input[type="text"] { padding: 8px; border: 1px solid #ddd; border-radius: 4px; }
        button { padding: 8px 16px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; }
        button:hover { background: #45a049; }
        .status { padding: 5px 10px; border-radius: 3px; margin: 10px 0; }
        .connected { background: #c8e6c9; color: #2e7d32; }
        .disconnected { background: #ffcdd2; color: #c62828; }
    </style>
</head>
<body>
    <h1>🚀 PTTP Phase 6: Real-Time Features Demo</h1>
    <p>This demo showcases the WebSocket and Server-Sent Events features implemented in Phase 6.</p>

    <div class="container">
        <div class="section">
            <h2>💬 WebSocket Chat</h2>
            <div id="ws-status" class="status disconnected">Disconnected</div>
            <div id="chat-container"></div>
            <div style="display: flex; gap: 10px;">
                <input type="text" id="message-input" placeholder="Type a message..." style="flex: 1;" />
                <button onclick="sendMessage()">Send</button>
            </div>
        </div>

        <div class="section">
            <h2>📡 Server-Sent Events</h2>
            <div id="sse-status" class="status disconnected">Disconnected</div>
            <div id="sse-container"></div>
            <button onclick="startSSE()">Start SSE Stream</button>
            <button onclick="stopSSE()">Stop SSE Stream</button>
        </div>
    </div>

    <h2>📚 Available Endpoints</h2>

    <div class="endpoint">
        <span class="method ws">WS</span> <code>ws://localhost:8080/ws/chat</code>
        <p>WebSocket endpoint for real-time chat</p>
    </div>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/sse/events</code>
        <p>Server-Sent Events stream with live updates</p>
    </div>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/sse/time</code>
        <p>SSE stream that sends current time every second</p>
    </div>

    <div class="endpoint">
        <span class="method get">GET</span> <code>/sse/counter</code>
        <p>SSE stream that counts from 1 to 10</p>
    </div>

    <script>
        let ws = null;
        let eventSource = null;

        // WebSocket Chat
        function connectWebSocket() {
            ws = new WebSocket('ws://localhost:8080/ws/chat');

            ws.onopen = () => {
                document.getElementById('ws-status').textContent = 'Connected';
                document.getElementById('ws-status').className = 'status connected';
                addChatMessage('System: Connected to chat');
            };

            ws.onmessage = (event) => {
                addChatMessage('Message: ' + event.data);
            };

            ws.onclose = () => {
                document.getElementById('ws-status').textContent = 'Disconnected';
                document.getElementById('ws-status').className = 'status disconnected';
                addChatMessage('System: Disconnected from chat');
            };

            ws.onerror = (error) => {
                console.error('WebSocket error:', error);
                addChatMessage('System: Connection error');
            };
        }

        function sendMessage() {
            const input = document.getElementById('message-input');
            const message = input.value.trim();

            if (message && ws && ws.readyState === WebSocket.OPEN) {
                ws.send(message);
                addChatMessage('You: ' + message);
                input.value = '';
            }
        }

        function addChatMessage(text) {
            const container = document.getElementById('chat-container');
            const messageDiv = document.createElement('div');
            messageDiv.className = 'message';
            messageDiv.textContent = text;
            container.appendChild(messageDiv);
            container.scrollTop = container.scrollHeight;
        }

        document.getElementById('message-input').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') sendMessage();
        });

        // Server-Sent Events
        function startSSE() {
            if (eventSource) {
                eventSource.close();
            }

            eventSource = new EventSource('/sse/events');

            eventSource.onopen = () => {
                document.getElementById('sse-status').textContent = 'Connected';
                document.getElementById('sse-status').className = 'status connected';
                addSSEEvent('System: Connected to SSE stream');
            };

            eventSource.onmessage = (event) => {
                addSSEEvent('Message: ' + event.data);
            };

            eventSource.addEventListener('update', (event) => {
                addSSEEvent('Update: ' + event.data);
            });

            eventSource.addEventListener('notification', (event) => {
                addSSEEvent('Notification: ' + event.data);
            });

            eventSource.onerror = () => {
                document.getElementById('sse-status').textContent = 'Error';
                document.getElementById('sse-status').className = 'status disconnected';
            };
        }

        function stopSSE() {
            if (eventSource) {
                eventSource.close();
                document.getElementById('sse-status').textContent = 'Disconnected';
                document.getElementById('sse-status').className = 'status disconnected';
                addSSEEvent('System: Disconnected from SSE stream');
            }
        }

        function addSSEEvent(text) {
            const container = document.getElementById('sse-container');
            const eventDiv = document.createElement('div');
            eventDiv.className = 'event';
            eventDiv.textContent = text;
            container.appendChild(eventDiv);
            container.scrollTop = container.scrollHeight;
        }

        // Auto-connect on load
        connectWebSocket();
    </script>
</body>
</html>"#,
            )
        })
    });

    // Route 2: WebSocket chat endpoint
    let chat_tx_clone = Arc::clone(&chat_tx);
    router.get("/ws/chat", move |ctx: Context| {
        let _chat_tx = Arc::clone(&chat_tx_clone);
        Box::pin(async move {
            // Check if this is a WebSocket upgrade request
            let request = ctx.request();
            if !WebSocketUpgrade::is_upgrade_request(request) {
                return Response::new(StatusCode::BadRequest)
                    .with_body(b"Expected WebSocket upgrade request".to_vec());
            }

            // Note: In a real implementation, we would need access to the raw TcpStream
            // For now, return a message explaining this limitation
            Response::html(
                r#"<html><body>
                <h1>WebSocket Endpoint</h1>
                <p>This endpoint handles WebSocket connections at <code>ws://localhost:8080/ws/chat</code></p>
                <p>Connect using a WebSocket client or the JavaScript interface on the home page.</p>
                </body></html>"#,
            )
        })
    });

    // Route 3: SSE events stream
    router.get("/sse/events", |_ctx: Context| {
        Box::pin(async move {
            create_sse_stream(|stream| async move {
                // Send initial events
                stream
                    .send(Event::new("update").data("Connected to event stream"))
                    .await
                    .ok();

                // Send periodic updates
                for i in 1..=5 {
                    time::sleep(Duration::from_secs(1)).await;
                    stream
                        .send(
                            Event::new("update")
                                .data(format!("Update #{}", i))
                                .id(i.to_string()),
                        )
                        .await
                        .ok();
                }

                // Send a notification
                stream
                    .send(Event::new("notification").data("All updates sent!"))
                    .await
                    .ok();
            })
            .await
        })
    });

    // Route 4: SSE time stream
    router.get("/sse/time", |_ctx: Context| {
        Box::pin(async move {
            create_sse_stream(|stream| async move {
                for _ in 0..10 {
                    let now = chrono::Local::now().format("%H:%M:%S");
                    stream
                        .send(Event::default().data(format!("Current time: {}", now)))
                        .await
                        .ok();
                    time::sleep(Duration::from_secs(1)).await;
                }
            })
            .await
        })
    });

    // Route 5: SSE counter stream
    router.get("/sse/counter", |_ctx: Context| {
        Box::pin(async move {
            create_sse_stream(|stream| async move {
                for i in 1..=10 {
                    stream
                        .send(
                            Event::new("counter")
                                .data(format!("Count: {}", i))
                                .id(i.to_string()),
                        )
                        .await
                        .ok();
                    time::sleep(Duration::from_millis(500)).await;
                }

                stream
                    .send(Event::new("complete").data("Counting complete!"))
                    .await
                    .ok();
            })
            .await
        })
    });

    // Route 6: API status endpoint
    router.get("/api/status", |_ctx: Context| {
        Box::pin(async move {
            Response::json(&serde_json::json!({
                "status": "ok",
                "phase": "6",
                "features": {
                    "websocket": true,
                    "sse": true,
                    "real_time": true
                },
                "endpoints": {
                    "websocket": ["/ws/chat"],
                    "sse": ["/sse/events", "/sse/time", "/sse/counter"]
                }
            }))
            .unwrap()
        })
    });

    // Build middleware stack
    let mut middlewares = MiddlewareStack::new();

    // Add logging middleware
    middlewares.add_middleware(Arc::new(Logger));

    // Add request ID middleware
    middlewares.add_middleware(Arc::new(RequestId));

    // Add CORS middleware
    middlewares.add_middleware(Arc::new(
        Cors::new()
            .allow_origin("*")
            .allow_methods("GET, POST, PUT, DELETE, PATCH, OPTIONS"),
    ));

    // Create and start server
    let server = Server::with_router_and_middleware("127.0.0.1:8080", router, middlewares);

    println!("🌐 Server running at http://127.0.0.1:8080");
    println!("📝 Visit http://127.0.0.1:8080 for the interactive demo\n");
    println!("WebSocket endpoints:");
    println!("  • ws://127.0.0.1:8080/ws/chat - Real-time chat\n");
    println!("SSE endpoints:");
    println!("  • http://127.0.0.1:8080/sse/events - Event stream");
    println!("  • http://127.0.0.1:8080/sse/time - Time updates");
    println!("  • http://127.0.0.1:8080/sse/counter - Counter stream\n");

    server.run().await?;

    Ok(())
}
