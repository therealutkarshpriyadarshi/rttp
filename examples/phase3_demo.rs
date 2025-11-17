//! Phase 3 Demo: Database Layer
//!
//! This example demonstrates the database features implemented in Phase 3:
//! - Connection pooling with health checks
//! - Query builder for type-safe SQL construction
//! - ORM features with Model trait
//! - Transaction management with automatic rollback
//!
//! To run this example, you need a PostgreSQL database running.
//! Set the following environment variables:
//! - DB_HOST (default: localhost)
//! - DB_PORT (default: 5432)
//! - DB_NAME (required)
//! - DB_USER (required)
//! - DB_PASSWORD (optional)
//!
//! Example:
//! ```bash
//! export DB_NAME=testdb
//! export DB_USER=postgres
//! cargo run --example phase3_demo
//! ```

use pttp::database::{
    Config, DeleteBuilder, FromRow, InsertBuilder, Model, Operator, OrderDirection, Pool,
    QueryBuilder, QueryExecutor, Transaction, UpdateBuilder, Value,
};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

/// Example User model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    age: i32,
}

impl Model for User {
    fn table_name() -> &'static str {
        "users"
    }

    fn from_row(row: &Row) -> pttp::database::Result<Self> {
        Ok(User {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            email: row.try_get("email")?,
            age: row.try_get("age")?,
        })
    }

    fn to_values(&self) -> Vec<(&str, Value)> {
        vec![
            ("name", self.name.clone().into()),
            ("email", self.email.clone().into()),
            ("age", self.age.into()),
        ]
    }
}

/// Example Post model
#[derive(Debug, Clone)]
struct Post {
    id: i32,
    user_id: i32,
    title: String,
    content: String,
}

impl Model for Post {
    fn table_name() -> &'static str {
        "posts"
    }

    fn from_row(row: &Row) -> pttp::database::Result<Self> {
        Ok(Post {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            title: row.try_get("title")?,
            content: row.try_get("content")?,
        })
    }

    fn to_values(&self) -> Vec<(&str, Value)> {
        vec![
            ("user_id", self.user_id.into()),
            ("title", self.title.clone().into()),
            ("content", self.content.clone().into()),
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("phase3_demo=debug,pttp=debug")
        .init();

    println!("🚀 Phase 3 Demo: Database Layer\n");

    // 1. Configuration and Connection Pool
    println!("📋 Step 1: Creating connection pool...");
    let config = Config::from_env().unwrap_or_else(|_| {
        println!("⚠️  Using default configuration. Set DB_* environment variables for custom config.");
        Config::builder()
            .host("localhost")
            .database("postgres")
            .user("postgres")
            .max_connections(5)
            .build()
            .unwrap()
    });

    println!("   Host: {}", config.host);
    println!("   Database: {}", config.database);
    println!("   Max connections: {}", config.max_connections);

    let pool = Pool::new(config).await?;
    println!("✅ Connection pool created\n");

    // 2. Setup: Create tables
    println!("📋 Step 2: Setting up database tables...");
    setup_tables(&pool).await?;
    println!("✅ Tables created\n");

    // 3. Query Builder Demo
    println!("📋 Step 3: Query Builder Demo");
    demo_query_builder().await?;
    println!("✅ Query builder demo complete\n");

    // 4. Insert Operations
    println!("📋 Step 4: Insert Operations");
    let user_id = demo_insert(&pool).await?;
    println!("✅ Insert operations complete\n");

    // 5. Select Operations
    println!("📋 Step 5: Select Operations");
    demo_select(&pool, user_id).await?;
    println!("✅ Select operations complete\n");

    // 6. Update Operations
    println!("📋 Step 6: Update Operations");
    demo_update(&pool, user_id).await?;
    println!("✅ Update operations complete\n");

    // 7. Transaction Demo
    println!("📋 Step 7: Transaction Management");
    demo_transactions(&pool).await?;
    println!("✅ Transaction demo complete\n");

    // 8. Delete Operations
    println!("📋 Step 8: Delete Operations");
    demo_delete(&pool, user_id).await?;
    println!("✅ Delete operations complete\n");

    // 9. Pool Statistics
    println!("📋 Step 9: Pool Statistics");
    println!("   Current pool size: {}", pool.size().await);
    println!("   Maximum pool size: {}", pool.max_size());
    println!("✅ Pool statistics retrieved\n");

    println!("🎉 Phase 3 Demo Complete!");
    println!("\n📊 Summary:");
    println!("   ✓ Connection pooling with health checks");
    println!("   ✓ Type-safe query builder");
    println!("   ✓ ORM with Model trait");
    println!("   ✓ Transaction management");
    println!("   ✓ CRUD operations");

    Ok(())
}

async fn setup_tables(pool: &Pool) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get().await?;

    // Drop tables if they exist
    conn.client()
        .execute("DROP TABLE IF EXISTS posts CASCADE", &[])
        .await?;
    conn.client()
        .execute("DROP TABLE IF EXISTS users CASCADE", &[])
        .await?;

    // Create users table
    conn.client()
        .execute(
            "CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL UNIQUE,
                age INTEGER NOT NULL
            )",
            &[],
        )
        .await?;

    // Create posts table
    conn.client()
        .execute(
            "CREATE TABLE posts (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL REFERENCES users(id),
                title VARCHAR(255) NOT NULL,
                content TEXT NOT NULL
            )",
            &[],
        )
        .await?;

    println!("   ✓ Created users table");
    println!("   ✓ Created posts table");

    Ok(())
}

