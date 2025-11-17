//! Role-Based Access Control (RBAC) authorization
//!
//! Provides authorization middleware and utilities for role and permission checking.
//!
//! # Example
//! ```rust,ignore
//! use pttp::security::authz::{RequireRole, RequirePermission, Permission};
//!
//! // Require admin role
//! let admin_middleware = RequireRole::new("admin");
//!
//! // Require specific permission
//! let permission_middleware = RequirePermission::new(Permission::new("posts", "write"));
//! ```

use crate::context::Context;
use crate::http::{Response, StatusCode};
use crate::middleware::{Middleware, Next};
use crate::security::auth::Claims;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Permission definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// Resource name (e.g., "posts", "users", "comments")
    pub resource: String,
    /// Action (e.g., "read", "write", "delete", "admin")
    pub action: String,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }

    /// Parse permission from string format "resource:action"
    pub fn parse(s: &str) -> Result<Self, AuthzError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(AuthzError::InvalidPermission(s.to_string()));
        }

        Ok(Self {
            resource: parts[0].to_string(),
            action: parts[1].to_string(),
        })
    }

    /// Convert permission to string format "resource:action"
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.resource, self.action)
    }
}

/// Role definition with permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Permissions granted to this role
    pub permissions: Vec<Permission>,
}

impl Role {
    /// Create a new role
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            permissions: Vec::new(),
        }
    }

    /// Add a permission to this role
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.push(permission);
        self
    }

    /// Add multiple permissions to this role
    pub fn with_permissions(mut self, permissions: Vec<Permission>) -> Self {
        self.permissions.extend(permissions);
        self
    }

    /// Check if this role has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }
}

/// Policy-based authorization checker
#[derive(Clone)]
pub struct AuthzPolicy {
    roles: HashMap<String, Role>,
}

impl AuthzPolicy {
    /// Create a new authorization policy
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
        }
    }

    /// Add a role to the policy
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }

    /// Check if a user with given roles has a specific permission
    pub fn check_permission(&self, user_roles: &[String], permission: &Permission) -> bool {
        user_roles.iter().any(|role_name| {
            self.roles
                .get(role_name)
                .map(|role| role.has_permission(permission))
                .unwrap_or(false)
        })
    }

    /// Get all permissions for a user's roles
    pub fn get_user_permissions(&self, user_roles: &[String]) -> Vec<Permission> {
        let mut permissions = Vec::new();
        for role_name in user_roles {
            if let Some(role) = self.roles.get(role_name) {
                permissions.extend(role.permissions.clone());
            }
        }
        permissions
    }
}

impl Default for AuthzPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware that requires a specific role
pub struct RequireRole {
    required_role: String,
}

impl RequireRole {
    /// Create new role requirement middleware
    pub fn new(role: impl Into<String>) -> Self {
        Self {
            required_role: role.into(),
        }
    }
}

