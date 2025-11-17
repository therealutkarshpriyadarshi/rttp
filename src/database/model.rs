//! ORM model trait and row mapping
//!
//! This module provides the foundation for ORM functionality, allowing
//! structs to be mapped to database tables and rows.

use crate::database::error::{Error, Result};
use crate::database::pool::PooledConnection;
use crate::database::query::{DeleteBuilder, InsertBuilder, Operator, QueryBuilder, UpdateBuilder};
use crate::database::value::Value;
use tokio_postgres::Row;

/// Trait for models that can be persisted to the database
pub trait Model: Sized {
    /// Get the table name for this model
    fn table_name() -> &'static str;

    /// Convert a database row to a model instance
    fn from_row(row: &Row) -> Result<Self>;

    /// Convert the model to a list of column-value pairs
    fn to_values(&self) -> Vec<(&str, Value)>;

    /// Get the primary key column name (defaults to "id")
    fn primary_key() -> &'static str {
        "id"
    }
}

/// Helper trait for extracting typed values from rows
pub trait FromRow {
    /// Try to extract a value from a row by column name
    fn try_get<T>(&self, column: &str) -> Result<T>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>;
}

impl FromRow for Row {
    fn try_get<T>(&self, column: &str) -> Result<T>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>,
    {
        self.try_get(column)
            .map_err(|e| Error::RowParse(format!("Failed to get column '{}': {}", column, e)))
    }
}

/// Query executor for models
pub struct QueryExecutor<'a> {
    conn: &'a mut PooledConnection,
}

impl<'a> QueryExecutor<'a> {
    /// Create a new query executor
    pub fn new(conn: &'a mut PooledConnection) -> Self {
        Self { conn }
    }

    /// Execute a SELECT query and return all results
    pub async fn query<M: Model>(&mut self, builder: QueryBuilder) -> Result<Vec<M>> {
        let (sql, params) = builder.build();
        let params_refs = Self::params_to_refs(&params);

        tracing::debug!("Executing query: {} with {} params", sql, params_refs.len());

        let rows = self
            .conn
            .client()
            .query(&sql, &params_refs[..])
            .await
            .map_err(|e| Error::Query(format!("Query failed: {}", e)))?;

        rows.iter().map(|row| M::from_row(row)).collect()
    }

    /// Execute a SELECT query and return the first result
    pub async fn query_one<M: Model>(&mut self, builder: QueryBuilder) -> Result<Option<M>> {
        let (sql, params) = builder.build();
        let params_refs = Self::params_to_refs(&params);

        tracing::debug!("Executing query: {} with {} params", sql, params_refs.len());

        let row = self
            .conn
            .client()
            .query_opt(&sql, &params_refs[..])
            .await
            .map_err(|e| Error::Query(format!("Query failed: {}", e)))?;

        row.map(|r| M::from_row(&r)).transpose()
    }

    /// Execute an INSERT query
    pub async fn insert(&mut self, builder: InsertBuilder) -> Result<u64> {
        let (sql, params) = builder.build();
        let params_refs = Self::params_to_refs(&params);

        tracing::debug!("Executing insert: {} with {} params", sql, params_refs.len());

        let rows_affected = self
            .conn
            .client()
            .execute(&sql, &params_refs[..])
            .await
            .map_err(|e| Error::Query(format!("Insert failed: {}", e)))?;

        Ok(rows_affected)
    }

    /// Execute an INSERT query and return the inserted row
    pub async fn insert_returning<M: Model>(
        &mut self,
        builder: InsertBuilder,
        columns: &[&str],
    ) -> Result<M> {
        let (sql, params) = builder.build_returning(columns);
        let params_refs = Self::params_to_refs(&params);

        tracing::debug!("Executing insert: {} with {} params", sql, params_refs.len());

        let row = self
            .conn
            .client()
            .query_one(&sql, &params_refs[..])
            .await
            .map_err(|e| Error::Query(format!("Insert failed: {}", e)))?;

        M::from_row(&row)
    }

