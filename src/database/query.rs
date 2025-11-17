//! Query builder for constructing SQL queries safely
//!
//! This module provides a type-safe query builder that prevents SQL injection
//! by using parameterized queries.

use crate::database::value::Value;

/// WHERE clause operator
#[derive(Debug, Clone)]
pub enum Operator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Like,
    In,
    IsNull,
    IsNotNull,
}

impl Operator {
    fn to_sql(&self) -> &str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "!=",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
            Operator::Like => "LIKE",
            Operator::In => "IN",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
        }
    }
}

/// WHERE clause condition
#[derive(Debug, Clone)]
pub struct WhereClause {
    column: String,
    operator: Operator,
    value: Option<Value>,
}

/// JOIN type
#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl JoinType {
    fn to_sql(&self) -> &str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL OUTER JOIN",
        }
    }
}

/// JOIN clause
#[derive(Debug, Clone)]
pub struct Join {
    join_type: JoinType,
    table: String,
    on: String,
}

/// ORDER BY direction
#[derive(Debug, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    fn to_sql(&self) -> &str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

/// ORDER BY clause
#[derive(Debug, Clone)]
pub struct OrderBy {
    column: String,
    direction: OrderDirection,
}

/// Query builder for SELECT statements
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: Option<String>,
    columns: Vec<String>,
    wheres: Vec<WhereClause>,
    joins: Vec<Join>,
    order_by: Vec<OrderBy>,
    limit: Option<i64>,
    offset: Option<i64>,
    params: Vec<Value>,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            table: None,
            columns: Vec::new(),
            wheres: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            params: Vec::new(),
        }
    }

    /// Select columns
    pub fn select(columns: &[&str]) -> Self {
        let mut builder = Self::new();
        builder.columns = columns.iter().map(|c| c.to_string()).collect();
        builder
    }

    /// Select all columns
    pub fn select_all() -> Self {
        let mut builder = Self::new();
        builder.columns = vec!["*".to_string()];
        builder
    }

    /// Specify the table
    pub fn from(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, column: &str, operator: Operator, value: impl Into<Value>) -> Self {
        let value = value.into();
        self.params.push(value.clone());
        self.wheres.push(WhereClause {
            column: column.to_string(),
            operator,
            value: Some(value),
        });
        self
    }

    /// Add a WHERE IS NULL clause
    pub fn where_null(mut self, column: &str) -> Self {
        self.wheres.push(WhereClause {
            column: column.to_string(),
            operator: Operator::IsNull,
            value: None,
        });
        self
    }

    /// Add a WHERE IS NOT NULL clause
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.wheres.push(WhereClause {
            column: column.to_string(),
            operator: Operator::IsNotNull,
            value: None,
        });
        self
    }

    /// Add a JOIN clause
    pub fn join(mut self, join_type: JoinType, table: &str, on: &str) -> Self {
        self.joins.push(Join {
            join_type,
            table: table.to_string(),
            on: on.to_string(),
        });
        self
    }

    /// Add an INNER JOIN clause
    pub fn inner_join(self, table: &str, on: &str) -> Self {
        self.join(JoinType::Inner, table, on)
    }

    /// Add a LEFT JOIN clause
    pub fn left_join(self, table: &str, on: &str) -> Self {
        self.join(JoinType::Left, table, on)
    }

    /// Add an ORDER BY clause
    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            direction,
        });
        self
    }

    /// Add a LIMIT clause
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add an OFFSET clause
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Build the SQL query and parameters
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = String::from("SELECT ");

        // Columns
        if self.columns.is_empty() {
            sql.push_str("*");
        } else {
            sql.push_str(&self.columns.join(", "));
        }

        // FROM
        if let Some(ref table) = self.table {
            sql.push_str(" FROM ");
            sql.push_str(table);
        }

        // JOINs
        for join in &self.joins {
            sql.push(' ');
            sql.push_str(join.join_type.to_sql());
            sql.push(' ');
            sql.push_str(&join.table);
            sql.push_str(" ON ");
            sql.push_str(&join.on);
        }

        // WHERE
        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            let mut param_index = 1;
            for (i, where_clause) in self.wheres.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&where_clause.column);
                sql.push(' ');
                sql.push_str(where_clause.operator.to_sql());

                match where_clause.operator {
                    Operator::IsNull | Operator::IsNotNull => {
                        // No parameter needed
                    }
                    _ => {
                        sql.push_str(&format!(" ${}", param_index));
                        param_index += 1;
                    }
                }
            }
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            for (i, order) in self.order_by.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(&order.column);
                sql.push(' ');
                sql.push_str(order.direction.to_sql());
            }
        }

        // LIMIT
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // OFFSET
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, self.params.clone())
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Insert query builder
#[derive(Debug)]
pub struct InsertBuilder {
    table: String,
    columns: Vec<String>,
    values: Vec<Value>,
}

