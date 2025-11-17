//! Background processing - Task queue and scheduler
//!
//! This module provides comprehensive background job processing capabilities:
//!
//! ## Task Queue
//!
//! A robust task queue system with the following features:
//! - **Priority-based execution**: Tasks can have Low, Normal, High, or Critical priority
//! - **Retry logic**: Automatic retry with exponential backoff
//! - **Worker pool**: Concurrent task processing with configurable workers
//! - **Timeout support**: Prevent tasks from running indefinitely
//! - **Delayed execution**: Schedule tasks to run after a delay
//!
//! ## Scheduler
//!
//! A flexible scheduler supporting:
//! - **One-time tasks**: Execute tasks at a specific datetime
//! - **Recurring tasks**: Use cron expressions for periodic execution
//! - **Task management**: Enable, disable, and remove scheduled tasks
//!
//! # Examples
//!
//! ## Using Task Queue
//!
//! ```
//! use pttp::background::{TaskQueue, Task, TaskPayload, Priority, WorkerPool, WorkerConfig};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let queue = Arc::new(TaskQueue::new());
//!
//!     // Register a task handler
//!     queue.register("send_email", |payload| async move {
//!         println!("Sending email...");
//!         Ok(())
//!     }).await;
//!
//!     // Enqueue a task
//!     let payload = TaskPayload::new(serde_json::json!({
//!         "to": "user@example.com",
//!         "subject": "Hello"
//!     }))?;
//!
//!     let task = Task::new("send_email", payload)
//!         .with_priority(Priority::High)
//!         .with_max_retries(3);
//!
//!     queue.enqueue(task).await?;
//!
//!     // Start worker pool
//!     let config = WorkerConfig::new().with_workers(4);
//!     let mut pool = WorkerPool::new(queue.clone(), config);
//!     pool.start().await;
//!
//!     // Process tasks...
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using Scheduler
//!
//! ```
//! use pttp::background::Scheduler;
//! use chrono::Local;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let scheduler = Scheduler::new();
//!
//!     // Schedule a one-time task
//!     let future_time = Local::now() + chrono::Duration::hours(1);
//!     scheduler.schedule_once("cleanup", future_time, || async {
//!         println!("Running cleanup...");
//!     }).await;
//!
//!     // Schedule a recurring task (every day at midnight)
//!     scheduler.schedule_cron("daily_report", "0 0 * * *", || async {
//!         println!("Generating daily report...");
//!     }).await?;
//!
//!     // Start the scheduler
//!     scheduler.start().await?;
//!
//!     // Keep running...
//!
//!     Ok(())
//! }
//! ```

mod cron;
mod queue;
mod scheduler;
mod worker;

pub use cron::{CronError, CronExpr};
pub use queue::{Priority, Task, TaskError, TaskPayload, TaskQueue, TaskStatus, QueueStats};
pub use scheduler::{Scheduler, SchedulerError, ScheduledTaskInfo};
pub use worker::{WorkerConfig, WorkerPool};