async fn demo_query_builder() -> Result<(), Box<dyn std::error::Error>> {
    // Build a complex SELECT query
    let (sql, _params) = QueryBuilder::select(&["id", "name", "email"])
        .from("users")
        .r#where("age", Operator::Gt, 18)
        .r#where("email", Operator::Like, "%@example.com")
        .order_by("name", OrderDirection::Asc)
        .limit(10)
        .offset(0)
        .build();

    println!("   SELECT query:");
    println!("   {}", sql);

    // Build an INSERT query
    let (sql, _params) = InsertBuilder::new("users")
        .set("name", "John Doe")
        .set("email", "john@example.com")
        .set("age", 25)
        .build();

    println!("\n   INSERT query:");
    println!("   {}", sql);

    // Build an UPDATE query
    let (sql, _params) = UpdateBuilder::new("users")
        .set("name", "Jane Doe")
        .set("age", 26)
        .r#where("id", Operator::Eq, 1)
        .build();

    println!("\n   UPDATE query:");
    println!("   {}", sql);

    // Build a DELETE query
    let (sql, _params) = DeleteBuilder::new("users")
        .r#where("id", Operator::Eq, 1)
        .build();

    println!("\n   DELETE query:");
    println!("   {}", sql);

    Ok(())
}

async fn demo_insert(pool: &Pool) -> Result<i32, Box<dyn std::error::Error>> {
    let mut conn = pool.get().await?;
    let mut executor = QueryExecutor::new(&mut conn);

    // Insert a user and get the ID back
    let insert = InsertBuilder::new("users")
        .set("name", "Alice Smith")
        .set("email", "alice@example.com")
        .set("age", 28);

    let user: User = executor
        .insert_returning(insert, &["id", "name", "email", "age"])
        .await?;

    println!("   ✓ Inserted user: {} (ID: {})", user.name, user.id);

    // Insert more users
    let insert = InsertBuilder::new("users")
        .set("name", "Bob Johnson")
        .set("email", "bob@example.com")
        .set("age", 32);

    executor.insert(insert).await?;
    println!("   ✓ Inserted user: Bob Johnson");

    let insert = InsertBuilder::new("users")
        .set("name", "Charlie Brown")
        .set("email", "charlie@example.com")
        .set("age", 22);

    executor.insert(insert).await?;
    println!("   ✓ Inserted user: Charlie Brown");

    Ok(user.id)
}

async fn demo_select(pool: &Pool, user_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get().await?;
    let mut executor = QueryExecutor::new(&mut conn);

    // Find by ID
    let user: Option<User> = executor.find(user_id).await?;
    if let Some(user) = user {
        println!("   ✓ Found user by ID: {} ({})", user.name, user.email);
    }

    // Find all users
    let users: Vec<User> = executor.find_all().await?;
    println!("   ✓ Found {} users total", users.len());

    // Query with conditions
    let query = QueryBuilder::select_all()
        .from("users")
        .r#where("age", Operator::Gt, 25)
        .order_by("age", OrderDirection::Desc);

    let older_users: Vec<User> = executor.query(query).await?;
    println!("   ✓ Found {} users over 25 years old", older_users.len());
    for user in older_users {
        println!("      - {} (age: {})", user.name, user.age);
    }

    Ok(())
}

async fn demo_update(pool: &Pool, user_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get().await?;
    let mut executor = QueryExecutor::new(&mut conn);

    // Update a user
    let update = UpdateBuilder::new("users")
        .set("age", 29)
        .r#where("id", Operator::Eq, user_id);

    let rows_affected = executor.update(update).await?;
    println!("   ✓ Updated {} row(s)", rows_affected);

    // Verify the update
    let user: Option<User> = executor.find(user_id).await?;
    if let Some(user) = user {
        println!("   ✓ Verified: {} is now {} years old", user.name, user.age);
    }

    Ok(())
}

async fn demo_transactions(pool: &Pool) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get().await?;

    println!("   Testing transaction commit...");
    {
        let tx = Transaction::begin(&mut conn).await?;

        tx.execute(
            "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)",
            &[&"Transaction User", &"tx@example.com", &35],
        )
        .await?;

        tx.commit().await?;
        println!("   ✓ Transaction committed successfully");
    }

    println!("\n   Testing transaction rollback...");
    {
        let tx = Transaction::begin(&mut conn).await?;

        tx.execute(
            "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)",
            &[&"Rollback User", &"rollback@example.com", &40],
        )
        .await?;

        tx.rollback().await?;
        println!("   ✓ Transaction rolled back successfully");
    }

    println!("\n   Testing automatic rollback on drop...");
    {
        let tx = Transaction::begin(&mut conn).await?;

        tx.execute(
            "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)",
            &[&"Drop User", &"drop@example.com", &45],
        )
        .await?;

        // tx is dropped here without commit, should rollback automatically
        println!("   ✓ Transaction will rollback on drop");
    }

    Ok(())
}

async fn demo_delete(pool: &Pool, user_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get().await?;
    let mut executor = QueryExecutor::new(&mut conn);

    // Delete by ID
    let rows_affected = executor.delete_by_id::<User>(user_id).await?;
    println!("   ✓ Deleted {} row(s) by ID", rows_affected);

    // Delete with conditions
    let delete = DeleteBuilder::new("users").r#where("age", Operator::Lt, 25);

    let rows_affected = executor.delete(delete).await?;
    println!("   ✓ Deleted {} row(s) with conditions", rows_affected);

    Ok(())
}