impl InsertBuilder {
    /// Create a new insert builder
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Add a column and value
    pub fn set(mut self, column: &str, value: impl Into<Value>) -> Self {
        self.columns.push(column.to_string());
        self.values.push(value.into());
        self
    }

    /// Build the SQL query and parameters
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("INSERT INTO {} (", self.table);
        sql.push_str(&self.columns.join(", "));
        sql.push_str(") VALUES (");

        let placeholders: Vec<String> = (1..=self.columns.len())
            .map(|i| format!("${}", i))
            .collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        (sql, self.values.clone())
    }

    /// Build with RETURNING clause
    pub fn build_returning(&self, columns: &[&str]) -> (String, Vec<Value>) {
        let (mut sql, params) = self.build();
        sql.push_str(" RETURNING ");
        sql.push_str(&columns.join(", "));
        (sql, params)
    }
}

/// Update query builder
#[derive(Debug)]
pub struct UpdateBuilder {
    table: String,
    sets: Vec<(String, Value)>,
    wheres: Vec<WhereClause>,
}

impl UpdateBuilder {
    /// Create a new update builder
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            sets: Vec::new(),
            wheres: Vec::new(),
        }
    }

    /// Set a column value
    pub fn set(mut self, column: &str, value: impl Into<Value>) -> Self {
        self.sets.push((column.to_string(), value.into()));
        self
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, column: &str, operator: Operator, value: impl Into<Value>) -> Self {
        self.wheres.push(WhereClause {
            column: column.to_string(),
            operator,
            value: Some(value.into()),
        });
        self
    }

    /// Build the SQL query and parameters
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("UPDATE {} SET ", self.table);
        let mut params = Vec::new();
        let mut param_index = 1;

        // SET clause
        for (i, (column, value)) in self.sets.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(column);
            sql.push_str(&format!(" = ${}", param_index));
            params.push(value.clone());
            param_index += 1;
        }

        // WHERE clause
        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            for (i, where_clause) in self.wheres.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&where_clause.column);
                sql.push(' ');
                sql.push_str(where_clause.operator.to_sql());
                sql.push_str(&format!(" ${}", param_index));
                if let Some(ref value) = where_clause.value {
                    params.push(value.clone());
                }
                param_index += 1;
            }
        }

        (sql, params)
    }
}

/// Delete query builder
#[derive(Debug)]
pub struct DeleteBuilder {
    table: String,
    wheres: Vec<WhereClause>,
}

impl DeleteBuilder {
    /// Create a new delete builder
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            wheres: Vec::new(),
        }
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, column: &str, operator: Operator, value: impl Into<Value>) -> Self {
        self.wheres.push(WhereClause {
            column: column.to_string(),
            operator,
            value: Some(value.into()),
        });
        self
    }

    /// Build the SQL query and parameters
    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("DELETE FROM {}", self.table);
        let mut params = Vec::new();
        let mut param_index = 1;

        // WHERE clause
        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            for (i, where_clause) in self.wheres.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&where_clause.column);
                sql.push(' ');
                sql.push_str(where_clause.operator.to_sql());
                sql.push_str(&format!(" ${}", param_index));
                if let Some(ref value) = where_clause.value {
                    params.push(value.clone());
                }
                param_index += 1;
            }
        }

        (sql, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_query() {
        let (sql, params) = QueryBuilder::select(&["id", "name"])
            .from("users")
            .r#where("age", Operator::Gt, 18)
            .order_by("name", OrderDirection::Asc)
            .limit(10)
            .build();

        assert_eq!(sql, "SELECT id, name FROM users WHERE age > $1 ORDER BY name ASC LIMIT 10");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_insert_query() {
        let (sql, params) = InsertBuilder::new("users")
            .set("name", "John")
            .set("age", 25)
            .build();

        assert_eq!(sql, "INSERT INTO users (name, age) VALUES ($1, $2)");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_update_query() {
        let (sql, params) = UpdateBuilder::new("users")
            .set("name", "Jane")
            .r#where("id", Operator::Eq, 1)
            .build();

        assert_eq!(sql, "UPDATE users SET name = $1 WHERE id = $2");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_delete_query() {
        let (sql, params) = DeleteBuilder::new("users")
            .r#where("id", Operator::Eq, 1)
            .build();

        assert_eq!(sql, "DELETE FROM users WHERE id = $1");
        assert_eq!(params.len(), 1);
    }
}
