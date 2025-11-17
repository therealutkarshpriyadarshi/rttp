//! Authentication, authorization, and security features
//!
//! This module provides comprehensive security features including:
//! - **JWT Authentication**: Token generation and validation
//! - **Password Hashing**: Secure password storage using Argon2
//! - **Session Management**: In-memory session storage
//! - **RBAC Authorization**: Role-based access control
//! - **CSRF Protection**: Cross-site request forgery prevention
//! - **Rate Limiting**: Token bucket-based rate limiting
//!
//! # Examples
//!
//! ## JWT Authentication
//! ```rust,ignore
//! use pttp::security::auth::{JwtAuth, Claims, RequireAuth};
//!
//! let jwt = JwtAuth::new(b"secret-key");
//! let claims = Claims::new("user123", vec!["admin".to_string()]);
//! let token = jwt.encode(&claims)?;
//!
//! // Use as middleware
//! let auth_middleware = RequireAuth::new(jwt);
//! ```
//!
//! ## Password Hashing
//! ```rust,ignore
//! use pttp::security::password::PasswordHasher;
//!
//! let hasher = PasswordHasher::new();
//! let hash = hasher.hash_password("secure-password")?;
//! assert!(hasher.verify_password("secure-password", &hash)?);
//! ```
//!
//! ## Session Management
//! ```rust,ignore
//! use pttp::security::session::{SessionStore, Session};
//!
//! let store = SessionStore::new();
//! let session = Session::new("user123");
//! let session_id = store.create(session).await;
//! ```
//!
//! ## RBAC Authorization
//! ```rust,ignore
//! use pttp::security::authz::{RequireRole, Permission, Role, AuthzPolicy};
//!
//! // Require specific role
//! let admin_middleware = RequireRole::new("admin");
//!
//! // Permission-based authorization
//! let mut policy = AuthzPolicy::new();
//! policy.add_role(
//!     Role::new("editor")
//!         .with_permission(Permission::new("posts", "write"))
//! );
//! ```
//!
//! ## CSRF Protection
//! ```rust,ignore
//! use pttp::security::csrf::{CsrfProtection, CsrfMiddleware};
//!
//! let csrf = CsrfProtection::new();
//! let token = csrf.generate_token().await;
//!
//! // Use as middleware
//! let csrf_middleware = CsrfMiddleware::new(csrf);
//! ```
//!
//! ## Rate Limiting
//! ```rust,ignore
//! use pttp::security::rate_limit::{RateLimiter, RateLimitConfig, RateLimitMiddleware};
//!
//! let config = RateLimitConfig::per_minute(60);
//! let limiter = RateLimiter::new(config);
//! let middleware = RateLimitMiddleware::new(limiter);
//! ```

pub mod auth;
pub mod authz;
pub mod csrf;
pub mod password;
pub mod rate_limit;
pub mod session;

// Re-export commonly used types
pub use auth::{AuthError, Claims, JwtAuth, RequireAuth};
pub use authz::{AuthzError, AuthzPolicy, Permission, RequireAllRoles, RequireAnyRole, RequireRole, Role};
pub use csrf::{CsrfMiddleware, CsrfProtection, CsrfToken};
pub use password::{PasswordError, PasswordHasher, PasswordValidator};
pub use rate_limit::{KeyExtractor, RateLimitConfig, RateLimitError, RateLimitMiddleware, RateLimiter};
pub use session::{Session, SessionError, SessionStore};
