//! JWT-based authentication system
//!
//! Provides token generation, validation, and authentication middleware.
//!
//! # Example
//! ```rust,ignore
//! use pttp::security::auth::{JwtAuth, Claims};
//!
//! let auth = JwtAuth::new(b"secret-key");
//! let claims = Claims::new("user123", vec!["admin".to_string()]);
//! let token = auth.encode(&claims)?;
//! let decoded = auth.decode::<Claims>(&token)?;
//! ```

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::context::Context;
use crate::http::{Request, Response, StatusCode};
use crate::middleware::{Middleware, Next};
use std::future::Future;
use std::pin::Pin;

/// JWT authentication handler
#[derive(Clone)]
pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    validation: Validation,
}

impl JwtAuth {
    /// Create a new JWT authenticator with a secret key
    pub fn new(secret: &[u8]) -> Self {
        Self::with_algorithm(secret, Algorithm::HS256)
    }

    /// Create a new JWT authenticator with a specific algorithm
    pub fn with_algorithm(secret: &[u8], algorithm: Algorithm) -> Self {
        let mut validation = Validation::new(algorithm);
        validation.validate_exp = true;

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            algorithm,
            validation,
        }
    }

    /// Encode claims into a JWT token
    pub fn encode<T: Serialize>(&self, claims: &T) -> Result<String, AuthError> {
        let header = Header::new(self.algorithm);
        encode(&header, claims, &self.encoding_key).map_err(|e| AuthError::TokenCreation(e.to_string()))
    }

    /// Decode and validate a JWT token
    pub fn decode<T: for<'de> Deserialize<'de>>(&self, token: &str) -> Result<T, AuthError> {
        decode::<T>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }

    /// Extract token from Authorization header (Bearer scheme)
    pub fn extract_token(req: &Request) -> Result<String, AuthError> {
        let auth_str = req
            .headers()
            .get("Authorization")
            .or_else(|| req.headers().get("authorization"))
            .ok_or(AuthError::MissingToken)?;

        if !auth_str.starts_with("Bearer ") {
            return Err(AuthError::InvalidScheme);
        }

        Ok(auth_str[7..].to_string())
    }
}

