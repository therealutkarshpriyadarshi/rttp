//! In-memory session management
//!
//! Provides session storage and management for user authentication state.
//!
//! # Example
//! ```rust,ignore
//! use pttp::security::session::{SessionStore, Session};
//!
//! let store = SessionStore::new();
//! let session = Session::new("user123");
//! let session_id = store.create(session).await;
//! let retrieved = store.get(&session_id).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// User ID associated with this session
    pub user_id: String,
    /// Session creation time
    pub created_at: SystemTime,
    /// Session last accessed time
    pub last_accessed: SystemTime,
    /// Session expiration time
    pub expires_at: SystemTime,
    /// Custom session data
    pub data: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session for a user
    /// Default expiration: 7 days
    pub fn new(user_id: impl Into<String>) -> Self {
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(7 * 24 * 60 * 60); // 7 days

        Self {
            user_id: user_id.into(),
            created_at: now,
            last_accessed: now,
            expires_at,
            data: HashMap::new(),
        }
    }

    /// Create a new session with custom expiration duration
    pub fn with_expiry(user_id: impl Into<String>, duration: Duration) -> Self {
        let now = SystemTime::now();
        let expires_at = now + duration;

        Self {
            user_id: user_id.into(),
            created_at: now,
            last_accessed: now,
            expires_at,
            data: HashMap::new(),
        }
    }

    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
    }

    /// Set custom data
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) -> Result<(), SessionError> {
        let json_value = serde_json::to_value(value).map_err(|e| SessionError::SerializationError(e.to_string()))?;
        self.data.insert(key.into(), json_value);
        Ok(())
    }

    /// Get custom data
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, SessionError> {
        match self.data.get(key) {
            Some(value) => {
                let data = serde_json::from_value(value.clone())
                    .map_err(|e| SessionError::DeserializationError(e.to_string()))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Remove custom data
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }
}

/// In-memory session store
#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionStore {
    /// Create a new session store
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return its ID
    pub async fn create(&self, session: Session) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        session_id
    }

    /// Get a session by ID
    pub async fn get(&self, session_id: &str) -> Result<Session, SessionError> {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(session_id) {
            Some(session) => {
                if session.is_expired() {
                    sessions.remove(session_id);
                    return Err(SessionError::Expired);
                }

                session.touch();
                Ok(session.clone())
            }
            None => Err(SessionError::NotFound),
        }
    }

    /// Update a session
    pub async fn update(&self, session_id: &str, session: Session) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;

        if !sessions.contains_key(session_id) {
            return Err(SessionError::NotFound);
        }

        sessions.insert(session_id.to_string(), session);
        Ok(())
    }

    /// Delete a session
    pub async fn delete(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).ok_or(SessionError::NotFound)?;
        Ok(())
    }

    /// Delete all sessions for a user
    pub async fn delete_user_sessions(&self, user_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| session.user_id != user_id);
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before_count = sessions.len();

        sessions.retain(|_, session| !session.is_expired());

        let after_count = sessions.len();
        before_count - after_count
    }

    /// Get total number of active sessions
    pub async fn count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Check if a session exists and is valid
    pub async fn exists(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        match sessions.get(session_id) {
            Some(session) => !session.is_expired(),
            None => false,
        }
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Session-related errors
#[derive(Debug, Clone)]
pub enum SessionError {
    /// Session not found
    NotFound,
    /// Session has expired
    Expired,
    /// Serialization error
    SerializationError(String),
    /// Deserialization error
    DeserializationError(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Session not found"),
            Self::Expired => write!(f, "Session has expired"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for SessionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let store = SessionStore::new();
        let session = Session::new("user123");

        let session_id = store.create(session.clone()).await;
        let retrieved = store.get(&session_id).await.expect("Failed to get session");

        assert_eq!(retrieved.user_id, "user123");
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() {
        let store = SessionStore::new();
        let result = store.get("nonexistent").await;

        assert!(matches!(result, Err(SessionError::NotFound)));
    }

    #[tokio::test]
    async fn test_update_session() {
        let store = SessionStore::new();
        let mut session = Session::new("user123");

        let session_id = store.create(session.clone()).await;

        session.data.insert("key".to_string(), serde_json::json!("value"));
        store.update(&session_id, session).await.expect("Failed to update");

        let retrieved = store.get(&session_id).await.expect("Failed to get session");
        assert_eq!(retrieved.data.get("key"), Some(&serde_json::json!("value")));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = SessionStore::new();
        let session = Session::new("user123");

        let session_id = store.create(session).await;
        store.delete(&session_id).await.expect("Failed to delete");

        let result = store.get(&session_id).await;
        assert!(matches!(result, Err(SessionError::NotFound)));
    }

    #[tokio::test]
    async fn test_delete_user_sessions() {
        let store = SessionStore::new();

        let session1 = Session::new("user123");
        let session2 = Session::new("user123");
        let session3 = Session::new("user456");

        store.create(session1).await;
        store.create(session2).await;
        let session3_id = store.create(session3).await;

        store.delete_user_sessions("user123").await;

        assert_eq!(store.count().await, 1);
        assert!(store.get(&session3_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let store = SessionStore::new();
        let session = Session::with_expiry("user123", Duration::from_millis(100));

        let session_id = store.create(session).await;

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        let result = store.get(&session_id).await;
        assert!(matches!(result, Err(SessionError::Expired)));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let store = SessionStore::new();

        let session1 = Session::with_expiry("user1", Duration::from_millis(100));
        let session2 = Session::with_expiry("user2", Duration::from_secs(3600));

        store.create(session1).await;
        store.create(session2).await;

        // Wait for first session to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        let cleaned = store.cleanup_expired().await;
        assert_eq!(cleaned, 1);
        assert_eq!(store.count().await, 1);
    }

    #[test]
    fn test_session_data() {
        let mut session = Session::new("user123");

        session.set("email", "user@example.com").expect("Failed to set");
        session.set("age", 25).expect("Failed to set");

        let email: String = session.get("email").expect("Failed to get").unwrap();
        let age: i32 = session.get("age").expect("Failed to get").unwrap();

        assert_eq!(email, "user@example.com");
        assert_eq!(age, 25);
    }

    #[test]
    fn test_session_remove_data() {
        let mut session = Session::new("user123");

        session.set("key", "value").expect("Failed to set");
        assert!(session.get::<String>("key").unwrap().is_some());

        session.remove("key");
        assert!(session.get::<String>("key").unwrap().is_none());
    }

    #[test]
    fn test_session_is_expired() {
        let expired = Session::with_expiry("user123", Duration::from_secs(0));
        let valid = Session::with_expiry("user456", Duration::from_secs(3600));

        // Give a small delay to ensure the first session expires
        std::thread::sleep(Duration::from_millis(10));

        assert!(expired.is_expired());
        assert!(!valid.is_expired());
    }
}
