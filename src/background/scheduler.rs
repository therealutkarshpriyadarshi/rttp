//! Task scheduler with cron support
//!
//! This module provides a flexible task scheduler that supports:
//! - One-time scheduled tasks
//! - Recurring tasks with cron expressions
//! - Task registration and management

use super::cron::{CronError, CronExpr};
use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// Scheduler error
#[derive(Debug, Clone)]
pub enum SchedulerError {
    /// Invalid cron expression
    InvalidCron(String),
    /// Task not found
    TaskNotFound(String),
    /// Scheduler is already running
    AlreadyRunning,
    /// Scheduler is not running
    NotRunning,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::InvalidCron(msg) => write!(f, "Invalid cron expression: {}", msg),
            SchedulerError::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            SchedulerError::AlreadyRunning => write!(f, "Scheduler is already running"),
            SchedulerError::NotRunning => write!(f, "Scheduler is not running"),
        }
    }
}

impl std::error::Error for SchedulerError {}

impl From<CronError> for SchedulerError {
    fn from(err: CronError) -> Self {
        SchedulerError::InvalidCron(err.to_string())
    }
}

/// Scheduled task handler
pub type ScheduledHandler = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Schedule type
#[derive(Debug, Clone)]
enum Schedule {
    /// One-time execution at a specific datetime
    Once(DateTime<Local>),
    /// Recurring execution based on cron expression
    Cron(CronExpr),
}

/// Scheduled task
struct ScheduledTask {
    /// Task ID
    id: String,
    /// Task name
    name: String,
    /// Schedule
    schedule: Schedule,
    /// Task handler
    handler: ScheduledHandler,
    /// Next execution time
    next_run: DateTime<Local>,
    /// Whether the task is enabled
    enabled: bool,
    /// Number of times this task has run
    run_count: u64,
}

impl ScheduledTask {
    /// Create a new one-time scheduled task
    fn new_once(
        id: String,
        name: String,
        at: DateTime<Local>,
        handler: ScheduledHandler,
    ) -> Self {
        Self {
            id,
            name,
            schedule: Schedule::Once(at),
            handler,
            next_run: at,
            enabled: true,
            run_count: 0,
        }
    }

    /// Create a new recurring scheduled task
    fn new_recurring(
        id: String,
        name: String,
        cron: CronExpr,
        handler: ScheduledHandler,
    ) -> Self {
        let next_run = cron.next(&Local::now()).unwrap_or_else(Local::now);
        Self {
            id,
            name,
            schedule: Schedule::Cron(cron),
            handler,
            next_run,
            enabled: true,
            run_count: 0,
        }
    }

    /// Check if task should run now
    fn should_run(&self) -> bool {
        self.enabled && Local::now() >= self.next_run
    }

    /// Calculate next run time
    fn calculate_next_run(&mut self) {
        match &self.schedule {
            Schedule::Once(_) => {
                // One-time tasks are disabled after running
                self.enabled = false;
            }
            Schedule::Cron(expr) => {
                // Find next execution time
                if let Some(next) = expr.next(&Local::now()) {
                    self.next_run = next;
                } else {
                    self.enabled = false;
                }
            }
        }
    }
}

