# PTTP: Pure Rust Web Framework with AI/LLM Integration

> **Project Goal:** Build a production-grade web framework from near-scratch in Rust to maximize learning about systems programming, async I/O, networking, and AI integration.

## 🎯 Learning Philosophy

**Minimal Dependencies Strategy:**
- Build core components from scratch to understand fundamentals
- Use only essential libraries where complexity is too high
- Allowed: tokio (async runtime), serde (serialization)
- Build yourself: HTTP parser, router, middleware, ORM, auth, etc.

---

## 📚 Prerequisites & Study Path

### Before Starting:
1. **Rust Book** - Chapters 1-10, 13, 15-16, 19
2. **Async Book** - Understanding `Future`, `async/await`, `Pin`
3. **Tokio Tutorial** - Runtime basics, tasks, I/O

### Key Concepts to Master:
- Ownership, borrowing, lifetimes
- Trait system and generics
- Error handling (`Result`, `?` operator)
- Async programming model
- Smart pointers (`Box`, `Arc`, `Rc`)
- Interior mutability (`RefCell`, `Mutex`)

---

## 🗺️ Strategic Roadmap

### **Phase 0: Foundation Setup** (Week 1)
**Goal:** Project structure and development environment

- [x] Initialize Cargo workspace
- [ ] Setup project structure (lib + examples)
- [ ] Configure development tools (rustfmt, clippy)
- [ ] Create basic module hierarchy
- [ ] Setup logging infrastructure

**Modules to Create:**
```
pttp/
├── src/
│   ├── lib.rs           # Public API
│   ├── http/            # HTTP protocol
│   ├── server/          # TCP server
│   ├── router/          # Request routing
│   ├── middleware/      # Middleware system
│   ├── context/         # Request context
│   ├── database/        # Database layer
│   ├── security/        # Auth & security
│   ├── cache/           # Caching layer
│   ├── realtime/        # WebSocket/SSE
│   ├── background/      # Task queue
│   └── llm/             # AI/LLM integration
└── examples/            # Usage examples
```

**Dependencies (Minimal):**
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Learning Outcomes:**
- Cargo workspace management
- Module organization patterns
- Rust project structure best practices

---

### **Phase 1: HTTP Server Core** (Weeks 2-4)
**Goal:** Build TCP server with HTTP/1.1 protocol support from scratch

#### 1.1 TCP Server (Week 2)
- [ ] Raw TCP listener with `tokio::net::TcpListener`
- [ ] Accept connections and spawn tasks
- [ ] Connection lifecycle management
- [ ] Graceful shutdown handling
- [ ] Connection pooling basics

**What You'll Learn:**
- Async I/O fundamentals
- Task spawning and lifecycle
- Error handling in async contexts
- Resource cleanup patterns

**Build From Scratch:**
```rust
// src/server/tcp.rs
pub struct TcpServer {
    listener: TcpListener,
    shutdown: broadcast::Receiver<()>,
}

impl TcpServer {
    pub async fn bind(addr: &str) -> Result<Self>;
    pub async fn accept_loop(&mut self) -> Result<()>;
    async fn handle_connection(stream: TcpStream);
}
```

#### 1.2 HTTP/1.1 Parser (Week 3)
- [ ] Request line parsing (method, path, version)
- [ ] Header parsing (key-value pairs)
- [ ] Body reading (Content-Length, chunked transfer)
- [ ] Query string parsing
- [ ] URL decoding

**What You'll Learn:**
- Byte-level protocol handling
- State machines for parsing
- Zero-copy techniques
- Error recovery strategies

**Build From Scratch:**
```rust
// src/http/parser.rs
pub struct HttpParser {
    state: ParserState,
    buffer: BytesMut,
}

impl HttpParser {
    pub fn parse_request(&mut self, buf: &[u8]) -> Result<Option<Request>>;
    fn parse_request_line(&self, line: &str) -> Result<(Method, Uri, Version)>;
    fn parse_headers(&self, lines: &[&str]) -> Result<HeaderMap>;
}
```

**Allowed Helper:** `httparse` crate (optional, try without first!)

#### 1.3 Request/Response Abstractions (Week 4)
- [ ] `Request` struct (method, uri, headers, body)
- [ ] `Response` struct (status, headers, body)
- [ ] `HeaderMap` implementation
- [ ] Body streaming interface
- [ ] Response builder pattern

