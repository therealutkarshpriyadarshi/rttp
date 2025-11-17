//! Database configuration

use crate::database::error::{Error, Result};

/// Database configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Host address
    pub host: String,

    /// Port number
    pub port: u16,

    /// Database name
    pub database: String,

    /// Username
    pub user: String,

    /// Password
    pub password: String,

    /// Maximum number of connections in the pool
    pub max_connections: usize,

    /// Connection timeout in seconds
    pub connect_timeout: u64,

    /// Idle timeout in seconds (time before connection is closed)
    pub idle_timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "postgres".to_string(),
            user: "postgres".to_string(),
            password: String::new(),
            max_connections: 10,
            connect_timeout: 30,
            idle_timeout: 600,
        }
    }
}

impl Config {
    /// Create a new configuration builder
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Build a connection string for PostgreSQL
    pub fn connection_string(&self) -> String {
        format!(
            "host={} port={} dbname={} user={} password={} connect_timeout={}",
            self.host, self.port, self.database, self.user, self.password, self.connect_timeout
        )
    }

    /// Parse from environment variables
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("DB_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5432);
        let database = std::env::var("DB_NAME").unwrap_or_else(|_| "postgres".to_string());
        let user = std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string());
        let password = std::env::var("DB_PASSWORD").unwrap_or_default();

        Ok(Self {
            host,
            port,
            database,
            user,
            password,
            ..Default::default()
        })
    }
}

/// Configuration builder
#[derive(Default)]
pub struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    user: Option<String>,
    password: Option<String>,
    max_connections: Option<usize>,
    connect_timeout: Option<u64>,
    idle_timeout: Option<u64>,
}

impl ConfigBuilder {
    /// Set host address
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set port number
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set database name
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set username
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set password
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set maximum number of connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = Some(max);
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: u64) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set idle timeout
    pub fn idle_timeout(mut self, timeout: u64) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<Config> {
        let database = self
            .database
            .ok_or_else(|| Error::Config("Database name is required".to_string()))?;

        let user = self
            .user
            .ok_or_else(|| Error::Config("Username is required".to_string()))?;

        Ok(Config {
            host: self.host.unwrap_or_else(|| "localhost".to_string()),
            port: self.port.unwrap_or(5432),
            database,
            user,
            password: self.password.unwrap_or_default(),
            max_connections: self.max_connections.unwrap_or(10),
            connect_timeout: self.connect_timeout.unwrap_or(30),
            idle_timeout: self.idle_timeout.unwrap_or(600),
        })
    }
}