/// Task scheduler
pub struct Scheduler {
    /// Scheduled tasks
    tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,
    /// Running state
    running: Arc<Mutex<bool>>,
    /// Worker handle
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Check interval
    check_interval: Duration,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
            handle: Arc::new(Mutex::new(None)),
            check_interval: Duration::from_secs(1), // Check every second
        }
    }

    /// Set the check interval
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Schedule a one-time task
    pub async fn schedule_once<F, Fut>(
        &self,
        name: impl Into<String>,
        at: DateTime<Local>,
        handler: F,
    ) -> String
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let id = uuid::Uuid::new_v4().to_string();
        let name = name.into();
        let handler: ScheduledHandler = Arc::new(move || Box::pin(handler()));

        let task = ScheduledTask::new_once(id.clone(), name, at, handler);

        self.tasks.write().await.insert(id.clone(), task);

        tracing::info!("Scheduled one-time task: {}", id);

        id
    }

    /// Schedule a recurring task with cron expression
    pub async fn schedule_cron<F, Fut>(
        &self,
        name: impl Into<String>,
        cron_expr: &str,
        handler: F,
    ) -> Result<String, SchedulerError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let id = uuid::Uuid::new_v4().to_string();
        let name = name.into();
        let cron = CronExpr::parse(cron_expr)?;
        let handler: ScheduledHandler = Arc::new(move || Box::pin(handler()));

        let task = ScheduledTask::new_recurring(id.clone(), name, cron, handler);

        self.tasks.write().await.insert(id.clone(), task);

        tracing::info!("Scheduled cron task: {} ({})", id, cron_expr);

        Ok(id)
    }

    /// Remove a scheduled task
    pub async fn remove(&self, task_id: &str) -> Result<(), SchedulerError> {
        self.tasks
            .write()
            .await
            .remove(task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id.to_string()))?;

        tracing::info!("Removed scheduled task: {}", task_id);

        Ok(())
    }

    /// Enable a task
    pub async fn enable(&self, task_id: &str) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id.to_string()))?;

        task.enabled = true;

        tracing::info!("Enabled scheduled task: {}", task_id);

        Ok(())
    }

    /// Disable a task
    pub async fn disable(&self, task_id: &str) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| SchedulerError::TaskNotFound(task_id.to_string()))?;

        task.enabled = false;

        tracing::info!("Disabled scheduled task: {}", task_id);

        Ok(())
    }

    /// Get task information
    pub async fn get_task(&self, task_id: &str) -> Option<ScheduledTaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).map(|task| ScheduledTaskInfo {
            id: task.id.clone(),
            name: task.name.clone(),
            next_run: task.next_run,
            enabled: task.enabled,
            run_count: task.run_count,
        })
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<ScheduledTaskInfo> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .map(|task| ScheduledTaskInfo {
                id: task.id.clone(),
                name: task.name.clone(),
                next_run: task.next_run,
                enabled: task.enabled,
                run_count: task.run_count,
            })
            .collect()
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<(), SchedulerError> {
        let mut running = self.running.lock().await;
        if *running {
            return Err(SchedulerError::AlreadyRunning);
        }

        *running = true;
        drop(running);

        let tasks = self.tasks.clone();
        let running_flag = self.running.clone();
        let check_interval = self.check_interval;

        let handle = tokio::spawn(async move {
            tracing::info!("Scheduler started");

            while *running_flag.lock().await {
                // Check all tasks
                let mut tasks_to_run = Vec::new();

                {
                    let tasks_read = tasks.read().await;
                    for task in tasks_read.values() {
                        if task.should_run() {
                            tasks_to_run.push((task.id.clone(), task.handler.clone()));
                        }
                    }
                }

                // Execute tasks that should run
                for (task_id, handler) in tasks_to_run {
                    let handler_clone = handler.clone();
                    let tasks_clone = tasks.clone();

                    tokio::spawn(async move {
                        tracing::debug!("Executing scheduled task: {}", task_id);

                        // Run the handler
                        handler_clone().await;

                        // Update task
                        let mut tasks_write = tasks_clone.write().await;
                        if let Some(task) = tasks_write.get_mut(&task_id) {
                            task.run_count += 1;
                            task.calculate_next_run();

                            if !task.enabled {
                                tracing::info!(
                                    "One-time task completed and disabled: {}",
                                    task_id
                                );
                            }
                        }
                    });
                }

                // Sleep before next check
                sleep(check_interval).await;
            }

            tracing::info!("Scheduler stopped");
        });

        *self.handle.lock().await = Some(handle);

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<(), SchedulerError> {
        let mut running = self.running.lock().await;
        if !*running {
            return Err(SchedulerError::NotRunning);
        }

        *running = false;
        drop(running);

        // Wait for handle to complete
        if let Some(handle) = self.handle.lock().await.take() {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Check if scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduled task information
#[derive(Debug, Clone)]
pub struct ScheduledTaskInfo {
    /// Task ID
    pub id: String,
    /// Task name
    pub name: String,
    /// Next execution time
    pub next_run: DateTime<Local>,
    /// Whether the task is enabled
    pub enabled: bool,
    /// Number of times this task has run
    pub run_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_schedule_once() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let future_time = Local::now() + chrono::Duration::milliseconds(100);

        let task_id = scheduler
            .schedule_once("test_once", future_time, move || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
            .await;

        assert!(!task_id.is_empty());

        let task_info = scheduler.get_task(&task_id).await.unwrap();
        assert_eq!(task_info.name, "test_once");
        assert!(task_info.enabled);
    }

    #[tokio::test]
    async fn test_schedule_cron() {
        let scheduler = Scheduler::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Every minute
        let task_id = scheduler
            .schedule_cron("test_cron", "* * * * *", move || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
            .await
            .unwrap();

        assert!(!task_id.is_empty());

        let task_info = scheduler.get_task(&task_id).await.unwrap();
        assert_eq!(task_info.name, "test_cron");
        assert!(task_info.enabled);
    }

    #[tokio::test]
    async fn test_invalid_cron() {
        let scheduler = Scheduler::new();

        let result = scheduler
            .schedule_cron("invalid", "invalid cron", || async {})
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let scheduler = Scheduler::new();
        let future_time = Local::now() + chrono::Duration::hours(1);

        let task_id = scheduler
            .schedule_once("test_task", future_time, || async {})
            .await;

        // Disable
        scheduler.disable(&task_id).await.unwrap();
        let task_info = scheduler.get_task(&task_id).await.unwrap();
        assert!(!task_info.enabled);

        // Enable
        scheduler.enable(&task_id).await.unwrap();
        let task_info = scheduler.get_task(&task_id).await.unwrap();
        assert!(task_info.enabled);
    }

    #[tokio::test]
    async fn test_remove() {
        let scheduler = Scheduler::new();
        let future_time = Local::now() + chrono::Duration::hours(1);

        let task_id = scheduler
            .schedule_once("test_task", future_time, || async {})
            .await;

        assert!(scheduler.get_task(&task_id).await.is_some());

        scheduler.remove(&task_id).await.unwrap();

        assert!(scheduler.get_task(&task_id).await.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let scheduler = Scheduler::new();
        let future_time = Local::now() + chrono::Duration::hours(1);

        scheduler
            .schedule_once("task1", future_time, || async {})
            .await;

        scheduler
            .schedule_once("task2", future_time, || async {})
            .await;

        let tasks = scheduler.list_tasks().await;
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let scheduler = Scheduler::new();

        assert!(!scheduler.is_running().await);

        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);

        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running().await);
    }

    #[tokio::test]
    async fn test_scheduler_execution() {
        let scheduler = Scheduler::new().with_check_interval(Duration::from_millis(50));
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        // Schedule task to run in 100ms
        let run_time = Local::now() + chrono::Duration::milliseconds(100);

        scheduler
            .schedule_once("test_execution", run_time, move || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
            .await;

        // Start scheduler
        scheduler.start().await.unwrap();

        // Wait for task to execute
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Check that task ran
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Stop scheduler
        scheduler.stop().await.unwrap();
    }
}