impl Middleware for RequireRole {
    fn handle(&self, ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let required_role = self.required_role.clone();
        Box::pin(async move {
            // Get claims from context extensions (set by RequireAuth middleware)
            let claims = match ctx.extensions().get::<Claims>() {
                Some(claims) => claims,
                None => {
                    tracing::warn!("No authentication claims found in request");
                    return Response::builder()
                        .status(StatusCode::Unauthorized)
                        .header("Content-Type", "application/json")
                        .body(r#"{"error":"Authentication required"}"#)
                        .unwrap();
                }
            };

            // Check if user has the required role
            if !claims.has_role(&required_role) {
                tracing::warn!("User {} lacks required role: {}", claims.sub, required_role);
                return Response::builder()
                    .status(StatusCode::Forbidden)
                    .header("Content-Type", "application/json")
                    .body(format!(r#"{{"error":"Insufficient permissions: role '{}' required"}}"#, required_role))
                    .unwrap();
            }

            next.run(ctx).await
        })
    }
}

/// Middleware that requires any of the specified roles
pub struct RequireAnyRole {
    required_roles: Vec<String>,
}

impl RequireAnyRole {
    /// Create new role requirement middleware (requires any of the specified roles)
    pub fn new(roles: Vec<String>) -> Self {
        Self {
            required_roles: roles,
        }
    }
}

impl Middleware for RequireAnyRole {
    fn handle(&self, ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let required_roles = self.required_roles.clone();
        Box::pin(async move {
            let claims = match ctx.extensions().get::<Claims>() {
                Some(claims) => claims,
                None => {
                    return Response::builder()
                        .status(StatusCode::Unauthorized)
                        .header("Content-Type", "application/json")
                        .body(r#"{"error":"Authentication required"}"#)
                        .unwrap();
                }
            };

            let role_refs: Vec<&str> = required_roles.iter().map(|s| s.as_str()).collect();
            if !claims.has_any_role(&role_refs) {
                tracing::warn!("User {} lacks any of required roles: {:?}", claims.sub, required_roles);
                return Response::builder()
                    .status(StatusCode::Forbidden)
                    .header("Content-Type", "application/json")
                    .body(r#"{"error":"Insufficient permissions"}"#)
                    .unwrap();
            }

            next.run(ctx).await
        })
    }
}

/// Middleware that requires all of the specified roles
pub struct RequireAllRoles {
    required_roles: Vec<String>,
}

impl RequireAllRoles {
    /// Create new role requirement middleware (requires all specified roles)
    pub fn new(roles: Vec<String>) -> Self {
        Self {
            required_roles: roles,
        }
    }
}

impl Middleware for RequireAllRoles {
    fn handle(&self, ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let required_roles = self.required_roles.clone();
        Box::pin(async move {
            let claims = match ctx.extensions().get::<Claims>() {
                Some(claims) => claims,
                None => {
                    return Response::builder()
                        .status(StatusCode::Unauthorized)
                        .header("Content-Type", "application/json")
                        .body(r#"{"error":"Authentication required"}"#)
                        .unwrap();
                }
            };

            let role_refs: Vec<&str> = required_roles.iter().map(|s| s.as_str()).collect();
            if !claims.has_all_roles(&role_refs) {
                tracing::warn!("User {} lacks all required roles: {:?}", claims.sub, required_roles);
                return Response::builder()
                    .status(StatusCode::Forbidden)
                    .header("Content-Type", "application/json")
                    .body(r#"{"error":"Insufficient permissions: all roles required"}"#)
                    .unwrap();
            }

            next.run(ctx).await
        })
    }
}

/// Middleware that requires a specific permission
pub struct RequirePermission {
    required_permission: Permission,
    policy: AuthzPolicy,
}

impl RequirePermission {
    /// Create new permission requirement middleware
    pub fn new(permission: Permission, policy: AuthzPolicy) -> Self {
        Self {
            required_permission: permission,
            policy,
        }
    }
}

impl Middleware for RequirePermission {
    fn handle(&self, ctx: Context, next: Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        let policy = self.policy.clone();
        let required_permission = self.required_permission.clone();
        Box::pin(async move {
            let claims = match ctx.extensions().get::<Claims>() {
                Some(claims) => claims,
                None => {
                    return Response::builder()
                        .status(StatusCode::Unauthorized)
                        .header("Content-Type", "application/json")
                        .body(r#"{"error":"Authentication required"}"#)
                        .unwrap();
                }
            };

            // Check if user has the required permission through their roles
            if !policy.check_permission(&claims.roles, &required_permission) {
                tracing::warn!(
                    "User {} lacks required permission: {}",
                    claims.sub,
                    required_permission
                );
                return Response::builder()
                    .status(StatusCode::Forbidden)
                    .header("Content-Type", "application/json")
                    .body(format!(
                        r#"{{"error":"Insufficient permissions: '{}' required"}}"#,
                        required_permission
                    ))
                    .unwrap();
            }

            next.run(ctx).await
        })
    }
}

/// Authorization errors
#[derive(Debug, Clone)]
pub enum AuthzError {
    /// Invalid permission format
    InvalidPermission(String),
    /// Role not found
    RoleNotFound(String),
    /// Permission denied
    PermissionDenied(String),
}

impl std::fmt::Display for AuthzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPermission(msg) => write!(f, "Invalid permission: {}", msg),
            Self::RoleNotFound(role) => write!(f, "Role not found: {}", role),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for AuthzError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new("posts", "read");
        assert_eq!(perm.resource, "posts");
        assert_eq!(perm.action, "read");
    }

    #[test]
    fn test_permission_parse() {
        let perm = Permission::parse("posts:write").expect("Failed to parse");
        assert_eq!(perm.resource, "posts");
        assert_eq!(perm.action, "write");
    }

    #[test]
    fn test_permission_parse_invalid() {
        assert!(Permission::parse("invalid").is_err());
        assert!(Permission::parse("too:many:parts").is_err());
    }

    #[test]
    fn test_permission_to_string() {
        let perm = Permission::new("users", "delete");
        assert_eq!(perm.to_string(), "users:delete");
    }

    #[test]
    fn test_role_with_permissions() {
        let role = Role::new("admin")
            .with_permission(Permission::new("posts", "read"))
            .with_permission(Permission::new("posts", "write"));

        assert_eq!(role.name, "admin");
        assert_eq!(role.permissions.len(), 2);
        assert!(role.has_permission(&Permission::new("posts", "read")));
        assert!(!role.has_permission(&Permission::new("posts", "delete")));
    }

    #[test]
    fn test_authz_policy() {
        let mut policy = AuthzPolicy::new();

        let admin_role = Role::new("admin")
            .with_permission(Permission::new("posts", "read"))
            .with_permission(Permission::new("posts", "write"))
            .with_permission(Permission::new("posts", "delete"));

        let user_role = Role::new("user").with_permission(Permission::new("posts", "read"));

        policy.add_role(admin_role);
        policy.add_role(user_role);

        // Admin should have all permissions
        assert!(policy.check_permission(&vec!["admin".to_string()], &Permission::new("posts", "write")));

        // User should only have read permission
        assert!(policy.check_permission(&vec!["user".to_string()], &Permission::new("posts", "read")));
        assert!(!policy.check_permission(&vec!["user".to_string()], &Permission::new("posts", "write")));
    }

    #[test]
    fn test_authz_policy_multiple_roles() {
        let mut policy = AuthzPolicy::new();

        policy.add_role(Role::new("reader").with_permission(Permission::new("posts", "read")));
        policy.add_role(Role::new("writer").with_permission(Permission::new("posts", "write")));

        // User with both roles should have both permissions
        let user_roles = vec!["reader".to_string(), "writer".to_string()];
        assert!(policy.check_permission(&user_roles, &Permission::new("posts", "read")));
        assert!(policy.check_permission(&user_roles, &Permission::new("posts", "write")));
    }

    #[test]
    fn test_get_user_permissions() {
        let mut policy = AuthzPolicy::new();

        policy.add_role(
            Role::new("admin")
                .with_permission(Permission::new("posts", "read"))
                .with_permission(Permission::new("posts", "write")),
        );

        let permissions = policy.get_user_permissions(&vec!["admin".to_string()]);
        assert_eq!(permissions.len(), 2);
        assert!(permissions.contains(&Permission::new("posts", "read")));
        assert!(permissions.contains(&Permission::new("posts", "write")));
    }
}