**Build From Scratch:**
```rust
// src/http/request.rs
pub struct Request {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    body: Body,
    extensions: Extensions, // For middleware data
}

// src/http/response.rs
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Body,
}

impl Response {
    pub fn builder() -> ResponseBuilder;
    pub fn json<T: Serialize>(data: T) -> Result<Self>;
    pub fn html(content: impl Into<String>) -> Self;
}
```

**Learning Outcomes:**
- Builder pattern implementation
- Type-safe APIs
- Generic programming with trait bounds
- Efficient string/byte handling

---

### **Phase 2: Router & Middleware** (Weeks 5-7)

#### 2.1 Router Implementation (Week 5)
- [ ] Pattern matching (exact, prefix, wildcards)
- [ ] Path parameter extraction (`/users/:id`)
- [ ] Regex-based routes
- [ ] Method-based routing (GET, POST, etc.)
- [ ] Nested routers (sub-applications)

**What You'll Learn:**
- Tree data structures (radix tree/trie)
- Pattern matching algorithms
- Generic handler types
- Trait objects vs generics trade-offs

**Build From Scratch:**
```rust
// src/router/mod.rs
pub struct Router {
    routes: Vec<Route>,
    // OR use a radix tree for efficiency
    tree: RadixTree<Handler>,
}

pub struct Route {
    pattern: Pattern,
    method: Method,
    handler: Box<dyn Handler>,
}

impl Router {
    pub fn get<H>(&mut self, path: &str, handler: H)
    where H: Handler + 'static;

    pub fn route(&self, req: &Request) -> Option<(Handler, Params)>;
}

// Advanced: Type-safe extractors like Axum
pub trait FromRequest: Sized {
    async fn from_request(req: &Request) -> Result<Self>;
}
```

#### 2.2 Middleware System (Week 6)
- [ ] Middleware trait definition
- [ ] Middleware chaining (onion model)
- [ ] Before/after request hooks
- [ ] Short-circuit (early return)
- [ ] Middleware ordering

**What You'll Learn:**
- Higher-order functions
- Closure capturing
- Trait objects for dynamic dispatch
- Async composition patterns

**Build From Scratch:**
```rust
// src/middleware/mod.rs
pub trait Middleware: Send + Sync {
    async fn handle(&self, req: Request, next: Next) -> Response;
}

pub struct Next {
    middlewares: Vec<Box<dyn Middleware>>,
    handler: Box<dyn Handler>,
    index: usize,
}

impl Next {
    pub async fn run(mut self, req: Request) -> Response;
}

// Example middleware
pub struct Logger;

impl Middleware for Logger {
    async fn handle(&self, req: Request, next: Next) -> Response {
        let start = Instant::now();
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        let response = next.run(req).await;

        let duration = start.elapsed();
        println!("{} {} - {} ({:?})", method, path, response.status(), duration);

        response
    }
}
```

#### 2.3 Context & Request-Scoped Data (Week 7)
- [ ] Context struct with type-safe data storage
- [ ] Request ID generation
- [ ] User data propagation
- [ ] Database connection passing
- [ ] Extension map (type-erased storage)

**What You'll Learn:**
- `Any` trait and type erasure
- Thread-safe interior mutability
- Lifetime annotations in complex scenarios

**Build From Scratch:**
```rust
// src/context/mod.rs
pub struct Context {
    request: Request,
    params: Params,
    extensions: Extensions,
}

pub struct Extensions {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T);
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T>;
}

impl Context {
    pub fn param(&self, name: &str) -> Option<&str>;
    pub fn query(&self, name: &str) -> Option<&str>;
    pub fn json<T: DeserializeOwned>(&mut self) -> Result<T>;
}
```

**Learning Outcomes:**
- Type-safe heterogeneous collections
- Request lifecycle management
- API design for developer experience

---

### **Phase 3: Database Layer** (Weeks 8-11)

#### 3.1 Connection Pool (Week 8)
- [ ] Generic connection pool implementation
- [ ] Connection health checks
- [ ] Max connections limit
- [ ] Connection timeout handling
- [ ] Idle connection cleanup

**What You'll Learn:**
- Concurrent data structures
- `Arc<Mutex<T>>` patterns
- Async locking strategies
- Resource lifecycle management

