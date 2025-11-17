# PTTP - Pure Rust Web Framework with AI/LLM Integration

> **Learning Project:** Building a production-grade web framework from near-scratch in Rust to maximize learning about systems programming, async I/O, networking, and AI integration.

## 🎯 Project Overview

PTTP is a web framework built with minimal dependencies to understand fundamentals. We use only essential libraries (tokio for async runtime, serde for serialization) and build most components from scratch.

## 🏗️ Architecture

```
pttp/
├── src/
│   ├── lib.rs           # Public API
│   ├── http/            # HTTP protocol ✅ COMPLETE
│   ├── server/          # TCP server ✅ COMPLETE
│   ├── router/          # Request routing ✅ COMPLETE
│   ├── middleware/      # Middleware system ✅ COMPLETE
│   ├── context/         # Request context ✅ COMPLETE
│   ├── database/        # Database layer ✅ COMPLETE
│   ├── security/        # Auth & security ✅ COMPLETE
│   ├── cache/           # Caching layer 📅 PLANNED
│   ├── realtime/        # WebSocket/SSE 📅 PLANNED
│   ├── background/      # Task queue 📅 PLANNED
│   └── llm/             # AI/LLM integration 📅 PLANNED
└── examples/            # Usage examples
```

## 📦 Dependencies

### Essential
- `tokio` - Async runtime
- `serde` & `serde_json` - Serialization
- `tracing` - Logging infrastructure
- `uuid` - Request ID generation

### Database (Phase 3)
- `tokio-postgres` - PostgreSQL wire protocol
- `postgres-types` - PostgreSQL type mappings

### Security (Phase 4)
- `jsonwebtoken` - JWT token generation/validation
- `argon2` - Password hashing
- `rand` - Cryptographically secure random numbers
- `base64` - Base64 encoding/decoding

### Minimal Philosophy
We build core components from scratch. External crates are only used where:
1. Complexity is extremely high (e.g., HTTP/2, cryptography)
2. Security is critical (e.g., password hashing, JWT)
3. Standards compliance is complex (e.g., database wire protocols)

## 🚀 Quick Start

### Prerequisites
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
rustup update stable
cargo install cargo-watch
```

### Build and Run

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run hello world example
cargo run --example hello_world

# Run Phase 2 demo (showcases router & middleware)
cargo run --example phase2_demo

# Run Phase 3 demo (showcases database layer)
# Note: Requires PostgreSQL database
cargo run --example phase3_demo

# Run Phase 4 demo (showcases security features)
cargo run --example phase4_demo

# Run with hot reload (development)
cargo watch -x 'run --example phase2_demo'

# Run clippy for lints
cargo clippy

# Format code
cargo fmt
```

## 📊 Progress Tracker

### ✅ Phase 0: Foundation Setup (COMPLETED)
- [x] Initialize Cargo workspace
- [x] Setup project structure (lib + examples)
- [x] Configure development tools (rustfmt, clippy)
- [x] Create basic module hierarchy
- [x] Setup logging infrastructure

### ✅ Phase 1: HTTP Server Core (COMPLETED)
- [x] Raw TCP listener with tokio::net::TcpListener
- [x] Accept connections and spawn tasks
- [x] HTTP/1.1 request parsing
- [x] Request/Response abstractions
- [x] Basic request handling

### ✅ Phase 2: Router & Middleware (COMPLETED)
- [x] Pattern matching router (exact, parameterized, wildcard)
- [x] Path parameter extraction
- [x] Middleware system with chaining (onion model)
- [x] Request context with type-safe extensions
- [x] Built-in middleware (Logger, CORS, RequestID)
- [x] Query parameter handling

### ✅ Phase 3: Database Layer (COMPLETED)
- [x] Connection pooling
- [x] Query builder
- [x] ORM features
- [x] Transaction management

### ✅ Phase 4: Security Layer (COMPLETED)
- [x] JWT authentication (token generation, validation)
- [x] Password hashing (Argon2)
- [x] Session management (in-memory backend)
- [x] RBAC authorization (roles, permissions, policies)
- [x] Authentication middleware (RequireAuth)
- [x] Authorization middleware (RequireRole, RequirePermission)
- [x] CSRF protection
- [x] Rate limiting (token bucket algorithm)

### 📅 Phase 5: Performance Features (PLANNED)
- [ ] In-memory cache (LRU)
- [ ] Redis client
- [ ] Compression

### 📅 Phase 6: Real-Time Features (PLANNED)
- [ ] WebSocket support
- [ ] Server-Sent Events

