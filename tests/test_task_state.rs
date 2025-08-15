use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tokio;
use cai::task_state::*;

#[tokio::test]
async fn test_task_state_creation() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    let tasks = vec![
        json!({
            "id": "task1",
            "description": "First test task",
            "status": "pending"
        }),
        json!({
            "id": "task2", 
            "description": "Second test task",
            "status": "pending"
        })
    ];
    
    let result = task_manager.create_tasks("Test user query".to_string(), tasks.clone())?;
    
    assert_eq!(result.user_query, "Test user query");
    assert_eq!(result.tasks.len(), 2);
    assert!(result.created_at <= result.updated_at);
    
    Ok(())
}

#[tokio::test]
async fn test_task_state_updates() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    let tasks = vec![
        json!({
            "id": "task1",
            "description": "Test task",
            "status": "pending"
        })
    ];
    
    task_manager.create_tasks("Test query".to_string(), tasks)?;
    
    let updates = vec![
        json!({
            "id": "task1",
            "status": "in_progress",
            "notes": "Starting work"
        })
    ];
    
    let update_results = task_manager.update_tasks(&updates)?;
    
    assert_eq!(update_results.len(), 1);
    assert_eq!(update_results[0]["id"], "task1");
    assert_eq!(update_results[0]["old_status"], "pending");
    assert_eq!(update_results[0]["new_status"], "in_progress");
    
    Ok(())
}

#[tokio::test]
async fn test_task_state_validation() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    // Test invalid task structure
    let invalid_tasks = vec![
        json!({
            "description": "Missing id field"
        })
    ];
    
    let result = task_manager.create_tasks("Test".to_string(), invalid_tasks);
    assert!(result.is_err());
    
    // Test invalid status
    let tasks = vec![
        json!({
            "id": "task1",
            "description": "Test task",
            "status": "pending"
        })
    ];
    
    task_manager.create_tasks("Test".to_string(), tasks)?;
    
    let invalid_updates = vec![
        json!({
            "id": "task1",
            "status": "invalid_status"
        })
    ];
    
    let result = task_manager.update_tasks(&invalid_updates);
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_task_statistics() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    let tasks = vec![
        json!({"id": "t1", "description": "Task 1", "status": "pending"}),
        json!({"id": "t2", "description": "Task 2", "status": "pending"}),
        json!({"id": "t3", "description": "Task 3", "status": "pending"})
    ];
    
    task_manager.create_tasks("Test".to_string(), tasks)?;
    
    // Update some tasks
    let updates = vec![
        json!({"id": "t1", "status": "completed"}),
        json!({"id": "t2", "status": "failed"}),
    ];
    
    task_manager.update_tasks(&updates)?;
    
    let stats = task_manager.get_task_statistics()?;
    
    assert_eq!(stats.total_tasks, 3);
    assert_eq!(stats.pending_tasks, 1);
    assert_eq!(stats.completed_tasks, 1);
    assert_eq!(stats.failed_tasks, 1);
    assert!((stats.completion_rate - 33.333333333333336).abs() < 0.001);
    
    Ok(())
}

#[tokio::test]
async fn test_task_filters() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    let tasks = vec![
        json!({"id": "t1", "description": "Task 1", "status": "pending"}),
        json!({"id": "t2", "description": "Task 2", "status": "completed"}),
        json!({"id": "t3", "description": "Task 3", "status": "pending"})
    ];
    
    task_manager.create_tasks("Test".to_string(), tasks)?;
    
    let pending_tasks = task_manager.get_tasks_by_status("pending")?;
    assert_eq!(pending_tasks.len(), 2);
    
    let completed_tasks = task_manager.get_tasks_by_status("completed")?;
    assert_eq!(completed_tasks.len(), 1);
    
    let failed_tasks = task_manager.get_tasks_by_status("failed")?;
    assert_eq!(failed_tasks.len(), 0);
    
    Ok(())
}

#[tokio::test] 
async fn test_concurrent_access() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    let tasks = vec![
        json!({"id": "t1", "description": "Task 1", "status": "pending"})
    ];
    
    task_manager.create_tasks("Test".to_string(), tasks)?;
    
    // Spawn multiple concurrent update operations
    let handles: Vec<_> = (0..10).map(|i| {
        let tm = task_manager.clone();
        tokio::spawn(async move {
            let updates = vec![
                json!({"id": "t1", "status": "in_progress", "notes": format!("Update {}", i)})
            ];
            tm.update_tasks(&updates)
        })
    }).collect();
    
    // Wait for all operations
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_global_instance() -> Result<()> {
    let global_manager = get_global_task_state();
    
    let tasks = vec![
        json!({"id": "global_test", "description": "Global test task", "status": "pending"})
    ];
    
    global_manager.create_tasks("Global test".to_string(), tasks)?;
    
    // Access from another reference to global instance
    let same_manager = get_global_task_state();
    let retrieved_tasks = same_manager.get_tasks()?;
    
    assert!(retrieved_tasks.is_some());
    assert_eq!(retrieved_tasks.unwrap().tasks.len(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_error_conditions() -> Result<()> {
    let task_manager = TaskStateManager::new();
    
    // Test updating non-existent task
    let updates = vec![
        json!({"id": "nonexistent", "status": "completed"})
    ];
    
    let result = task_manager.update_tasks(&updates);
    assert!(result.is_err());
    
    // Test getting tasks when none exist
    let stats = task_manager.get_task_statistics()?;
    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.completion_rate, 0.0);
    
    Ok(())
}

#[test]
fn test_task_statistics_default() {
    let stats = TaskStatistics::default();
    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.completion_rate, 0.0);
}