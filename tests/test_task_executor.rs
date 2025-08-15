use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;
use std::fs;
use cai::task_executor::*;
use cai::openrouter_client::OpenRouterClient;

/// Mock OpenRouter client for testing
struct MockOpenRouterClient;

impl MockOpenRouterClient {
    fn new() -> Self {
        Self
    }
    
    async fn plan_tasks(&self, _user_request: &str) -> Result<Vec<String>> {
        // Return mock tasks
        Ok(vec![
            "TASK id=t1; title=\"Read project files\"; tool=read_file; params={\"path\":\"/project/README.md\"}; depends_on=[]; action=\"Read the main project documentation\"".to_string(),
            "TASK id=t2; title=\"List source files\"; tool=list_directory; params={\"path\":\"/project/src\"}; depends_on=[\"t1\"]; action=\"List all source code files\"".to_string(),
        ])
    }
}

#[tokio::test]
async fn test_task_executor_creation() {
    let executor = TaskExecutor::new(None);
    
    // Test initial state
    assert!(!executor.is_running().await);
    
    let status = executor.get_queue_status().await;
    // Should start with empty queue
    let queue = executor.queue.lock().await;
    assert_eq!(queue.len(), 0);
}

#[tokio::test]
async fn test_task_parsing() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=t1; title=\"Test Task\"; tool=read_file; params={\"path\":\"/test.txt\"}; depends_on=[]; action=\"Read a test file\"".to_string(),
        "TASK id=t2; title=\"Another Task\"; tool=list_directory; params={\"path\":\"/src\"}; depends_on=[\"t1\"]; action=\"List directory contents\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    let queue = executor.queue.lock().await;
    assert_eq!(queue.len(), 2);
    
    // Check first task
    assert_eq!(queue[0].id, "t1");
    assert_eq!(queue[0].title, "Test Task");
    assert_eq!(queue[0].description, "Read a test file");
    assert!(queue[0].dependencies.is_empty());
    
    // Check second task
    assert_eq!(queue[1].id, "t2");
    assert_eq!(queue[1].dependencies, vec!["t1".to_string()]);
    
    Ok(())
}

#[tokio::test]
async fn test_task_dependency_resolution() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=t3; title=\"Third\"; tool=read_file; params={}; depends_on=[\"t1\",\"t2\"]; action=\"Depends on t1 and t2\"".to_string(),
        "TASK id=t1; title=\"First\"; tool=read_file; params={}; depends_on=[]; action=\"No dependencies\"".to_string(),
        "TASK id=t2; title=\"Second\"; tool=read_file; params={}; depends_on=[\"t1\"]; action=\"Depends on t1\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    let queue = executor.queue.lock().await;
    assert_eq!(queue.len(), 3);
    
    // Tasks should be sorted by dependencies (topological sort)
    // t1 should come first (no deps), then t2 (depends on t1), then t3 (depends on t1,t2)
    assert_eq!(queue[0].id, "t1");
    assert_eq!(queue[1].id, "t2");
    assert_eq!(queue[2].id, "t3");
    
    Ok(())
}

#[tokio::test]
async fn test_task_status_tracking() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=t1; title=\"Test\"; tool=read_file; params={}; depends_on=[]; action=\"Test task\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    {
        let mut queue = executor.queue.lock().await;
        assert_eq!(queue[0].status, TaskStatus::Waiting);
        
        // Simulate status change
        queue[0].status = TaskStatus::Running;
    }
    
    let queue = executor.queue.lock().await;
    assert_eq!(queue[0].status, TaskStatus::Running);
    
    Ok(())
}

