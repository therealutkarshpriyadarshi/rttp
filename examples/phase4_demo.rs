//! Phase 4 Demo: Security Layer
//!
//! This example demonstrates all Phase 4 security features:
//! - JWT Authentication
//! - Password Hashing
//! - Session Management
//! - RBAC Authorization
//! - CSRF Protection
//! - Rate Limiting
//!
//! Run with:
//! ```bash
//! cargo run --example phase4_demo
//! ```

use pttp::prelude::*;
use pttp::security::*;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== PTTP Phase 4: Security Layer Demo ===\n");

    // Demo 1: JWT Authentication
    demo_jwt_auth();
    println!();

    // Demo 2: Password Hashing
    demo_password_hashing();
    println!();

    // Demo 3: Session Management
    demo_session_management().await;
    println!();

    // Demo 4: RBAC Authorization
    demo_rbac_authorization();
    println!();

    // Demo 5: CSRF Protection
    demo_csrf_protection().await;
    println!();

    // Demo 6: Rate Limiting
    demo_rate_limiting().await;
    println!();

    println!("=== All Phase 4 Features Demonstrated Successfully! ===");
}

/// Demo 1: JWT Authentication
fn demo_jwt_auth() {
    println!("📋 Demo 1: JWT Authentication");
    println!("------------------------------");

    // Create JWT authenticator
    let jwt = JwtAuth::new(b"my-secret-key");

    // Create claims for a user
    let claims = Claims::new("user123", vec!["admin".to_string(), "editor".to_string()])
        .with_data(serde_json::json!({
            "email": "user@example.com",
            "name": "John Doe"
        }));

    println!("✓ Created claims for user: {}", claims.sub);
    println!("  Roles: {:?}", claims.roles);

    // Encode JWT token
    let token = jwt.encode(&claims).expect("Failed to encode token");
    println!("✓ Generated JWT token: {}...", &token[..50]);

    // Decode and verify token
    let decoded: Claims = jwt.decode(&token).expect("Failed to decode token");
    println!("✓ Decoded token successfully");
    println!("  User ID: {}", decoded.sub);
    println!("  Has admin role: {}", decoded.has_role("admin"));
    println!("  Has any role [admin, user]: {}", decoded.has_any_role(&["admin", "user"]));
}

/// Demo 2: Password Hashing
fn demo_password_hashing() {
    println!("🔐 Demo 2: Password Hashing");
    println!("---------------------------");

    let hasher = PasswordHasher::new();
    let password = "SuperSecret123!";

    // Hash password
    let hash = hasher.hash_password(password).expect("Failed to hash password");
    println!("✓ Hashed password: {}...", &hash[..50]);

    // Verify correct password
    let is_valid = hasher.verify_password(password, &hash).expect("Failed to verify");
    println!("✓ Password verification (correct): {}", is_valid);

    // Verify wrong password
    let is_valid = hasher.verify_password("WrongPassword", &hash).expect("Failed to verify");
    println!("✓ Password verification (wrong): {}", is_valid);

    // Password validation
    let validator = PasswordValidator::new()
        .min_length(8)
        .require_uppercase(true)
        .require_digit(true);

    match validator.validate("weak") {
        Ok(_) => println!("✗ Weak password should fail"),
        Err(e) => println!("✓ Weak password rejected: {}", e),
    }

    match validator.validate("StrongPass123") {
        Ok(_) => println!("✓ Strong password accepted"),
        Err(e) => println!("✗ Strong password should pass: {}", e),
    }
}

/// Demo 3: Session Management
async fn demo_session_management() {
    println!("🎫 Demo 3: Session Management");
    println!("-----------------------------");

    let store = SessionStore::new();

    // Create a session
    let mut session = Session::new("user123");
    session.set("theme", "dark").expect("Failed to set theme");
    session.set("language", "en").expect("Failed to set language");

    let session_id = store.create(session).await;
    println!("✓ Created session: {}", session_id);

    // Retrieve session
    let retrieved = store.get(&session_id).await.expect("Failed to get session");
    println!("✓ Retrieved session for user: {}", retrieved.user_id);

    let theme: String = retrieved.get("theme").expect("Failed to get theme").unwrap();
    println!("  Theme: {}", theme);

    // Create multiple sessions for same user
    let session2 = Session::new("user123");
    store.create(session2).await;
    println!("✓ Created second session for same user");
    println!("  Total sessions: {}", store.count().await);

    // Delete all sessions for user
    store.delete_user_sessions("user123").await;
    println!("✓ Deleted all sessions for user123");
    println!("  Total sessions: {}", store.count().await);

    // Test session expiration
    let short_session = Session::with_expiry("user456", Duration::from_millis(100));
    let short_id = store.create(short_session).await;
    println!("✓ Created short-lived session");

    tokio::time::sleep(Duration::from_millis(150)).await;
    match store.get(&short_id).await {
        Err(SessionError::Expired) => println!("✓ Session expired correctly"),
        _ => println!("✗ Session should have expired"),
    }
}