/// Standard JWT claims with user ID and roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// User roles
    pub roles: Vec<String>,
    /// Additional custom data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Claims {
    /// Create new claims for a user with given roles
    /// Default expiration: 24 hours
    pub fn new(user_id: impl Into<String>, roles: Vec<String>) -> Self {
        Self::with_expiry(user_id, roles, 86400)
    }

    /// Create new claims with custom expiration (in seconds)
    pub fn with_expiry(user_id: impl Into<String>, roles: Vec<String>, expires_in: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            sub: user_id.into(),
            iat: now,
            exp: now + expires_in,
            roles,
            data: None,
        }
    }

    /// Add custom data to claims
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if the user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if the user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    /// No authentication token provided
    MissingToken,
    /// Invalid authentication scheme (expected Bearer)
    InvalidScheme,
    /// Token is invalid or expired
    InvalidToken(String),
    /// Token creation failed
    TokenCreation(String),
    /// User not found
    UserNotFound,
    /// Invalid credentials
    InvalidCredentials,
    /// Insufficient permissions
    Forbidden,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingToken => write!(f, "No authentication token provided"),
            Self::InvalidScheme => write!(f, "Invalid authentication scheme (expected Bearer)"),
            Self::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            Self::TokenCreation(msg) => write!(f, "Token creation failed: {}", msg),
            Self::UserNotFound => write!(f, "User not found"),
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
            Self::Forbidden => write!(f, "Insufficient permissions"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Middleware that requires valid JWT authentication
pub struct RequireAuth {
    jwt: JwtAuth,
}

impl RequireAuth {
    /// Create new authentication middleware
    pub fn new(jwt: JwtAuth) -> Self {
        Self { jwt }
    }
}

impl Middleware for RequireAuth {
    fn handle(&self, mut ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let jwt = self.jwt.clone();
        Box::pin(async move {
            // Extract and validate token
            let token = match JwtAuth::extract_token(ctx.request()) {
                Ok(token) => token,
                Err(e) => {
                    tracing::warn!("Authentication failed: {}", e);
                    return Response::builder()
                        .status(StatusCode::Unauthorized)
                        .header("Content-Type", "application/json")
                        .body(format!(r#"{{"error":"{}"}}"#, e))
                        .unwrap();
                }
            };

            let claims = match jwt.decode::<Claims>(&token) {
                Ok(claims) => claims,
                Err(e) => {
                    tracing::warn!("Token validation failed: {}", e);
                    return Response::builder()
                        .status(StatusCode::Unauthorized)
                        .header("Content-Type", "application/json")
                        .body(format!(r#"{{"error":"{}"}}"#, e))
                        .unwrap();
                }
            };

            // Store claims in context extensions for downstream handlers
            ctx.extensions_mut().insert(claims);

            next.run(ctx).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_encode_decode() {
        let auth = JwtAuth::new(b"test-secret");
        let claims = Claims::new("user123", vec!["admin".to_string()]);

        let token = auth.encode(&claims).expect("Failed to encode");
        let decoded: Claims = auth.decode(&token).expect("Failed to decode");

        assert_eq!(decoded.sub, "user123");
        assert_eq!(decoded.roles, vec!["admin"]);
    }

    #[test]
    fn test_jwt_invalid_secret() {
        let auth1 = JwtAuth::new(b"secret1");
        let auth2 = JwtAuth::new(b"secret2");

        let claims = Claims::new("user123", vec!["admin".to_string()]);
        let token = auth1.encode(&claims).expect("Failed to encode");

        // Should fail with different secret
        assert!(auth2.decode::<Claims>(&token).is_err());
    }

    #[test]
    fn test_claims_with_custom_expiry() {
        let claims = Claims::with_expiry("user123", vec!["user".to_string()], 3600);

        assert_eq!(claims.sub, "user123");
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.exp - claims.iat, 3600);
    }

    #[test]
    fn test_claims_with_data() {
        let data = serde_json::json!({"email": "user@example.com"});
        let claims = Claims::new("user123", vec!["user".to_string()])
            .with_data(data.clone());

        assert_eq!(claims.data, Some(data));
    }

    #[test]
    fn test_has_role() {
        let claims = Claims::new("user123", vec!["admin".to_string(), "moderator".to_string()]);

        assert!(claims.has_role("admin"));
        assert!(claims.has_role("moderator"));
        assert!(!claims.has_role("user"));
    }

    #[test]
    fn test_has_any_role() {
        let claims = Claims::new("user123", vec!["admin".to_string()]);

        assert!(claims.has_any_role(&["admin", "moderator"]));
        assert!(claims.has_any_role(&["user", "admin"]));
        assert!(!claims.has_any_role(&["user", "moderator"]));
    }

    #[test]
    fn test_has_all_roles() {
        let claims = Claims::new("user123", vec!["admin".to_string(), "moderator".to_string()]);

        assert!(claims.has_all_roles(&["admin", "moderator"]));
        assert!(claims.has_all_roles(&["admin"]));
        assert!(!claims.has_all_roles(&["admin", "moderator", "user"]));
    }

    #[test]
    fn test_extract_token_from_request() {
        use crate::http::Method;
        let mut req = Request::new(Method::GET, "/test".to_string());
        req.headers_mut().insert("authorization".to_string(), "Bearer token123".to_string());

        let token = JwtAuth::extract_token(&req).expect("Failed to extract token");
        assert_eq!(token, "token123");
    }

    #[test]
    fn test_extract_token_missing() {
        use crate::http::Method;
        let req = Request::new(Method::GET, "/test".to_string());
        assert!(matches!(JwtAuth::extract_token(&req), Err(AuthError::MissingToken)));
    }

    #[test]
    fn test_extract_token_invalid_scheme() {
        use crate::http::Method;
        let mut req = Request::new(Method::GET, "/test".to_string());
        req.headers_mut().insert("authorization".to_string(), "Basic dXNlcjpwYXNz".to_string());

        assert!(matches!(JwtAuth::extract_token(&req), Err(AuthError::InvalidScheme)));
    }
}
