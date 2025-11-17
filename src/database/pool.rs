//! Connection pool implementation
//!
//! This module provides a custom connection pool for PostgreSQL that manages
//! connection lifecycle, health checks, and resource cleanup.

use crate::database::config::Config;
use crate::database::error::{Error, Result};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio_postgres::{Client, NoTls};

/// Connection wrapper with metadata
struct PooledConn {
    /// The actual database client
    client: Client,

    /// When this connection was created
    created_at: Instant,

    /// When this connection was last used
    last_used: Instant,
}

impl PooledConn {
    /// Create a new pooled connection
    fn new(client: Client) -> Self {
        let now = Instant::now();
        Self {
            client,
            created_at: now,
            last_used: now,
        }
    }

    /// Check if connection is idle for too long
    fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }

    /// Update last used timestamp
    fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    /// Check if connection is still healthy
    async fn is_healthy(&self) -> bool {
        // Simple health check: try to execute a simple query
        self.client.simple_query("SELECT 1").await.is_ok()
    }
}

/// Connection pool for database connections
pub struct Pool {
    /// Configuration
    config: Config,

    /// Available connections
    connections: Arc<Mutex<VecDeque<PooledConn>>>,

    /// Semaphore to limit concurrent connections
    semaphore: Arc<Semaphore>,

    /// Background task handle for cleanup
    _cleanup_handle: tokio::task::JoinHandle<()>,
}

impl Pool {
    /// Create a new connection pool
    pub async fn new(config: Config) -> Result<Self> {
        let max_connections = config.max_connections;
        let connections = Arc::new(Mutex::new(VecDeque::new()));
        let semaphore = Arc::new(Semaphore::new(max_connections));

        // Start background cleanup task
        let cleanup_handle = {
            let connections = Arc::clone(&connections);
            let idle_timeout = config.idle_timeout;
            tokio::spawn(async move {
                Self::cleanup_loop(connections, idle_timeout).await;
            })
        };

        Ok(Self {
            config,
            connections,
            semaphore,
            _cleanup_handle: cleanup_handle,
        })
    }

    /// Get a connection from the pool
    pub async fn get(&self) -> Result<PooledConnection> {
        // Acquire semaphore permit
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::Pool(format!("Failed to acquire permit: {}", e)))?;

        // Try to get an existing connection
        let conn = {
            let mut connections = self.connections.lock().await;
            connections.pop_front()
        };

        // If no connection available or unhealthy, create a new one
        if let Some(mut c) = conn {
            // Check if connection is healthy
            if c.is_healthy().await {
                c.touch();
                return Ok(PooledConnection {
                    conn: Some(c),
                    pool: Arc::clone(&self.connections),
                    _permit: permit,
                });
            }
            // Connection is unhealthy, drop it and create new one
            tracing::warn!("Unhealthy connection detected, creating new one");
        }

        // Create new connection
        let client = self.create_connection().await?;
        let pooled_conn = PooledConn::new(client);

        Ok(PooledConnection {
            conn: Some(pooled_conn),
            pool: Arc::clone(&self.connections),
            _permit: permit,
        })
    }

    /// Create a new database connection
    async fn create_connection(&self) -> Result<Client> {
        let config_str = self.config.connection_string();

        let (client, connection) = tokio_postgres::connect(&config_str, NoTls)
            .await
            .map_err(|e| Error::Connection(format!("Failed to connect: {}", e)))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Connection error: {}", e);
            }
        });

        // Test connection
        client
            .simple_query("SELECT 1")
            .await
            .map_err(|e| Error::Connection(format!("Connection test failed: {}", e)))?;

        tracing::info!("Created new database connection");

        Ok(client)
    }

    /// Background task to cleanup idle connections
    async fn cleanup_loop(
        connections: Arc<Mutex<VecDeque<PooledConn>>>,
        idle_timeout_secs: u64,
    ) {
        let idle_timeout = Duration::from_secs(idle_timeout_secs);
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let mut conns = connections.lock().await;
            let original_len = conns.len();

            // Remove idle connections
            conns.retain(|conn| !conn.is_idle(idle_timeout));

            let removed = original_len - conns.len();
            if removed > 0 {
                tracing::info!("Cleaned up {} idle connections", removed);
            }
        }
    }

    /// Get the current number of connections in the pool
    pub async fn size(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// Get the maximum number of connections
    pub fn max_size(&self) -> usize {
        self.config.max_connections
    }
}

/// A connection from the pool that will be returned when dropped
pub struct PooledConnection {
    /// The connection (Option so we can take it on drop)
    conn: Option<PooledConn>,

    /// Reference to the pool to return the connection
    pool: Arc<Mutex<VecDeque<PooledConn>>>,

    /// Semaphore permit
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledConnection {
    /// Get a reference to the underlying client
    pub fn client(&self) -> &Client {
        &self.conn.as_ref().unwrap().client
    }

    /// Get a mutable reference to the underlying client
    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.conn.as_mut().unwrap().client
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(mut conn) = self.conn.take() {
            conn.touch();
            let pool = Arc::clone(&self.pool);

            // Return connection to pool in background
            tokio::spawn(async move {
                let mut connections = pool.lock().await;
                connections.push_back(conn);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires PostgreSQL to be running
    async fn test_pool_creation() {
        let config = Config::builder()
            .host("localhost")
            .database("test")
            .user("test")
            .max_connections(5)
            .build()
            .unwrap();

        let pool = Pool::new(config).await.unwrap();
        assert_eq!(pool.max_size(), 5);
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL to be running
    async fn test_get_connection() {
        let config = Config::builder()
            .host("localhost")
            .database("test")
            .user("test")
            .build()
            .unwrap();

        let pool = Pool::new(config).await.unwrap();
        let conn = pool.get().await.unwrap();
        assert!(conn.client().is_closed() == false);
    }
}