/// Demo 4: RBAC Authorization
fn demo_rbac_authorization() {
    println!("👮 Demo 4: RBAC Authorization");
    println!("-----------------------------");

    // Create permissions
    let read_posts = Permission::new("posts", "read");
    let write_posts = Permission::new("posts", "write");
    let delete_posts = Permission::new("posts", "delete");

    println!("✓ Created permissions:");
    println!("  - {}", read_posts);
    println!("  - {}", write_posts);
    println!("  - {}", delete_posts);

    // Create roles
    let viewer_role = Role::new("viewer")
        .with_permission(read_posts.clone());

    let editor_role = Role::new("editor")
        .with_permission(read_posts.clone())
        .with_permission(write_posts.clone());

    let admin_role = Role::new("admin")
        .with_permissions(vec![read_posts.clone(), write_posts.clone(), delete_posts.clone()]);

    println!("✓ Created roles: viewer, editor, admin");

    // Create authorization policy
    let mut policy = AuthzPolicy::new();
    policy.add_role(viewer_role);
    policy.add_role(editor_role);
    policy.add_role(admin_role);

    println!("✓ Created authorization policy");

    // Check permissions
    let viewer_roles = vec!["viewer".to_string()];
    let editor_roles = vec!["editor".to_string()];
    let admin_roles = vec!["admin".to_string()];

    println!("\nPermission checks:");
    println!("  Viewer can read posts: {}", policy.check_permission(&viewer_roles, &read_posts));
    println!("  Viewer can write posts: {}", policy.check_permission(&viewer_roles, &write_posts));
    println!("  Editor can write posts: {}", policy.check_permission(&editor_roles, &write_posts));
    println!("  Editor can delete posts: {}", policy.check_permission(&editor_roles, &delete_posts));
    println!("  Admin can delete posts: {}", policy.check_permission(&admin_roles, &delete_posts));
}

/// Demo 5: CSRF Protection
async fn demo_csrf_protection() {
    println!("🛡️  Demo 5: CSRF Protection");
    println!("---------------------------");

    let csrf = CsrfProtection::new()
        .with_ttl(Duration::from_secs(3600))
        .with_header_name("X-CSRF-Token");

    println!("✓ Created CSRF protection handler");

    // Generate tokens
    let token1 = csrf.generate_token().await;
    let token2 = csrf.generate_token().await;

    println!("✓ Generated tokens:");
    println!("  Token 1: {}...", &token1.value()[..20]);
    println!("  Token 2: {}...", &token2.value()[..20]);

    // Validate tokens
    println!("\nToken validation:");
    println!("  Token 1 valid: {}", csrf.validate_token(token1.value()).await);
    println!("  Token 2 valid: {}", csrf.validate_token(token2.value()).await);
    println!("  Invalid token valid: {}", csrf.validate_token("invalid-token").await);

    // Invalidate token
    csrf.invalidate_token(token1.value()).await;
    println!("\n✓ Invalidated token 1");
    println!("  Token 1 valid: {}", csrf.validate_token(token1.value()).await);
}

/// Demo 6: Rate Limiting
async fn demo_rate_limiting() {
    println!("⏱️  Demo 6: Rate Limiting");
    println!("------------------------");

    // Create rate limiter (5 requests per 10 seconds)
    let config = RateLimitConfig::new(5, 10);
    let limiter = RateLimiter::new(config);

    println!("✓ Created rate limiter: 5 requests per 10 seconds");

    // Test rate limiting for a client
    let client_key = "client-ip-192.168.1.1";

    println!("\nTesting rate limit for {}:", client_key);
    for i in 1..=7 {
        match limiter.check(client_key).await {
            Ok(()) => println!("  Request {}: ✓ Allowed", i),
            Err(RateLimitError::LimitExceeded { retry_after }) => {
                println!("  Request {}: ✗ Rate limited (retry after {} secs)", i, retry_after.as_secs());
            }
        }
    }

    // Test different client
    let client2_key = "client-ip-192.168.1.2";
    println!("\nTesting different client {}:", client2_key);
    match limiter.check(client2_key).await {
        Ok(()) => println!("  Request 1: ✓ Allowed (independent bucket)"),
        Err(_) => println!("  Request 1: ✗ Should be allowed"),
    }

    // Test config presets
    println!("\nRate limit config presets:");
    let per_second = RateLimitConfig::per_second(10);
    let per_minute = RateLimitConfig::per_minute(60);
    let per_hour = RateLimitConfig::per_hour(1000);

    println!("  Per second: {} requests / {} secs", per_second.capacity, per_second.window.as_secs());
    println!("  Per minute: {} requests / {} secs", per_minute.capacity, per_minute.window.as_secs());
    println!("  Per hour: {} requests / {} secs", per_hour.capacity, per_hour.window.as_secs());
}