**Build From Scratch:**
```rust
// src/database/pool.rs
pub struct Pool<C> {
    connections: Arc<Mutex<VecDeque<C>>>,
    max_size: usize,
    creator: Box<dyn Fn() -> BoxFuture<'static, Result<C>>>,
}

impl<C> Pool<C> {
    pub async fn get(&self) -> Result<PooledConnection<C>>;
    pub async fn release(&self, conn: C);
}

pub struct PooledConnection<C> {
    conn: Option<C>,
    pool: Arc<Mutex<VecDeque<C>>>,
}

impl<C> Drop for PooledConnection<C> {
    fn drop(&mut self) {
        // Return to pool
    }
}
```

#### 3.2 Query Builder (Week 9)
- [ ] SQL query construction (SELECT, INSERT, UPDATE, DELETE)
- [ ] WHERE clause builder
- [ ] JOIN support
- [ ] Parameter binding (prevent SQL injection)
- [ ] Type-safe column selection

**What You'll Learn:**
- Builder pattern mastery
- Compile-time SQL safety
- String manipulation
- Preventing injection attacks

**Build From Scratch:**
```rust
// src/database/query.rs
pub struct QueryBuilder {
    table: String,
    columns: Vec<String>,
    wheres: Vec<WhereClause>,
    joins: Vec<Join>,
    params: Vec<Value>,
}

impl QueryBuilder {
    pub fn select(columns: &[&str]) -> Self;
    pub fn from(table: &str) -> Self;
    pub fn r#where(column: &str, op: &str, value: Value) -> Self;
    pub fn join(table: &str, on: &str) -> Self;
    pub fn build(&self) -> (String, Vec<Value>);
}

// Usage:
// let (sql, params) = QueryBuilder::select(&["id", "name"])
//     .from("users")
//     .where("age", ">", 18.into())
//     .build();
```

#### 3.3 ORM Features (Week 10)
- [ ] Derive macro for models (optional: use `serde` initially)
- [ ] Struct <-> Row mapping
- [ ] Relationship loading (belongs_to, has_many)
- [ ] Lazy loading
- [ ] Eager loading (N+1 prevention)

**What You'll Learn:**
- Procedural macros (advanced)
- Reflection-like patterns in Rust
- Type-level programming

**Build From Scratch:**
```rust
// src/database/model.rs
pub trait Model {
    fn table_name() -> &'static str;
    fn from_row(row: Row) -> Result<Self> where Self: Sized;
    fn to_values(&self) -> Vec<(&str, Value)>;
}

// Manual implementation (later: derive macro)
struct User {
    id: i64,
    name: String,
    email: String,
}

impl Model for User {
    fn table_name() -> &'static str { "users" }
    fn from_row(row: Row) -> Result<Self> {
        Ok(User {
            id: row.get("id")?,
            name: row.get("name")?,
            email: row.get("email")?,
        })
    }
}
```

#### 3.4 Transaction Management (Week 11)
- [ ] Transaction begin/commit/rollback
- [ ] Nested transactions (savepoints)
- [ ] Isolation level configuration
- [ ] Automatic rollback on error

**Build From Scratch:**
```rust
// src/database/transaction.rs
pub struct Transaction<'a> {
    conn: &'a mut Connection,
    committed: bool,
}

impl<'a> Transaction<'a> {
    pub async fn commit(mut self) -> Result<()>;
    pub async fn rollback(mut self) -> Result<()>;
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if !self.committed {
            // Rollback on drop
        }
    }
}

// Usage:
// let mut tx = pool.begin().await?;
// tx.execute("INSERT ...").await?;
// tx.commit().await?;
```

**Allowed Helper:** Database driver crates:
- `tokio-postgres` for PostgreSQL wire protocol
- `mysql_async` for MySQL
- `rusqlite` with async wrapper for SQLite

---

### **Phase 4: Security Layer** (Weeks 12-14)

#### 4.1 Authentication (Week 12)
- [ ] JWT token generation/validation
- [ ] Session storage (in-memory + Redis)
- [ ] Password hashing (bcrypt/argon2)
- [ ] Authentication middleware
- [ ] Token refresh mechanism

**What You'll Learn:**
- Cryptography basics
- Token-based auth patterns
- Secure storage strategies

**Build From Scratch:**
```rust
// src/security/auth.rs
pub struct JwtAuth {
    secret: Vec<u8>,
    algorithm: Algorithm,
}

impl JwtAuth {
    pub fn encode<T: Serialize>(&self, claims: &T) -> Result<String>;
    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<T>;
}

// Middleware
pub struct RequireAuth {
    jwt: JwtAuth,
}

impl Middleware for RequireAuth {
    async fn handle(&self, mut req: Request, next: Next) -> Response {
        let token = extract_token(&req)?;
        let claims = self.jwt.decode::<Claims>(&token)?;
        req.extensions_mut().insert(claims);
        next.run(req).await
    }
}
```

