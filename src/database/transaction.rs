//! Transaction management
//!
//! This module provides transaction support with automatic rollback on drop.

use crate::database::error::{Error, Result};
use crate::database::pool::PooledConnection;
use tokio_postgres::Transaction as PgTransaction;

/// Transaction wrapper that provides RAII semantics
pub struct Transaction<'a> {
    /// The underlying PostgreSQL transaction
    tx: Option<PgTransaction<'a>>,

    /// Whether the transaction has been committed
    committed: bool,
}

impl<'a> Transaction<'a> {
    /// Begin a new transaction from a pooled connection
    pub async fn begin(conn: &'a mut PooledConnection) -> Result<Transaction<'a>> {
        let tx = conn
            .client_mut()
            .transaction()
            .await
            .map_err(|e| Error::Transaction(format!("Failed to begin transaction: {}", e)))?;

        tracing::debug!("Transaction started");

        Ok(Transaction {
            tx: Some(tx),
            committed: false,
        })
    }

    /// Commit the transaction
    pub async fn commit(mut self) -> Result<()> {
        if let Some(tx) = self.tx.take() {
            tx.commit()
                .await
                .map_err(|e| Error::Transaction(format!("Failed to commit transaction: {}", e)))?;
            self.committed = true;
            tracing::debug!("Transaction committed");
            Ok(())
        } else {
            Err(Error::Transaction(
                "Transaction already completed".to_string(),
            ))
        }
    }

    /// Rollback the transaction
    pub async fn rollback(mut self) -> Result<()> {
        if let Some(tx) = self.tx.take() {
            tx.rollback()
                .await
                .map_err(|e| Error::Transaction(format!("Failed to rollback transaction: {}", e)))?;
            tracing::debug!("Transaction rolled back");
            Ok(())
        } else {
            Err(Error::Transaction(
                "Transaction already completed".to_string(),
            ))
        }
    }

    /// Get a reference to the underlying transaction
    pub fn client(&self) -> &tokio_postgres::Client {
        self.tx.as_ref().unwrap().client()
    }

    /// Execute a query
    pub async fn execute(&self, sql: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::Transaction("Transaction already completed".to_string()))?;

        tx.execute(sql, params)
            .await
            .map_err(|e| Error::Query(format!("Query failed: {}", e)))
    }

    /// Execute a query and return a single row
    pub async fn query_one(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<tokio_postgres::Row> {
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::Transaction("Transaction already completed".to_string()))?;

        tx.query_one(sql, params)
            .await
            .map_err(|e| Error::Query(format!("Query failed: {}", e)))
    }

    /// Execute a query and return all rows
    pub async fn query(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>> {
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::Transaction("Transaction already completed".to_string()))?;

        tx.query(sql, params)
            .await
            .map_err(|e| Error::Query(format!("Query failed: {}", e)))
    }

    /// Create a savepoint (nested transaction)
    pub async fn savepoint(&mut self, name: &str) -> Result<()> {
        let sql = format!("SAVEPOINT {}", name);
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::Transaction("Transaction already completed".to_string()))?;

        tx.execute(&sql, &[])
            .await
            .map_err(|e| Error::Transaction(format!("Failed to create savepoint: {}", e)))?;

        tracing::debug!("Savepoint '{}' created", name);
        Ok(())
    }

    /// Rollback to a savepoint
    pub async fn rollback_to(&mut self, name: &str) -> Result<()> {
        let sql = format!("ROLLBACK TO SAVEPOINT {}", name);
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::Transaction("Transaction already completed".to_string()))?;

        tx.execute(&sql, &[])
            .await
            .map_err(|e| Error::Transaction(format!("Failed to rollback to savepoint: {}", e)))?;

        tracing::debug!("Rolled back to savepoint '{}'", name);
        Ok(())
    }

    /// Release a savepoint
    pub async fn release_savepoint(&mut self, name: &str) -> Result<()> {
        let sql = format!("RELEASE SAVEPOINT {}", name);
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::Transaction("Transaction already completed".to_string()))?;

        tx.execute(&sql, &[])
            .await
            .map_err(|e| Error::Transaction(format!("Failed to release savepoint: {}", e)))?;

        tracing::debug!("Savepoint '{}' released", name);
        Ok(())
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        if !self.committed && self.tx.is_some() {
            // Transaction will be automatically rolled back when dropped
            tracing::warn!("Transaction dropped without explicit commit, will rollback");
        }
    }
}

/// Transaction isolation level
#[derive(Debug, Clone, Copy)]
pub enum IsolationLevel {
    /// Read uncommitted (PostgreSQL doesn't support this, will use READ COMMITTED)
    ReadUncommitted,

    /// Read committed (default)
    ReadCommitted,

    /// Repeatable read
    RepeatableRead,

    /// Serializable
    Serializable,
}

impl IsolationLevel {
    fn to_sql(&self) -> &str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Transaction builder for configuring transaction options
pub struct TransactionBuilder<'a> {
    conn: &'a mut PooledConnection,
    isolation_level: Option<IsolationLevel>,
    read_only: bool,
    deferrable: bool,
}

impl<'a> TransactionBuilder<'a> {
    /// Create a new transaction builder
    pub fn new(conn: &'a mut PooledConnection) -> Self {
        Self {
            conn,
            isolation_level: None,
            read_only: false,
            deferrable: false,
        }
    }

    /// Set the isolation level
    pub fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = Some(level);
        self
    }

    /// Set read-only mode
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Set deferrable mode (only for read-only, serializable transactions)
    pub fn deferrable(mut self) -> Self {
        self.deferrable = true;
        self
    }

    /// Begin the transaction with the configured options
    pub async fn begin(self) -> Result<Transaction<'a>> {
        // Build the BEGIN statement with options
        let mut sql = String::from("BEGIN");

        if let Some(level) = self.isolation_level {
            sql.push_str(" ISOLATION LEVEL ");
            sql.push_str(level.to_sql());
        }

        if self.read_only {
            sql.push_str(" READ ONLY");
        }

        if self.deferrable {
            sql.push_str(" DEFERRABLE");
        }

        // Execute the BEGIN statement
        self.conn
            .client()
            .execute(&sql, &[])
            .await
            .map_err(|e| Error::Transaction(format!("Failed to begin transaction: {}", e)))?;

        tracing::debug!("Transaction started with options: {}", sql);

        // Now get the transaction
        Transaction::begin(self.conn).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_sql() {
        assert_eq!(IsolationLevel::ReadCommitted.to_sql(), "READ COMMITTED");
        assert_eq!(
            IsolationLevel::RepeatableRead.to_sql(),
            "REPEATABLE READ"
        );
        assert_eq!(IsolationLevel::Serializable.to_sql(), "SERIALIZABLE");
    }
}