### 📅 Phase 7: Background Processing (PLANNED)
- [ ] Task queue
- [ ] Job scheduler

### 📅 Phase 8: AI/LLM Integration (PLANNED)
- [ ] HTTP client for LLM APIs
- [ ] Prompt template engine
- [ ] Token management
- [ ] Vector database
- [ ] RAG pipeline

## 🎓 Learning Resources

### Recommended Reading
- [The Rust Book](https://doc.rust-lang.org/book/) - Chapters 1-10, 13, 15-16, 19
- [Async Book](https://rust-lang.github.io/async-book/) - Understanding Future, async/await, Pin
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Runtime basics, tasks, I/O

### Key Concepts
- Ownership, borrowing, lifetimes
- Trait system and generics
- Error handling (Result, ? operator)
- Async programming model
- Smart pointers (Box, Arc, Rc)
- Interior mutability (RefCell, Mutex)

### Code References
- [Axum](https://github.com/tokio-rs/axum) - Type-safe routing
- [Actix-web](https://github.com/actix/actix-web) - High-performance patterns
- [Hyper](https://github.com/hyperium/hyper) - HTTP implementation

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in specific module
cargo test http::

# Run with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## 🔧 Development

### Code Style
- We use `rustfmt` for consistent formatting
- Configuration in `rustfmt.toml`
- Run `cargo fmt` before committing

### Linting
- We use `clippy` for additional lints
- Configuration in `clippy.toml`
- Run `cargo clippy` to check

### Commit Convention
```
<type>: <description>

Types:
- feat: New feature
- fix: Bug fix
- docs: Documentation changes
- refactor: Code refactoring
- test: Test additions/changes
- chore: Maintenance tasks
```

## 📝 Current Status

**Phase 3 Completed!** ✅

The Database Layer is fully operational:

**Phase 1 - HTTP Server Core:**
- ✅ Project structure established
- ✅ Module hierarchy created
- ✅ Development tools configured
- ✅ Logging infrastructure ready
- ✅ HTTP types implemented (Method, StatusCode, Version)
- ✅ HTTP/1.1 request parser built from scratch
- ✅ TCP connection handling with async I/O
- ✅ Request/response lifecycle complete

**Phase 2 - Router & Middleware:**
- ✅ Pattern matching router (exact, parameterized, wildcard)
- ✅ Path parameter extraction (`/users/:id`)
- ✅ Middleware system with onion model chaining
- ✅ Built-in middleware (Logger, CORS, RequestID)
- ✅ Request context with type-safe extensions
- ✅ Query parameter handling
- ✅ Comprehensive test coverage (45 unit tests)

**Phase 3 - Database Layer:**
- ✅ Custom connection pool with health checks and idle timeout
- ✅ Type-safe query builder for SELECT, INSERT, UPDATE, DELETE
- ✅ ORM features with Model trait and row mapping
- ✅ Transaction management with automatic rollback on drop
- ✅ Support for nested transactions (savepoints)
- ✅ Parameterized queries to prevent SQL injection
- ✅ PostgreSQL integration with tokio-postgres

**Working Features:**
- HTTP/1.1 protocol support
- Advanced routing with path parameters (`/users/:id`, `/users/:user_id/posts/:post_id`)
- Wildcard routes (`/files/*`)
- Method-based routing (GET, POST, PUT, DELETE, PATCH)
- Middleware chaining with before/after hooks
- Type-safe request context and extensions
- Query parameter extraction
- JSON and HTML responses
- Database connection pooling with automatic health checks
- Type-safe SQL query builder with parameter binding
- ORM with Model trait for struct-to-table mapping
- ACID transactions with automatic rollback
- Proper error handling and edge cases

**Next Steps (Phase 4):**
1. Implement JWT authentication
2. Build session management
3. Add RBAC authorization
4. Create security middleware (CSRF, rate limiting)

## 🎯 Success Metrics (End Goal)

- [ ] Handle 10,000 concurrent connections
- [ ] Complete HTTP/1.1 and HTTP/2 support
- [ ] Working ORM with PostgreSQL/MySQL/SQLite
- [ ] JWT authentication + RBAC authorization
- [ ] WebSocket + SSE real-time features
- [ ] Background job queue + scheduler
- [ ] Full LLM integration with RAG pipeline
- [ ] Complete example application
- [ ] 80%+ test coverage

## 📄 License

MIT

## 🤝 Contributing

This is a learning project! Contributions, suggestions, and feedback are welcome.

---

**Built with 🦀 Rust and ❤️ for learning**
