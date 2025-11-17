//! Database error types

use std::fmt;

/// Result type for database operations
pub type Result<T> = std::result::Result<T, Error>;

/// Database error type
#[derive(Debug)]
pub enum Error {
    /// Database connection error
    Connection(String),

    /// Query execution error
    Query(String),

    /// Connection pool error
    Pool(String),

    /// Transaction error
    Transaction(String),

    /// Row parsing error
    RowParse(String),

    /// Configuration error
    Config(String),

    /// Type conversion error
    TypeConversion(String),

    /// PostgreSQL error
    Postgres(tokio_postgres::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {}", msg),
            Error::Query(msg) => write!(f, "Query error: {}", msg),
            Error::Pool(msg) => write!(f, "Pool error: {}", msg),
            Error::Transaction(msg) => write!(f, "Transaction error: {}", msg),
            Error::RowParse(msg) => write!(f, "Row parse error: {}", msg),
            Error::Config(msg) => write!(f, "Configuration error: {}", msg),
            Error::TypeConversion(msg) => write!(f, "Type conversion error: {}", msg),
            Error::Postgres(err) => write!(f, "PostgreSQL error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Error::Postgres(err)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Query(msg)
    }
}