    /// Execute an UPDATE query
    pub async fn update(&mut self, builder: UpdateBuilder) -> Result<u64> {
        let (sql, params) = builder.build();
        let params_refs = Self::params_to_refs(&params);

        tracing::debug!("Executing update: {} with {} params", sql, params_refs.len());

        let rows_affected = self
            .conn
            .client()
            .execute(&sql, &params_refs[..])
            .await
            .map_err(|e| Error::Query(format!("Update failed: {}", e)))?;

        Ok(rows_affected)
    }

    /// Execute a DELETE query
    pub async fn delete(&mut self, builder: DeleteBuilder) -> Result<u64> {
        let (sql, params) = builder.build();
        let params_refs = Self::params_to_refs(&params);

        tracing::debug!("Executing delete: {} with {} params", sql, params_refs.len());

        let rows_affected = self
            .conn
            .client()
            .execute(&sql, &params_refs[..])
            .await
            .map_err(|e| Error::Query(format!("Delete failed: {}", e)))?;

        Ok(rows_affected)
    }

    /// Find a model by primary key
    pub async fn find<M: Model>(&mut self, id: impl Into<Value>) -> Result<Option<M>> {
        let query = QueryBuilder::select_all()
            .from(M::table_name())
            .r#where(M::primary_key(), Operator::Eq, id)
            .limit(1);

        self.query_one(query).await
    }

    /// Find all models
    pub async fn find_all<M: Model>(&mut self) -> Result<Vec<M>> {
        let query = QueryBuilder::select_all().from(M::table_name());

        self.query(query).await
    }

    /// Save a model (insert or update)
    pub async fn save<M: Model>(&mut self, model: &M) -> Result<u64> {
        let values = model.to_values();
        let mut insert = InsertBuilder::new(M::table_name());

        for (column, value) in values {
            insert = insert.set(column, value);
        }

        self.insert(insert).await
    }

    /// Delete a model by primary key
    pub async fn delete_by_id<M: Model>(&mut self, id: impl Into<Value>) -> Result<u64> {
        let delete = DeleteBuilder::new(M::table_name()).r#where(M::primary_key(), Operator::Eq, id);

        self.delete(delete).await
    }

    /// Convert Value types to references that tokio-postgres can use
    fn params_to_refs(params: &[Value]) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)> {
        params
            .iter()
            .map(|v| match v {
                Value::Null => {
                    // For NULL values, we use Option::<i32>::None as a workaround
                    // This is a limitation of the type system, but works in practice
                    static NULL: Option<i32> = None;
                    &NULL as &(dyn tokio_postgres::types::ToSql + Sync)
                }
                Value::Bool(b) => b as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::I16(i) => i as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::I32(i) => i as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::I64(i) => i as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::F32(f) => f as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::F64(f) => f as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::String(s) => s as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::Bytes(b) => b as &(dyn tokio_postgres::types::ToSql + Sync),
                Value::Json(j) => j as &(dyn tokio_postgres::types::ToSql + Sync),
            })
            .collect()
    }
}

/// Helper struct for building model queries
pub struct ModelQuery<M> {
    builder: QueryBuilder,
    _phantom: std::marker::PhantomData<M>,
}

impl<M: Model> ModelQuery<M> {
    /// Create a new model query
    pub fn new() -> Self {
        Self {
            builder: QueryBuilder::select_all().from(M::table_name()),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, column: &str, operator: Operator, value: impl Into<Value>) -> Self {
        self.builder = self.builder.r#where(column, operator, value);
        self
    }

    /// Add a LIMIT clause
    pub fn limit(mut self, limit: i64) -> Self {
        self.builder = self.builder.limit(limit);
        self
    }

    /// Add an OFFSET clause
    pub fn offset(mut self, offset: i64) -> Self {
        self.builder = self.builder.offset(offset);
        self
    }

    /// Execute the query
    pub async fn execute(self, executor: &mut QueryExecutor<'_>) -> Result<Vec<M>> {
        executor.query(self.builder).await
    }

    /// Execute the query and return the first result
    pub async fn first(self, executor: &mut QueryExecutor<'_>) -> Result<Option<M>> {
        executor.query_one(self.builder.limit(1)).await
    }
}

impl<M: Model> Default for ModelQuery<M> {
    fn default() -> Self {
        Self::new()
    }
}
