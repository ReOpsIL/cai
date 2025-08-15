use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::{Arc, RwLock, OnceLock};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::logger::{log_debug, log_info};

/// Thread-safe task state manager
#[derive(Debug, Clone)]
pub struct TaskStateManager {
    tasks: Arc<RwLock<Option<TaskList>>>,
}

#[derive(Debug, Clone)]
pub struct TaskList {
    pub user_query: String,
    pub tasks: Vec<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

impl TaskStateManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Create a new task list
    pub fn create_tasks(&self, user_query: String, tasks: Vec<Value>) -> Result<TaskList> {
        // Validate task structure
        for (i, task) in tasks.iter().enumerate() {
            let task_obj = task.as_object()
                .ok_or_else(|| anyhow!("task {} is not an object", i))?;
            
            if !task_obj.contains_key("id") || !task_obj.contains_key("description") {
                return Err(anyhow!("task {} missing required fields (id, description)", i));
            }
        }
        
        let now = Utc::now();
        let task_list = TaskList {
            user_query: user_query.clone(),
            tasks: tasks.clone(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        };
        
        // Store the task list
        {
            let mut guard = self.tasks.write()
                .map_err(|_| anyhow!("Failed to acquire write lock on task state"))?;
            *guard = Some(task_list.clone());
        }
        
        log_info!("task_state", "Created task list with {} tasks for: {}", tasks.len(), user_query);
        Ok(task_list)
    }
    
    /// Update existing tasks
    pub fn update_tasks(&self, task_updates: &[Value]) -> Result<Vec<Value>> {
        let mut guard = self.tasks.write()
            .map_err(|_| anyhow!("Failed to acquire write lock on task state"))?;
        
        let task_list = guard.as_mut()
            .ok_or_else(|| anyhow!("No task list exists. Create tasks first."))?;
        
        let mut updates_made = Vec::new();
        
        for update in task_updates {
            let update_obj = update.as_object()
                .ok_or_else(|| anyhow!("update is not an object"))?;
            
            let update_id = update_obj.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("update missing 'id'"))?;
            
            let new_status = update_obj.get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("update missing 'status'"))?;
            
            // Validate status
            if !["pending", "in_progress", "completed", "failed", "cancelled"].contains(&new_status) {
                return Err(anyhow!("Invalid status: {}. Valid statuses: pending, in_progress, completed, failed, cancelled", new_status));
            }
            
            // Find and update the task
            let mut task_found = false;
            for task in task_list.tasks.iter_mut() {
                if let Some(task_obj) = task.as_object_mut() {
                    if let Some(task_id) = task_obj.get("id").and_then(|v| v.as_str()) {
                        if task_id == update_id {
                            let old_status = task_obj.get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("pending")
                                .to_string();
                            
                            task_obj.insert("status".to_string(), Value::String(new_status.to_string()));
                            task_obj.insert("updated_at".to_string(), 
                                           Value::String(Utc::now().to_rfc3339()));
                            
                            // Add optional fields from update
                            if let Some(notes) = update_obj.get("notes") {
                                task_obj.insert("notes".to_string(), notes.clone());
                            }
                            
                            if let Some(result) = update_obj.get("result") {
                                task_obj.insert("result".to_string(), result.clone());
                            }
                            
                            if let Some(error) = update_obj.get("error") {
                                task_obj.insert("error".to_string(), error.clone());
                            }
                            
                            updates_made.push(serde_json::json!({
                                "id": update_id,
                                "old_status": old_status,
                                "new_status": new_status
                            }));
                            
                            task_found = true;
                            break;
                        }
                    }
                }
            }
            
            if !task_found {
                return Err(anyhow!("Task '{}' not found", update_id));
            }
        }
        
        // Update the task list timestamp
        task_list.updated_at = Utc::now();
        
        log_debug!("task_state", "Updated {} task(s)", updates_made.len());
        Ok(updates_made)
    }
    
    /// Get current task list
    pub fn get_tasks(&self) -> Result<Option<TaskList>> {
        let guard = self.tasks.read()
            .map_err(|_| anyhow!("Failed to acquire read lock on task state"))?;
        Ok(guard.clone())
    }
    
    /// Get tasks with a specific status
    pub fn get_tasks_by_status(&self, status: &str) -> Result<Vec<Value>> {
        let guard = self.tasks.read()
            .map_err(|_| anyhow!("Failed to acquire read lock on task state"))?;
        
        if let Some(task_list) = guard.as_ref() {
            let filtered_tasks = task_list.tasks
                .iter()
                .filter(|task| {
                    task.as_object()
                        .and_then(|obj| obj.get("status"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending") == status
                })
                .cloned()
                .collect();
            
            Ok(filtered_tasks)
        } else {
            Ok(vec![])
        }
    }
    
    /// Clear current task list
    pub fn clear_tasks(&self) -> Result<()> {
        let mut guard = self.tasks.write()
            .map_err(|_| anyhow!("Failed to acquire write lock on task state"))?;
        *guard = None;
        log_info!("task_state", "Cleared task list");
        Ok(())
    }
    
    /// Get task statistics
    pub fn get_task_statistics(&self) -> Result<TaskStatistics> {
        let guard = self.tasks.read()
            .map_err(|_| anyhow!("Failed to acquire read lock on task state"))?;
        
        if let Some(task_list) = guard.as_ref() {
            let mut stats = TaskStatistics::default();
            stats.total_tasks = task_list.tasks.len();
            
            for task in &task_list.tasks {
                if let Some(task_obj) = task.as_object() {
                    let status = task_obj.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending");
                    
                    match status {
                        "pending" => stats.pending_tasks += 1,
                        "in_progress" => stats.in_progress_tasks += 1,
                        "completed" => stats.completed_tasks += 1,
                        "failed" => stats.failed_tasks += 1,
                        "cancelled" => stats.cancelled_tasks += 1,
                        _ => {}
                    }
                }
            }
            
            stats.completion_rate = if stats.total_tasks > 0 {
                (stats.completed_tasks as f64 / stats.total_tasks as f64) * 100.0
            } else {
                0.0
            };
            
            Ok(stats)
        } else {
            Ok(TaskStatistics::default())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskStatistics {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub in_progress_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub cancelled_tasks: usize,
    pub completion_rate: f64,
}

/// Global task state manager instance
static GLOBAL_TASK_STATE: OnceLock<TaskStateManager> = OnceLock::new();

/// Get the global task state manager
pub fn get_global_task_state() -> &'static TaskStateManager {
    GLOBAL_TASK_STATE.get_or_init(|| {
        log_info!("task_state", "Initializing global task state manager");
        TaskStateManager::new()
    })
}