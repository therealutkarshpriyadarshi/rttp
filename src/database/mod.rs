//! Database layer with connection pooling and ORM
//!
//! This module provides a complete database abstraction layer including:
//! - **Connection pooling**: Efficient management of database connections
//! - **Query builder**: Type-safe SQL query construction
//! - **ORM features**: Model trait for mapping structs to tables
//! - **Transaction management**: ACID transactions with automatic rollback
//!
//! # Example
//!
//! ```rust,ignore
//! use pttp::database::{Config, Pool, Model, QueryExecutor};
//!
//! // Configure and create a connection pool
//! let config = Config::builder()
//!     .host("localhost")
//!     .database("mydb")
//!     .user("postgres")
//!     .max_connections(10)
//!     .build()?;
//!
//! let pool = Pool::new(config).await?;
//!
//! // Get a connection and execute queries
//! let mut conn = pool.get().await?;
//! let mut executor = QueryExecutor::new(&mut conn);
//!
//! // Use the query builder
//! let users = executor.query::<User>(
//!     QueryBuilder::select_all()
//!         .from("users")
//!         .where("age", Operator::Gt, 18)
//! ).await?;
//! ```

mod config;
mod error;
mod model;
mod pool;
mod query;
mod transaction;
mod value;

// Re-export public API
pub use config::{Config, ConfigBuilder};
pub use error::{Error, Result};
pub use model::{FromRow, Model, ModelQuery, QueryExecutor};
pub use pool::{Pool, PooledConnection};
pub use query::{
    DeleteBuilder, InsertBuilder, JoinType, Operator, OrderDirection, QueryBuilder, UpdateBuilder,
};
pub use transaction::{IsolationLevel, Transaction, TransactionBuilder};
pub use value::Value;
