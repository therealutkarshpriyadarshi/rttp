//! Worker pool for concurrent task processing
//!
//! This module provides a worker pool that processes tasks from a queue concurrently.

use super::queue::{Task, TaskQueue};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Worker pool configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of worker threads
    pub workers: usize,
    /// Worker name prefix
    pub name_prefix: String,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            workers: num_cpus::get(),
            name_prefix: "worker".to_string(),
        }
    }
}

impl WorkerConfig {
    /// Create a new worker configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of workers
    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers.max(1); // At least 1 worker
        self
    }

    /// Set worker name prefix
    pub fn with_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.name_prefix = prefix.into();
        self
    }
}

/// Worker pool for processing tasks
pub struct WorkerPool {
    /// Queue reference
    queue: Arc<TaskQueue>,
    /// Worker handles
    handles: Vec<JoinHandle<()>>,
    /// Configuration
    config: WorkerConfig,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(queue: Arc<TaskQueue>, config: WorkerConfig) -> Self {
        Self {
            queue,
            handles: Vec::new(),
            config,
        }
    }

    /// Start all workers
    pub async fn start(&mut self) {
        for i in 0..self.config.workers {
            let queue = self.queue.clone();
            let worker_name = format!("{}-{}", self.config.name_prefix, i);

            let handle = tokio::spawn(async move {
                tracing::info!("Worker {} started", worker_name);
                queue.start().await;
                tracing::info!("Worker {} stopped", worker_name);
            });

            self.handles.push(handle);
        }

        tracing::info!(
            "Worker pool started with {} workers",
            self.config.workers
        );
    }

    /// Stop all workers and wait for completion
    pub async fn stop(self) {
        self.queue.shutdown().await;

        for handle in self.handles {
            let _ = handle.await;
        }

        tracing::info!("Worker pool stopped");
    }

    /// Get number of active workers
    pub fn worker_count(&self) -> usize {
        self.handles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::queue::TaskPayload;

    #[tokio::test]
    async fn test_worker_config() {
        let config = WorkerConfig::new()
            .with_workers(4)
            .with_name_prefix("test-worker");

        assert_eq!(config.workers, 4);
        assert_eq!(config.name_prefix, "test-worker");
    }

    #[tokio::test]
    async fn test_worker_pool_creation() {
        let queue = Arc::new(TaskQueue::new());
        let config = WorkerConfig::new().with_workers(2);
        let pool = WorkerPool::new(queue, config);

        assert_eq!(pool.worker_count(), 0); // Not started yet
    }

    #[tokio::test]
    async fn test_worker_pool_start_stop() {
        let queue = Arc::new(TaskQueue::new());
        let config = WorkerConfig::new().with_workers(2);
        let mut pool = WorkerPool::new(queue.clone(), config);

        pool.start().await;
        assert_eq!(pool.worker_count(), 2);

        // Give workers time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        pool.stop().await;
    }

    #[tokio::test]
    async fn test_worker_pool_processes_tasks() {
        let queue = Arc::new(TaskQueue::new());

        // Register a simple handler
        let counter = Arc::new(tokio::sync::Mutex::new(0));
        let counter_clone = counter.clone();

        queue.register("count_task", move |_payload| {
            let counter = counter_clone.clone();
            async move {
                let mut count = counter.lock().await;
                *count += 1;
                Ok(())
            }
        }).await;

        // Start worker pool
        let config = WorkerConfig::new().with_workers(2);
        let mut pool = WorkerPool::new(queue.clone(), config);
        pool.start().await;

        // Enqueue some tasks
        for i in 0..5 {
            let payload = TaskPayload::new(serde_json::json!({"id": i})).unwrap();
            let task = Task::new("count_task", payload);
            queue.enqueue(task).await.unwrap();
        }

        // Wait for tasks to process
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Check that tasks were processed
        let count = *counter.lock().await;
        assert_eq!(count, 5);

        pool.stop().await;
    }
}