#[tokio::test]
async fn test_task_clear() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=t1; title=\"Test\"; tool=read_file; params={}; depends_on=[]; action=\"Test task\"".to_string(),
        "TASK id=t2; title=\"Test2\"; tool=read_file; params={}; depends_on=[]; action=\"Test task 2\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    {
        let queue = executor.queue.lock().await;
        assert_eq!(queue.len(), 2);
    }
    
    executor.clear_tasks().await;
    
    {
        let queue = executor.queue.lock().await;
        assert_eq!(queue.len(), 0);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_invalid_task_parsing() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    // Test malformed task string
    let invalid_task_strings = vec![
        "INVALID TASK FORMAT".to_string(),
        "TASK id=; title=; tool=; params=; depends_on=; action=".to_string(),
    ];
    
    // Should handle invalid tasks gracefully
    let result = executor.add_tasks_from_strings(invalid_task_strings).await;
    // The implementation should either skip invalid tasks or return an error
    // Either behavior is acceptable as long as it doesn't crash
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_task_operations() -> Result<()> {
    let executor = Arc::new(TaskExecutor::new(None));
    
    // Add tasks from multiple threads concurrently
    let handles: Vec<_> = (0..5).map(|i| {
        let executor = executor.clone();
        tokio::spawn(async move {
            let task_strings = vec![
                format!("TASK id=t{}; title=\"Task {}\"; tool=read_file; params={{}}; depends_on=[]; action=\"Test task {}\"", i, i, i),
            ];
            executor.add_tasks_from_strings(task_strings).await
        })
    }).collect();
    
    // Wait for all operations
    for handle in handles {
        handle.await.unwrap()?;
    }
    
    let queue = executor.queue.lock().await;
    assert_eq!(queue.len(), 5);
    
    Ok(())
}

#[tokio::test]
async fn test_task_status_display() -> Result<()> {
    // Test TaskStatus display methods
    assert_eq!(TaskStatus::Waiting.icon(), "⏳");
    assert_eq!(TaskStatus::Running.icon(), "🔄");
    assert_eq!(TaskStatus::Done.icon(), "✅");
    assert_eq!(TaskStatus::Failed.icon(), "❌");
    
    Ok(())
}

#[tokio::test]
async fn test_task_execution_blocking() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=t1; title=\"Test\"; tool=read_file; params={}; depends_on=[]; action=\"Test task\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    // Try to execute twice simultaneously - should block
    let executor1 = Arc::new(executor);
    let executor2 = executor1.clone();
    
    let handle1 = tokio::spawn(async move {
        executor1.execute_all().await
    });
    
    // Give first execution a chance to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    let handle2 = tokio::spawn(async move {
        executor2.execute_all().await
    });
    
    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();
    
    // One should succeed, one should fail with "already in progress"
    let success_count = [&result1, &result2].iter().filter(|r| r.is_ok()).count();
    let failure_count = [&result1, &result2].iter().filter(|r| r.is_err()).count();
    
    assert!(success_count <= 1); // At most one should succeed
    assert!(failure_count >= 1); // At least one should fail
    
    if let Err(e) = &result2 {
        assert!(e.to_string().contains("already in progress"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_heuristic_fallback() -> Result<()> {
    // Test that heuristic analysis works when LLM is not available
    let executor = TaskExecutor::new(None); // No LLM client
    
    // This would normally use LLM analysis, but should fall back to heuristics
    // We can't easily test the internal method, but we can verify the fallback exists
    // by checking that the executor doesn't panic when no LLM is available
    
    let task_strings = vec![
        "TASK id=t1; title=\"Read files\"; tool=read_file; params={}; depends_on=[]; action=\"Read project files\"".to_string(),
    ];
    
    let result = executor.add_tasks_from_strings(task_strings).await;
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_task_metadata() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=test_task; title=\"Complex Task\"; tool=multiedit_file; params={\"file_path\":\"/test.txt\",\"edits\":[]}; depends_on=[]; action=\"Perform complex file editing\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    let queue = executor.queue.lock().await;
    let task = &queue[0];
    
    assert_eq!(task.id, "test_task");
    assert_eq!(task.title, "Complex Task");
    assert_eq!(task.description, "Perform complex file editing");
    assert_eq!(task.dependencies.len(), 0);
    assert!(task.mcp_calls.is_empty()); // Initially empty
    
    Ok(())
}

#[tokio::test]
async fn test_queue_status_reporting() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    let task_strings = vec![
        "TASK id=t1; title=\"Task 1\"; tool=read_file; params={}; depends_on=[]; action=\"First task\"".to_string(),
        "TASK id=t2; title=\"Task 2\"; tool=read_file; params={}; depends_on=[]; action=\"Second task\"".to_string(),
        "TASK id=t3; title=\"Task 3\"; tool=read_file; params={}; depends_on=[]; action=\"Third task\"".to_string(),
    ];
    
    executor.add_tasks_from_strings(task_strings).await?;
    
    // Simulate different task states
    {
        let mut queue = executor.queue.lock().await;
        queue[0].status = TaskStatus::Done;
        queue[1].status = TaskStatus::Running;
        queue[2].status = TaskStatus::Failed;
    }
    
    // Test status reporting (this mainly tests that it doesn't crash)
    executor.show_queue_status().await;
    
    Ok(())
}

#[test]
fn test_task_display() {
    let task = Task {
        id: "test123".to_string(),
        title: "Test Task".to_string(),
        description: "This is a test task description".to_string(),
        status: TaskStatus::Waiting,
        dependencies: vec!["dep1".to_string()],
        mcp_calls: vec![],
    };
    
    let summary = task.display_summary();
    assert!(summary.contains("⏳")); // Status icon
    assert!(summary.contains("Test Task")); // Title
    assert!(summary.contains("test123")); // ID
}

#[tokio::test]
async fn test_circular_dependency_detection() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    // Create circular dependency: t1 -> t2 -> t3 -> t1
    let task_strings = vec![
        "TASK id=t1; title=\"Task 1\"; tool=read_file; params={}; depends_on=[\"t3\"]; action=\"Depends on t3\"".to_string(),
        "TASK id=t2; title=\"Task 2\"; tool=read_file; params={}; depends_on=[\"t1\"]; action=\"Depends on t1\"".to_string(),
        "TASK id=t3; title=\"Task 3\"; tool=read_file; params={}; depends_on=[\"t2\"]; action=\"Depends on t2\"".to_string(),
    ];
    
    let result = executor.add_tasks_from_strings(task_strings).await;
    
    // The implementation should either:
    // 1. Detect and reject circular dependencies, or
    // 2. Handle them gracefully by falling back to original order
    // Either is acceptable as long as it doesn't infinite loop
    
    if result.is_ok() {
        let queue = executor.queue.lock().await;
        assert_eq!(queue.len(), 3); // Should still have all tasks
    }
    
    Ok(())
}

#[tokio::test]
async fn test_empty_task_list() -> Result<()> {
    let executor = TaskExecutor::new(None);
    
    executor.add_tasks_from_strings(vec![]).await?;
    
    let queue = executor.queue.lock().await;
    assert_eq!(queue.len(), 0);
    
    // Should handle empty execution gracefully
    drop(queue);
    let result = executor.execute_all().await;
    assert!(result.is_ok());
    
    Ok(())
}