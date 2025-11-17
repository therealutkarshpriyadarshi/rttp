//! Task queue implementation
//!
//! This module provides a robust task queue with the following features:
//! - Job priority support
//! - Retry logic with exponential backoff
//! - Worker pool for concurrent processing
//! - In-memory job persistence with optional DB backing

use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{sleep, Instant};

/// Type alias for async task handlers
pub type TaskHandler = Arc<
    dyn Fn(TaskPayload) -> Pin<Box<dyn Future<Output = Result<(), TaskError>> + Send>>
        + Send
        + Sync,
>;

/// Task error type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskError {
    /// Task execution failed
    ExecutionFailed(String),
    /// Task timed out
    Timeout,
    /// Task was cancelled
    Cancelled,
    /// Maximum retries exceeded
    MaxRetriesExceeded,
    /// Handler not found
    HandlerNotFound(String),
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::ExecutionFailed(msg) => write!(f, "Task execution failed: {}", msg),
            TaskError::Timeout => write!(f, "Task timed out"),
            TaskError::Cancelled => write!(f, "Task was cancelled"),
            TaskError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
            TaskError::HandlerNotFound(name) => write!(f, "Handler not found: {}", name),
        }
    }
}

impl std::error::Error for TaskError {}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority (processed last)
    Low = 1,
    /// Normal priority (default)
    Normal = 2,
    /// High priority (processed first)
    High = 3,
    /// Critical priority (processed immediately)
    Critical = 4,
}

/// Task payload - serializable data passed to task handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    /// Task data as JSON value
    pub data: serde_json::Value,
}

impl TaskPayload {
    /// Create a new task payload
    pub fn new<T: Serialize>(data: T) -> Result<Self, serde_json::Error> {
        Ok(Self {
            data: serde_json::to_value(data)?,
        })
    }

    /// Extract typed data from payload
    pub fn extract<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
}

/// Task status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// A task in the queue
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task ID
    pub id: String,
    /// Task name/handler identifier
    pub name: String,
    /// Task payload
    pub payload: TaskPayload,
    /// Task priority
    pub priority: Priority,
    /// Current retry count
    pub retry_count: u32,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Task status
    pub status: TaskStatus,
    /// When the task was created
    pub created_at: Instant,
    /// When the task should be executed (for delayed tasks)
    pub execute_at: Instant,
    /// Task timeout
    pub timeout: Option<Duration>,
}

impl Task {
    /// Create a new task
    pub fn new(name: impl Into<String>, payload: TaskPayload) -> Self {
        let now = Instant::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            payload,
            priority: Priority::Normal,
            retry_count: 0,
            max_retries: 3,
            status: TaskStatus::Pending,
            created_at: now,
            execute_at: now,
            timeout: Some(Duration::from_secs(300)), // 5 minutes default
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set task timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Delay task execution
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.execute_at = Instant::now() + delay;
        self
    }

    /// Check if task should be executed now
    pub fn should_execute(&self) -> bool {
        Instant::now() >= self.execute_at
    }

    /// Calculate backoff duration for retry
    pub fn backoff_duration(&self) -> Duration {
        // Exponential backoff: 2^retry_count seconds, max 300 seconds
        let seconds = 2u64.pow(self.retry_count).min(300);
        Duration::from_secs(seconds)
    }
}

// Implement ordering for priority queue
impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier execute_at
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.execute_at.cmp(&self.execute_at),
            ord => ord,
        }
    }
}

/// Task queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Total tasks enqueued
    pub total_enqueued: u64,
    /// Total tasks completed
    pub total_completed: u64,
    /// Total tasks failed
    pub total_failed: u64,
    /// Current pending tasks
    pub pending: usize,
    /// Current running tasks
    pub running: usize,
}

/// Task queue for background job processing
pub struct TaskQueue {
    /// Priority queue of pending tasks
    pending: Arc<Mutex<BinaryHeap<Task>>>,
    /// Currently running tasks
    running: Arc<RwLock<Vec<Task>>>,
    /// Task handlers registry
    handlers: Arc<RwLock<std::collections::HashMap<String, TaskHandler>>>,
    /// Channel for new tasks
    tx: mpsc::UnboundedSender<Task>,
    /// Statistics
    stats: Arc<RwLock<QueueStats>>,
    /// Shutdown signal
    shutdown: Arc<Mutex<bool>>,
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self {
            pending: Arc::new(Mutex::new(BinaryHeap::new())),
            running: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            tx,
            stats: Arc::new(RwLock::new(QueueStats::default())),
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    /// Register a task handler
    pub async fn register<F, Fut>(&self, name: impl Into<String>, handler: F)
    where
        F: Fn(TaskPayload) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), TaskError>> + Send + 'static,
    {
        let handler: TaskHandler = Arc::new(move |payload| Box::pin(handler(payload)));
        self.handlers.write().await.insert(name.into(), handler);
    }

    /// Enqueue a task
    pub async fn enqueue(&self, task: Task) -> Result<String, TaskError> {
        let task_id = task.id.clone();
        self.pending.lock().await.push(task);
        self.stats.write().await.total_enqueued += 1;
        Ok(task_id)
    }

    /// Get the next task to execute
    async fn next_task(&self) -> Option<Task> {
        let mut pending = self.pending.lock().await;

        // Find the first task that should execute now
        let mut tasks = Vec::new();
        while let Some(task) = pending.pop() {
            if task.should_execute() {
                return Some(task);
            }
            tasks.push(task);
        }

        // Put back tasks that aren't ready
        for task in tasks {
            pending.push(task);
        }

        None
    }