**Allowed Helpers:**
- `jsonwebtoken` for JWT (crypto is complex)
- `argon2` for password hashing (don't roll your own!)

#### 4.2 Authorization (Week 13)
- [ ] RBAC (Role-Based Access Control)
- [ ] Permission checking middleware
- [ ] Resource-level permissions
- [ ] Policy-based authorization

**Build From Scratch:**
```rust
// src/security/authz.rs
pub struct RbacMiddleware {
    required_role: Role,
}

impl Middleware for RbacMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Response {
        let claims = req.extensions().get::<Claims>()
            .ok_or(Error::Unauthorized)?;

        if !claims.has_role(&self.required_role) {
            return Response::forbidden();
        }

        next.run(req).await
    }
}
```

#### 4.3 CORS, CSRF, Rate Limiting (Week 14)
- [ ] CORS headers middleware
- [ ] CSRF token generation/validation
- [ ] Rate limiter (token bucket algorithm)
- [ ] IP-based throttling

**What You'll Learn:**
- Web security fundamentals
- Rate limiting algorithms
- Time-based state management

**Build From Scratch:**
```rust
// src/security/rate_limit.rs
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
    capacity: f64,
    refill_rate: f64,
}

impl RateLimiter {
    pub async fn check(&self, key: &str) -> Result<(), RateLimitError>;
}
```

---

### **Phase 5: Performance Features** (Weeks 15-17)

#### 5.1 In-Memory Cache (Week 15)
- [ ] LRU cache implementation
- [ ] TTL support
- [ ] Thread-safe access
- [ ] Cache invalidation strategies

**What You'll Learn:**
- Data structure implementation
- Concurrent access patterns
- Memory management

**Build From Scratch:**
```rust
// src/cache/memory.rs
pub struct LruCache<K, V> {
    map: HashMap<K, CacheEntry<V>>,
    list: LinkedList<K>,
    capacity: usize,
}

struct CacheEntry<V> {
    value: V,
    expires_at: Option<Instant>,
}

impl<K: Hash + Eq, V> LruCache<K, V> {
    pub fn get(&mut self, key: &K) -> Option<&V>;
    pub fn insert(&mut self, key: K, value: V, ttl: Option<Duration>);
    fn evict_oldest(&mut self);
}
```

#### 5.2 Redis Client (Week 16)
- [ ] RESP protocol implementation (Redis Serialization Protocol)
- [ ] Basic commands (GET, SET, DEL, EXPIRE)
- [ ] Connection pooling for Redis
- [ ] Pub/Sub support

**What You'll Learn:**
- Network protocol implementation
- Binary protocol parsing
- Client-server communication

**Build From Scratch:**
```rust
// src/cache/redis.rs
pub struct RedisClient {
    pool: Pool<TcpStream>,
}

impl RedisClient {
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    pub async fn set(&self, key: &str, value: &[u8], ttl: Option<u64>) -> Result<()>;
}

// RESP Protocol parser
fn encode_command(parts: &[&str]) -> Vec<u8> {
    // *3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n
}

fn decode_response(buf: &[u8]) -> Result<RespValue>;
```

**Allowed Helper:** `redis` crate (only if RESP is too complex initially)

#### 5.3 Compression (Week 17)
- [ ] Gzip compression middleware
- [ ] Brotli support
- [ ] Accept-Encoding negotiation
- [ ] Compression level configuration

**Allowed Helpers:**
- `flate2` for gzip
- `brotli` for brotli

---

### **Phase 6: Real-Time Features** (Weeks 18-20)

#### 6.1 WebSocket Support (Week 18-19)
- [ ] WebSocket handshake (HTTP upgrade)
- [ ] Frame parsing (RFC 6455)
- [ ] Message fragmentation
- [ ] Ping/pong keepalive
- [ ] Connection management

**What You'll Learn:**
- Protocol upgrade mechanisms
- Binary frame parsing
- Stateful connections
- Async stream handling

**Build From Scratch:**
```rust
// src/realtime/websocket.rs
pub struct WebSocket {
    stream: TcpStream,
    state: ConnectionState,
}

impl WebSocket {
    pub async fn accept(req: Request) -> Result<Self>;
    pub async fn send(&mut self, msg: Message) -> Result<()>;
    pub async fn recv(&mut self) -> Result<Option<Message>>;
}

pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}
```

**Allowed Helper:** `tokio-tungstenite` (if frame parsing is too complex)

#### 6.2 Server-Sent Events (Week 20)
- [ ] SSE response format
- [ ] Event streaming
- [ ] Reconnection handling
- [ ] Custom event types

**Build From Scratch:**
```rust
// src/realtime/sse.rs
pub struct SseStream {
    events: mpsc::Receiver<Event>,
}

pub struct Event {
    id: Option<String>,
    event: Option<String>,
    data: String,
}

impl SseStream {
    pub fn into_response(self) -> Response {
        Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(Body::from_stream(self))
            .unwrap()
    }
}
```

---

### **Phase 7: Background Processing** (Weeks 21-23)

#### 7.1 Task Queue (Week 21-22)
- [ ] Job queue implementation
- [ ] Worker pool
- [ ] Job persistence (DB-backed)
- [ ] Retry logic with backoff
- [ ] Job priorities

**What You'll Learn:**
- Queue data structures
- Worker pool patterns
- Distributed system basics
- Failure handling strategies

**Build From Scratch:**
```rust
// src/background/queue.rs
pub struct TaskQueue {
    pending: Arc<Mutex<VecDeque<Task>>>,
    workers: Vec<Worker>,
}

pub struct Task {
    id: String,
    payload: Vec<u8>,
    retry_count: u32,
    max_retries: u32,
}

impl TaskQueue {
    pub async fn enqueue(&self, task: Task) -> Result<()>;
    pub async fn start_workers(&mut self, count: usize);
}

struct Worker {
    handle: JoinHandle<()>,
}
```

#### 7.2 Scheduler (Week 23)
- [ ] Cron expression parser
- [ ] Scheduled task execution
- [ ] Task registration
- [ ] One-time and recurring tasks

**Build From Scratch:**
```rust
// src/background/scheduler.rs
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
}

struct ScheduledTask {
    schedule: CronExpr,
    handler: Box<dyn Fn() -> BoxFuture<'static, ()>>,
    next_run: DateTime<Utc>,
}

impl Scheduler {
    pub fn schedule<F>(&mut self, cron: &str, handler: F)
    where F: Fn() -> BoxFuture<'static, ()> + 'static;

    pub async fn run(&mut self);
}
```

---

### **Phase 8: AI/LLM Integration** (Weeks 24-28) 🤖

This is your framework's unique selling point!

#### 8.1 HTTP Client for LLM APIs (Week 24)
- [ ] Generic HTTP client
- [ ] Streaming response handling
- [ ] Request/response types for OpenAI
- [ ] Request/response types for Anthropic
- [ ] Error handling and retries

**Build From Scratch:**
```rust
// src/llm/client.rs
pub struct LlmClient {
    base_url: String,
    api_key: String,
    http_client: HttpClient,
}

pub struct StreamingResponse {
    stream: Pin<Box<dyn Stream<Item = Result<Chunk>>>>,
}

impl LlmClient {
    pub async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
    pub async fn stream(&self, req: CompletionRequest) -> Result<StreamingResponse>;
}
```

#### 8.2 Prompt Template Engine (Week 25)
- [ ] Template parsing (Jinja-like syntax)
- [ ] Variable interpolation
- [ ] Conditional rendering
- [ ] Loop support
- [ ] Template inheritance

**What You'll Learn:**
- Parser implementation
- Template engine design
- AST construction

**Build From Scratch:**
```rust
// src/llm/prompt.rs
pub struct PromptTemplate {
    template: String,
    variables: HashSet<String>,
}

impl PromptTemplate {
    pub fn parse(template: &str) -> Result<Self>;
    pub fn render(&self, context: &HashMap<String, Value>) -> Result<String>;
}

// Usage:
// let tmpl = PromptTemplate::parse("Hello {{name}}! You are {{age}} years old.")?;
// let result = tmpl.render(&context)?;
```

#### 8.3 Context Window & Token Management (Week 26)
- [ ] Token counting (BPE tokenizer)
- [ ] Context window tracking
- [ ] Automatic message truncation
- [ ] Token budget allocation

**What You'll Learn:**
- Tokenization algorithms
- Sliding window algorithms
- Memory-constrained processing

**Build From Scratch:**
```rust
// src/llm/tokens.rs
pub struct TokenCounter {
    // Simplified BPE tokenizer
    vocab: HashMap<String, usize>,
}

impl TokenCounter {
    pub fn count(&self, text: &str) -> usize;
    pub fn truncate(&self, text: &str, max_tokens: usize) -> String;
}

pub struct ContextWindow {
    messages: VecDeque<Message>,
    max_tokens: usize,
    current_tokens: usize,
}

impl ContextWindow {
    pub fn add_message(&mut self, msg: Message);
    fn evict_oldest(&mut self);
}
```

**Allowed Helper:** `tiktoken-rs` (OpenAI's tokenizer is complex)

#### 8.4 Vector Database Client (Week 27)
- [ ] Embedding storage interface
- [ ] Cosine similarity search
- [ ] In-memory vector store
- [ ] Client for external vector DBs (Pinecone, Weaviate)

**Build From Scratch:**
```rust
// src/llm/vector.rs
pub struct VectorStore {
    embeddings: Vec<(String, Vec<f32>)>, // (id, vector)
}

impl VectorStore {
    pub fn insert(&mut self, id: String, vector: Vec<f32>);
    pub fn search(&self, query: Vec<f32>, top_k: usize) -> Vec<(String, f32)>;
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (mag_a * mag_b)
}
```

#### 8.5 RAG Pipeline (Week 28)
- [ ] Document chunking
- [ ] Embedding generation
- [ ] Retrieval logic
- [ ] Context injection
- [ ] Response synthesis

**Build From Scratch:**
```rust
// src/llm/rag.rs
pub struct RagPipeline {
    vector_store: VectorStore,
    llm_client: LlmClient,
    chunk_size: usize,
}

impl RagPipeline {
    pub async fn index_document(&mut self, doc: &str) -> Result<()>;
    pub async fn query(&self, question: &str) -> Result<String>;
}

// Workflow:
// 1. Chunk document
// 2. Generate embeddings (call OpenAI embeddings API)
// 3. Store in vector DB
// 4. On query: retrieve relevant chunks
// 5. Inject into prompt
// 6. Call LLM
```

#### 8.6 Function Calling & Conversation History (Bonus)
- [ ] Function/tool definition schema
- [ ] Automatic function call parsing
- [ ] Conversation history storage
- [ ] Multi-turn conversation management

---

### **Phase 9: Developer Experience** (Weeks 29-31)

#### 9.1 CLI Tool (Week 29)
- [ ] Project scaffolding (`pttp new my-app`)
- [ ] Code generation (models, controllers)
- [ ] Migration runner
- [ ] Server commands

**Build From Scratch:**
```rust
// src/cli/mod.rs
pub struct Cli {
    command: Command,
}

enum Command {
    New { name: String },
    Generate { template: String, name: String },
    Serve { port: u16 },
    Migrate { direction: Direction },
}
```

**Allowed Helper:** `clap` (CLI parser)

#### 9.2 Hot Reload (Week 30)
- [ ] File watcher
- [ ] Server restart on code change
- [ ] State preservation (if possible)

**Allowed Helper:** `notify` (file system watcher)

#### 9.3 Testing Utilities (Week 31)
- [ ] Test client for HTTP requests
- [ ] Mock database
- [ ] Fixture loading
- [ ] Integration test helpers

**Build From Scratch:**
```rust
// src/testing/mod.rs
pub struct TestClient {
    server: TestServer,
}

impl TestClient {
    pub async fn get(&self, path: &str) -> TestResponse;
    pub async fn post(&self, path: &str, body: impl Serialize) -> TestResponse;
}
```

---

### **Phase 10: HTTP/2 & Optimization** (Weeks 32-34)

#### 10.1 HTTP/2 Support (Week 32-33)
- [ ] HTTP/2 frame parsing
- [ ] Stream multiplexing
- [ ] Server push
- [ ] Flow control

**What You'll Learn:**
- Binary protocol complexity
- Advanced async patterns
- Performance optimization

**Allowed Helper:** `h2` crate (HTTP/2 is extremely complex)

#### 10.2 Performance Tuning (Week 34)
- [ ] Connection keep-alive
- [ ] Zero-copy optimizations
- [ ] Buffer pooling
- [ ] Profiling and benchmarking

---

## 🎓 Learning Milestones

### After Phase 1 (Week 4):
You'll understand:
- Async I/O fundamentals
- TCP/HTTP protocol internals
- Task spawning and lifecycle

### After Phase 2 (Week 7):
You'll understand:
- Complex trait usage
- Generic programming
- Middleware patterns

### After Phase 3 (Week 11):
You'll understand:
- Concurrent data structures
- Resource management
- SQL and database internals

### After Phase 5 (Week 17):
You'll understand:
- Performance optimization
- Caching strategies
- Network protocol implementation

### After Phase 8 (Week 28):
You'll understand:
- AI/LLM integration patterns
- Vector search algorithms
- RAG architecture

---

## 📦 Minimal Dependency List

### Essential (Can't Avoid):
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }  # Async runtime
serde = { version = "1", features = ["derive"] }  # Serialization
serde_json = "1"  # JSON support
```

### Cryptography (Don't Roll Your Own):
```toml
argon2 = "0.5"  # Password hashing
jsonwebtoken = "9"  # JWT tokens
```

### Optional (Use Only If Building From Scratch Is Too Hard):
```toml
# HTTP/2 is extremely complex
h2 = "0.4"

# Database drivers (wire protocol is complex)
tokio-postgres = "0.7"
mysql_async = "0.34"

# WebSocket frames (RFC 6455 is detailed)
tokio-tungstenite = "0.21"

# Compression
flate2 = "1.0"
brotli = "6.0"

# CLI parsing
clap = { version = "4", features = ["derive"] }

# OpenAI tokenizer (BPE is complex)
tiktoken-rs = "0.5"
```

---

## 🚀 Getting Started

### Week 1 Action Items:

1. **Setup Development Environment:**
   ```bash
   rustup update stable
   cargo install cargo-watch
   cargo install cargo-edit
   ```

2. **Create Project:**
   ```bash
   cargo new pttp --lib
   cd pttp
   ```

3. **Initialize Git:**
   ```bash
   git init
   git add .
   git commit -m "Initial commit: Rust web framework project"
   ```

4. **Study Resources:**
   - Read Tokio tutorial: https://tokio.rs/tokio/tutorial
   - Study HTTP/1.1 spec: https://datatracker.ietf.org/doc/html/rfc2616
   - Review Axum source code: https://github.com/tokio-rs/axum

5. **First Code:**
   - Create module structure
   - Implement basic TCP server
   - Parse HTTP request line

---

## 📊 Success Metrics

### By End of Project:
- [ ] Can handle 10,000 concurrent connections
- [ ] Complete HTTP/1.1 and HTTP/2 support
- [ ] Working ORM with PostgreSQL/MySQL/SQLite
- [ ] JWT authentication + RBAC authorization
- [ ] WebSocket + SSE real-time features
- [ ] Background job queue + scheduler
- [ ] Full LLM integration with RAG pipeline
- [ ] Complete example application
- [ ] 80%+ test coverage

### Learning Metrics:
- [ ] Deep understanding of Rust ownership model
- [ ] Mastery of async programming
- [ ] Network protocol expertise
- [ ] Database internals knowledge
- [ ] Security best practices
- [ ] AI/LLM integration patterns

---

## 🎯 The Challenge

**This is a 6-8 month journey.** You will:
- Hit the borrow checker wall (many times)
- Debug async lifetime issues
- Implement complex algorithms
- Read RFCs and protocol specs
- Write thousands of lines of code

**But you will emerge as a Rust expert** with deep systems programming knowledge.

---

## 📖 Additional Resources

### Books:
- "Programming Rust" by Jim Blandy
- "Rust for Rustaceans" by Jon Gjengset
- "Asynchronous Programming in Rust"

### Code to Study:
- **Axum**: Type-safe routing and extractors
- **Actix-web**: High-performance patterns
- **Tokio**: Async runtime internals
- **Hyper**: HTTP implementation

### RFCs to Read:
- RFC 2616 (HTTP/1.1)
- RFC 7540 (HTTP/2)
- RFC 6455 (WebSocket)
- RFC 7519 (JWT)

---

## 🤝 Contributing to Your Learning

As you build, create:
1. **Documentation**: Explain your design decisions
2. **Tests**: Test every component
3. **Examples**: Show how to use each feature
4. **Benchmarks**: Measure performance
5. **Blog Posts**: Teach others what you learned

---

**Ready to start? Let's build Phase 0 together!**

Commands to begin:
```bash
# Update this README with your progress
# Check off items as you complete them
# Add notes and learnings
```

Good luck! 🦀🚀
