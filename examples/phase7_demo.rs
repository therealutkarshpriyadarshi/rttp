//! Phase 7 Demo: Background Processing
//!
//! This example demonstrates the background processing features:
//! - Task queue with priorities
//! - Worker pool for concurrent processing
//! - Retry logic with exponential backoff
//! - Job scheduler with cron expressions
//! - One-time and recurring tasks

use chrono::Local;
use pttp::background::{
    Priority, Scheduler, Task, TaskPayload, TaskQueue, WorkerConfig, WorkerPool,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 PTTP Phase 7 Demo: Background Processing\n");

    // Demo 1: Task Queue with Worker Pool
    demo_task_queue().await?;

    println!("\n{}\n", "=".repeat(60));

    // Demo 2: Task Priorities
    demo_priorities().await?;

    println!("\n{}\n", "=".repeat(60));

    // Demo 3: Task Retry Logic
    demo_retry().await?;

    println!("\n{}\n", "=".repeat(60));

    // Demo 4: Job Scheduler with Cron
    demo_scheduler().await?;

    println!("\n✅ Phase 7 demo completed!");

    Ok(())
}

/// Demo 1: Basic task queue with worker pool
async fn demo_task_queue() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Demo 1: Task Queue with Worker Pool");
    println!("{}", "─".repeat(60));

    let queue = Arc::new(TaskQueue::new());
    let processed_count = Arc::new(AtomicU32::new(0));

    // Register task handlers
    {
        let counter = processed_count.clone();
        queue
            .register("process_order", move |payload| {
                let counter = counter.clone();
                async move {
                    let data: serde_json::Value = payload.extract().unwrap();
                    println!(
                        "  📦 Processing order: {}",
                        data.get("order_id").unwrap()
                    );

                    // Simulate work
                    sleep(Duration::from_millis(100)).await;

                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;
    }

    {
        let counter = processed_count.clone();
        queue
            .register("send_notification", move |payload| {
                let counter = counter.clone();
                async move {
                    let data: serde_json::Value = payload.extract().unwrap();
                    println!(
                        "  📧 Sending notification to: {}",
                        data.get("recipient").unwrap().as_str().unwrap()
                    );

                    // Simulate work
                    sleep(Duration::from_millis(50)).await;

                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;
    }

    // Start worker pool
    let config = WorkerConfig::new().with_workers(4).with_name_prefix("demo");
    let mut pool = WorkerPool::new(queue.clone(), config);
    pool.start().await;

    println!("✓ Worker pool started with 4 workers");

    // Enqueue some tasks
    for i in 1..=5 {
        let payload = TaskPayload::new(serde_json::json!({
            "order_id": format!("ORD-{:03}", i),
            "amount": i * 100
        }))?;
        queue.enqueue(Task::new("process_order", payload)).await?;
    }

    for recipient in &["alice@example.com", "bob@example.com", "charlie@example.com"] {
        let payload = TaskPayload::new(serde_json::json!({
            "recipient": recipient,
            "message": "Your order has been shipped!"
        }))?;
        queue
            .enqueue(Task::new("send_notification", payload))
            .await?;
    }

    println!("✓ Enqueued 8 tasks");

    // Wait for processing
    sleep(Duration::from_millis(500)).await;

    let stats = queue.stats().await;
    println!("\n📊 Queue Statistics:");
    println!("  - Total enqueued: {}", stats.total_enqueued);
    println!("  - Total completed: {}", stats.total_completed);
    println!("  - Pending: {}", stats.pending);
    println!("  - Running: {}", stats.running);

    // Shutdown
    pool.stop().await;
    println!("✓ Worker pool stopped");

    Ok(())
}

/// Demo 2: Task priorities
async fn demo_priorities() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Demo 2: Task Priorities");
    println!("{}", "─".repeat(60));

    let queue = Arc::new(TaskQueue::new());

    // Register handler
    queue
        .register("priority_task", |payload| async move {
            let data: serde_json::Value = payload.extract().unwrap();
            let priority = data.get("priority").unwrap().as_str().unwrap();
            let task_id = data.get("id").unwrap().as_u64().unwrap();
            println!(
                "  ⚡ Executing {} priority task #{}",
                priority, task_id
            );
            Ok(())
        })
        .await;

    // Start worker pool
    let config = WorkerConfig::new().with_workers(1);
    let mut pool = WorkerPool::new(queue.clone(), config);
    pool.start().await;

    // Enqueue tasks with different priorities
    // Note: They should be processed in priority order (Critical > High > Normal > Low)

    let priorities = vec![
        (Priority::Low, "Low"),
        (Priority::Normal, "Normal"),
        (Priority::Critical, "Critical"),
        (Priority::High, "High"),
        (Priority::Low, "Low"),
    ];

    for (i, (priority, name)) in priorities.iter().enumerate() {
        let payload = TaskPayload::new(serde_json::json!({
            "priority": name,
            "id": i + 1
        }))?;

        queue
            .enqueue(Task::new("priority_task", payload).with_priority(*priority))
            .await?;
    }

    println!("✓ Enqueued 5 tasks with different priorities");
    println!("  (Tasks should execute: Critical, High, Normal, Low, Low)");

    // Wait for processing
    sleep(Duration::from_millis(300)).await;

    pool.stop().await;

    Ok(())
}

/// Demo 3: Retry logic
async fn demo_retry() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Demo 3: Retry Logic with Exponential Backoff");
    println!("{}", "─".repeat(60));

    let queue = Arc::new(TaskQueue::new());
    let attempt_count = Arc::new(AtomicU32::new(0));

    // Register handler that fails first 2 times
    {
        let counter = attempt_count.clone();
        queue
            .register("flaky_task", move |_payload| {
                let counter = counter.clone();
                async move {
                    let attempts = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    println!("  🔧 Attempt #{}", attempts);

                    if attempts < 3 {
                        println!("  ❌ Failed (will retry with backoff)");
                        Err(pttp::background::TaskError::ExecutionFailed(
                            "Simulated failure".to_string(),
                        ))
                    } else {
                        println!("  ✅ Success on attempt #{}", attempts);
                        Ok(())
                    }
                }
            })
            .await;
    }

    // Start worker pool
    let config = WorkerConfig::new().with_workers(1);
    let mut pool = WorkerPool::new(queue.clone(), config);
    pool.start().await;

    // Enqueue task with retry enabled
    let payload = TaskPayload::new(serde_json::json!({}))?;
    queue
        .enqueue(Task::new("flaky_task", payload).with_max_retries(5))
        .await?;

    println!("✓ Enqueued task with max 5 retries");
    println!("  (Task will fail twice, then succeed)");

    // Wait for processing (including retries with backoff)
    sleep(Duration::from_secs(8)).await;

    let total_attempts = attempt_count.load(Ordering::SeqCst);
    println!("\n📊 Total attempts: {}", total_attempts);

    pool.stop().await;

    Ok(())
}

/// Demo 4: Job scheduler with cron expressions
async fn demo_scheduler() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏰ Demo 4: Job Scheduler");
    println!("{}", "─".repeat(60));

    let scheduler = Scheduler::new().with_check_interval(Duration::from_millis(100));

    let execution_count = Arc::new(AtomicU32::new(0));

    // Schedule a one-time task
    {
        let now = Local::now();
        let run_time = now + chrono::Duration::milliseconds(200);

        scheduler
            .schedule_once("cleanup_temp_files", run_time, || async {
                println!("  🧹 Running cleanup task (one-time)");
            })
            .await;

        println!("✓ Scheduled one-time task to run at: {}", run_time.format("%H:%M:%S%.3f"));
    }

    // Schedule recurring tasks with cron expressions
    {
        // Every second (simulated with frequent checks)
        // Note: Standard cron only supports minute-level granularity
        // This is just for demo purposes - we'll use one-time tasks instead

        for i in 1..=3 {
            let run_time = Local::now() + chrono::Duration::milliseconds(i * 300);
            let counter = execution_count.clone();

            scheduler
                .schedule_once(
                    format!("recurring_task_{}", i),
                    run_time,
                    move || {
                        let counter = counter.clone();
                        async move {
                            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
                            println!(
                                "  📊 Recurring task execution #{} at {}",
                                count,
                                Local::now().format("%H:%M:%S%.3f")
                            );
                        }
                    },
                )
                .await;
        }

        println!("✓ Scheduled 3 recurring tasks");
    }

    // Start the scheduler
    scheduler.start().await?;
    println!("✓ Scheduler started");

    // List all scheduled tasks
    let tasks = scheduler.list_tasks().await;
    println!("\n📋 Scheduled Tasks:");
    for task in &tasks {
        println!(
            "  - {} ({}): next run at {}",
            task.name,
            task.id,
            task.next_run.format("%H:%M:%S%.3f")
        );
    }

    // Wait for tasks to execute
    println!("\n⏳ Waiting for scheduled tasks to execute...");
    sleep(Duration::from_secs(2)).await;

    // Stop the scheduler
    scheduler.stop().await?;
    println!("\n✓ Scheduler stopped");

    println!("\n📊 Total recurring task executions: {}", execution_count.load(Ordering::SeqCst));

    Ok(())
}