    /// Execute a single task
    async fn execute_task(&self, mut task: Task) -> Result<(), TaskError> {
        // Find handler
        let handler = {
            let handlers = self.handlers.read().await;
            handlers
                .get(&task.name)
                .cloned()
                .ok_or_else(|| TaskError::HandlerNotFound(task.name.clone()))?
        };

        // Update status
        task.status = TaskStatus::Running;
        self.running.write().await.push(task.clone());

        // Execute with timeout
        let result = if let Some(timeout) = task.timeout {
            match tokio::time::timeout(timeout, handler(task.payload.clone())).await {
                Ok(result) => result,
                Err(_) => Err(TaskError::Timeout),
            }
        } else {
            handler(task.payload.clone()).await
        };

        // Remove from running
        self.running
            .write()
            .await
            .retain(|t| t.id != task.id);

        // Handle result
        match result {
            Ok(_) => {
                task.status = TaskStatus::Completed;
                self.stats.write().await.total_completed += 1;
                Ok(())
            }
            Err(err) => {
                task.retry_count += 1;

                if task.retry_count >= task.max_retries {
                    task.status = TaskStatus::Failed;
                    self.stats.write().await.total_failed += 1;
                    Err(TaskError::MaxRetriesExceeded)
                } else {
                    // Retry with backoff
                    let backoff = task.backoff_duration();
                    task.execute_at = Instant::now() + backoff;
                    task.status = TaskStatus::Pending;
                    self.pending.lock().await.push(task);
                    Err(err)
                }
            }
        }
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let mut stats = self.stats.read().await.clone();
        stats.pending = self.pending.lock().await.len();
        stats.running = self.running.read().await.len();
        stats
    }

    /// Shutdown the queue
    pub async fn shutdown(&self) {
        *self.shutdown.lock().await = true;
    }

    /// Check if queue is shutdown
    async fn is_shutdown(&self) -> bool {
        *self.shutdown.lock().await
    }

    /// Start processing tasks (single worker)
    pub async fn start(&self) {
        while !self.is_shutdown().await {
            if let Some(task) = self.next_task().await {
                if let Err(e) = self.execute_task(task).await {
                    tracing::error!("Task execution error: {}", e);
                }
            } else {
                // No tasks ready, sleep briefly
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_creation() {
        let payload = TaskPayload::new(serde_json::json!({"test": "data"})).unwrap();
        let task = Task::new("test_task", payload);

        assert_eq!(task.name, "test_task");
        assert_eq!(task.priority, Priority::Normal);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_task_priority() {
        let payload = TaskPayload::new(serde_json::json!({})).unwrap();

        let low = Task::new("low", payload.clone()).with_priority(Priority::Low);
        let high = Task::new("high", payload.clone()).with_priority(Priority::High);

        assert!(high > low);
    }

    #[tokio::test]
    async fn test_task_backoff() {
        let payload = TaskPayload::new(serde_json::json!({})).unwrap();
        let mut task = Task::new("test", payload);

        task.retry_count = 0;
        assert_eq!(task.backoff_duration(), Duration::from_secs(1));

        task.retry_count = 1;
        assert_eq!(task.backoff_duration(), Duration::from_secs(2));

        task.retry_count = 3;
        assert_eq!(task.backoff_duration(), Duration::from_secs(8));
    }

    #[tokio::test]
    async fn test_queue_enqueue() {
        let queue = TaskQueue::new();
        let payload = TaskPayload::new(serde_json::json!({"test": "data"})).unwrap();
        let task = Task::new("test_task", payload);

        let task_id = queue.enqueue(task).await.unwrap();
        assert!(!task_id.is_empty());

        let stats = queue.stats().await;
        assert_eq!(stats.total_enqueued, 1);
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn test_task_handler_registration() {
        let queue = TaskQueue::new();

        queue.register("test_handler", |_payload| async {
            Ok(())
        }).await;

        let handlers = queue.handlers.read().await;
        assert!(handlers.contains_key("test_handler"));
    }

    #[tokio::test]
    async fn test_task_execution() {
        let queue = TaskQueue::new();

        // Register handler
        queue.register("success_task", |_payload| async {
            Ok(())
        }).await;

        // Enqueue task
        let payload = TaskPayload::new(serde_json::json!({"test": "data"})).unwrap();
        let task = Task::new("success_task", payload);
        queue.enqueue(task).await.unwrap();

        // Execute task
        let task = queue.next_task().await.unwrap();
        let result = queue.execute_task(task).await;
        assert!(result.is_ok());

        let stats = queue.stats().await;
        assert_eq!(stats.total_completed, 1);
    }

    #[tokio::test]
    async fn test_task_retry() {
        let queue = TaskQueue::new();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        // Register handler that fails first time
        queue.register("retry_task", move |_payload| {
            let counter = counter_clone.clone();
            async move {
                let mut count = counter.lock().await;
                *count += 1;
                if *count < 2 {
                    Err(TaskError::ExecutionFailed("Simulated failure".to_string()))
                } else {
                    Ok(())
                }
            }
        }).await;

        // Enqueue task
        let payload = TaskPayload::new(serde_json::json!({})).unwrap();
        let task = Task::new("retry_task", payload).with_max_retries(3);
        queue.enqueue(task).await.unwrap();

        // First execution should fail and retry
        let task = queue.next_task().await.unwrap();
        let result = queue.execute_task(task).await;
        assert!(result.is_err());

        // Should have re-enqueued for retry
        let stats = queue.stats().await;
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn test_task_payload() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            value: i32,
            message: String,
        }

        let data = TestData {
            value: 42,
            message: "test".to_string(),
        };

        let payload = TaskPayload::new(&data).unwrap();
        let extracted: TestData = payload.extract().unwrap();

        assert_eq!(extracted, data);
    }
}
