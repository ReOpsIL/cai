use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

pub type TaskId = String;
pub type ProgressListenerId = String;

/// Real-time progress tracking system for long-running operations
#[derive(Debug)]
pub struct ProgressTracker {
    /// Currently active tasks
    current_tasks: Arc<RwLock<HashMap<TaskId, TaskProgress>>>,
    /// Event broadcaster for real-time updates
    event_sender: broadcast::Sender<ProgressEvent>,
    /// Whether UI updates are enabled
    ui_enabled: bool,
    /// Maximum number of concurrent progress listeners
    max_listeners: usize,
    /// History of completed tasks (limited size)
    task_history: Arc<RwLock<Vec<CompletedTask>>>,
    /// Maximum history size
    max_history_size: usize,
}

/// Progress information for a specific task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: TaskId,
    pub title: String,
    pub description: String,
    pub current_step: String,
    pub progress_percentage: f32,
    pub started_at: SystemTime,
    pub estimated_completion: Option<SystemTime>,
    pub substeps: Vec<SubStep>,
    pub current_substep_index: usize,
    pub status: TaskStatus,
    pub metadata: HashMap<String, String>,
    pub cancellation_token: Option<String>,
}

/// Sub-step within a larger task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub weight: f32, // Relative weight for progress calculation
    pub status: SubStepStatus,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub error: Option<String>,
}

/// Status of a task
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Status of a sub-step
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Progress events for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    TaskStarted {
        task_id: TaskId,
        title: String,
        estimated_duration: Option<Duration>,
    },
    TaskUpdated {
        task_id: TaskId,
        progress_percentage: f32,
        current_step: String,
        estimated_completion: Option<SystemTime>,
    },
    SubStepStarted {
        task_id: TaskId,
        substep_id: String,
        substep_name: String,
    },
    SubStepCompleted {
        task_id: TaskId,
        substep_id: String,
        success: bool,
        error: Option<String>,
    },
    TaskCompleted {
        task_id: TaskId,
        success: bool,
        duration: Duration,
        result_summary: Option<String>,
    },
    TaskPaused {
        task_id: TaskId,
        reason: String,
    },
    TaskResumed {
        task_id: TaskId,
    },
    TaskCancelled {
        task_id: TaskId,
        reason: String,
    },
    ProgressMessage {
        task_id: TaskId,
        message: String,
        level: MessageLevel,
    },
}

/// Level of progress messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Debug,
}

/// Completed task information for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    pub task_id: TaskId,
    pub title: String,
    pub started_at: SystemTime,
    pub completed_at: SystemTime,
    pub duration: Duration,
    pub final_status: TaskStatus,
    pub progress_percentage: f32,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

/// Progress listener for receiving updates
#[derive(Debug)]
pub struct ProgressListener {
    pub id: ProgressListenerId,
    pub receiver: broadcast::Receiver<ProgressEvent>,
    pub filter: Option<ProgressFilter>,
}

/// Filter for progress events
#[derive(Debug, Clone)]
pub struct ProgressFilter {
    pub task_ids: Option<Vec<TaskId>>,
    pub event_types: Option<Vec<String>>,
    pub message_levels: Option<Vec<MessageLevel>>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(ui_enabled: bool) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            current_tasks: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            ui_enabled,
            max_listeners: 50,
            task_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 1000,
        }
    }

    /// Start tracking a new task
    pub async fn start_task(
        &self,
        title: String,
        description: String,
        substeps: Vec<String>,
        estimated_duration: Option<Duration>,
    ) -> Result<TaskProgressHandle> {
        let task_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        
        let estimated_completion = estimated_duration.map(|duration| now + duration);
        
        let substeps: Vec<SubStep> = substeps
            .into_iter()
            .enumerate()
            .map(|(i, name)| SubStep {
                id: format!("{}_{}", task_id, i),
                name: name.clone(),
                description: name,
                weight: 1.0 / substeps.len() as f32,
                status: SubStepStatus::Pending,
                started_at: None,
                completed_at: None,
                error: None,
            })
            .collect();

        let task_progress = TaskProgress {
            task_id: task_id.clone(),
            title: title.clone(),
            description,
            current_step: "Starting...".to_string(),
            progress_percentage: 0.0,
            started_at: now,
            estimated_completion,
            substeps,
            current_substep_index: 0,
            status: TaskStatus::Running,
            metadata: HashMap::new(),
            cancellation_token: Some(Uuid::new_v4().to_string()),
        };

        // Add to current tasks
        {
            let mut tasks = self.current_tasks.write().await;
            tasks.insert(task_id.clone(), task_progress.clone());
        }

        // Send started event
        let _ = self.event_sender.send(ProgressEvent::TaskStarted {
            task_id: task_id.clone(),
            title,
            estimated_duration,
        });

        Ok(TaskProgressHandle {
            task_id,
            tracker: self.clone(),
        })
    }

    /// Update task progress
    pub async fn update_progress(
        &self,
        task_id: &TaskId,
        progress_percentage: f32,
        current_step: String,
        estimated_completion: Option<SystemTime>,
    ) -> Result<()> {
        let mut tasks = self.current_tasks.write().await;
        
        if let Some(task) = tasks.get_mut(task_id) {
            task.progress_percentage = progress_percentage.clamp(0.0, 100.0);
            task.current_step = current_step.clone();
            if let Some(completion) = estimated_completion {
                task.estimated_completion = Some(completion);
            }

            // Send update event
            let _ = self.event_sender.send(ProgressEvent::TaskUpdated {
                task_id: task_id.clone(),
                progress_percentage: task.progress_percentage,
                current_step,
                estimated_completion: task.estimated_completion,
            });
        }

        Ok(())
    }

    /// Start a substep
    pub async fn start_substep(
        &self,
        task_id: &TaskId,
        substep_index: usize,
    ) -> Result<()> {
        let mut tasks = self.current_tasks.write().await;
        
        if let Some(task) = tasks.get_mut(task_id) {
            if substep_index < task.substeps.len() {
                task.current_substep_index = substep_index;
                task.substeps[substep_index].status = SubStepStatus::Running;
                task.substeps[substep_index].started_at = Some(SystemTime::now());
                
                // Update current step
                task.current_step = task.substeps[substep_index].name.clone();

                // Send substep started event
                let _ = self.event_sender.send(ProgressEvent::SubStepStarted {
                    task_id: task_id.clone(),
                    substep_id: task.substeps[substep_index].id.clone(),
                    substep_name: task.substeps[substep_index].name.clone(),
                });

                // Update overall progress
                self.update_overall_progress(task).await;
            }
        }

        Ok(())
    }

    /// Complete a substep
    pub async fn complete_substep(
        &self,
        task_id: &TaskId,
        substep_index: usize,
        success: bool,
        error: Option<String>,
    ) -> Result<()> {
        let mut tasks = self.current_tasks.write().await;
        
        if let Some(task) = tasks.get_mut(task_id) {
            if substep_index < task.substeps.len() {
                task.substeps[substep_index].status = if success {
                    SubStepStatus::Completed
                } else {
                    SubStepStatus::Failed
                };
                task.substeps[substep_index].completed_at = Some(SystemTime::now());
                task.substeps[substep_index].error = error.clone();

                // Send substep completed event
                let _ = self.event_sender.send(ProgressEvent::SubStepCompleted {
                    task_id: task_id.clone(),
                    substep_id: task.substeps[substep_index].id.clone(),
                    success,
                    error,
                });

                // Update overall progress
                self.update_overall_progress(task).await;
            }
        }

        Ok(())
    }

    /// Complete a task
    pub async fn complete_task(
        &self,
        task_id: &TaskId,
        success: bool,
        result_summary: Option<String>,
    ) -> Result<()> {
        let completed_task = {
            let mut tasks = self.current_tasks.write().await;
            
            if let Some(mut task) = tasks.remove(task_id) {
                let now = SystemTime::now();
                let duration = now.duration_since(task.started_at).unwrap_or_default();
                
                task.status = if success {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Failed
                };
                task.progress_percentage = if success { 100.0 } else { task.progress_percentage };

                // Send completion event
                let _ = self.event_sender.send(ProgressEvent::TaskCompleted {
                    task_id: task_id.clone(),
                    success,
                    duration,
                    result_summary: result_summary.clone(),
                });

                // Create completed task record
                CompletedTask {
                    task_id: task.task_id.clone(),
                    title: task.title.clone(),
                    started_at: task.started_at,
                    completed_at: now,
                    duration,
                    final_status: task.status.clone(),
                    progress_percentage: task.progress_percentage,
                    result_summary,
                    error_message: if success { None } else { Some("Task failed".to_string()) },
                }
            } else {
                return Ok(()); // Task not found
            }
        };

        // Add to history
        {
            let mut history = self.task_history.write().await;
            history.push(completed_task);

            // Trim history if needed
            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }

        Ok(())
    }

    /// Send a progress message
    pub async fn send_message(
        &self,
        task_id: &TaskId,
        message: String,
        level: MessageLevel,
    ) -> Result<()> {
        let _ = self.event_sender.send(ProgressEvent::ProgressMessage {
            task_id: task_id.clone(),
            message,
            level,
        });
        
        Ok(())
    }

    /// Pause a task
    pub async fn pause_task(&self, task_id: &TaskId, reason: String) -> Result<()> {
        let mut tasks = self.current_tasks.write().await;
        
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = TaskStatus::Paused;
            
            let _ = self.event_sender.send(ProgressEvent::TaskPaused {
                task_id: task_id.clone(),
                reason,
            });
        }

        Ok(())
    }

    /// Resume a paused task
    pub async fn resume_task(&self, task_id: &TaskId) -> Result<()> {
        let mut tasks = self.current_tasks.write().await;
        
        if let Some(task) = tasks.get_mut(task_id) {
            if task.status == TaskStatus::Paused {
                task.status = TaskStatus::Running;
                
                let _ = self.event_sender.send(ProgressEvent::TaskResumed {
                    task_id: task_id.clone(),
                });
            }
        }

        Ok(())
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &TaskId, reason: String) -> Result<()> {
        let cancelled_task = {
            let mut tasks = self.current_tasks.write().await;
            
            if let Some(mut task) = tasks.remove(task_id) {
                task.status = TaskStatus::Cancelled;
                
                let _ = self.event_sender.send(ProgressEvent::TaskCancelled {
                    task_id: task_id.clone(),
                    reason: reason.clone(),
                });

                // Create completed task record
                let now = SystemTime::now();
                let duration = now.duration_since(task.started_at).unwrap_or_default();
                
                CompletedTask {
                    task_id: task.task_id.clone(),
                    title: task.title.clone(),
                    started_at: task.started_at,
                    completed_at: now,
                    duration,
                    final_status: TaskStatus::Cancelled,
                    progress_percentage: task.progress_percentage,
                    result_summary: Some(format!("Cancelled: {}", reason)),
                    error_message: None,
                }
            } else {
                return Ok(()); // Task not found
            }
        };

        // Add to history
        {
            let mut history = self.task_history.write().await;
            history.push(cancelled_task);

            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }

        Ok(())
    }

    /// Get current task status
    pub async fn get_task_status(&self, task_id: &TaskId) -> Option<TaskProgress> {
        let tasks = self.current_tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// Get all current tasks
    pub async fn get_all_current_tasks(&self) -> Vec<TaskProgress> {
        let tasks = self.current_tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Get task history
    pub async fn get_task_history(&self, limit: Option<usize>) -> Vec<CompletedTask> {
        let history = self.task_history.read().await;
        let start_index = if let Some(limit) = limit {
            history.len().saturating_sub(limit)
        } else {
            0
        };
        
        history[start_index..].to_vec()
    }

    /// Create a progress listener
    pub fn create_listener(&self, filter: Option<ProgressFilter>) -> ProgressListener {
        let receiver = self.event_sender.subscribe();
        let id = Uuid::new_v4().to_string();
        
        ProgressListener {
            id,
            receiver,
            filter,
        }
    }

    /// Update overall progress based on substep completion
    async fn update_overall_progress(&self, task: &mut TaskProgress) {
        let completed_weight: f32 = task
            .substeps
            .iter()
            .filter(|step| step.status == SubStepStatus::Completed)
            .map(|step| step.weight)
            .sum();

        let running_weight: f32 = task
            .substeps
            .iter()
            .filter(|step| step.status == SubStepStatus::Running)
            .map(|step| step.weight * 0.5) // Assume running steps are 50% complete
            .sum();

        task.progress_percentage = ((completed_weight + running_weight) * 100.0).clamp(0.0, 100.0);
    }

    /// Get statistics about progress tracking
    pub async fn get_statistics(&self) -> ProgressStatistics {
        let current_tasks = self.current_tasks.read().await;
        let history = self.task_history.read().await;

        let active_task_count = current_tasks.len();
        let total_completed_tasks = history.len();
        
        let successful_tasks = history
            .iter()
            .filter(|task| task.final_status == TaskStatus::Completed)
            .count();

        let failed_tasks = history
            .iter()
            .filter(|task| task.final_status == TaskStatus::Failed)
            .count();

        let cancelled_tasks = history
            .iter()
            .filter(|task| task.final_status == TaskStatus::Cancelled)
            .count();

        let average_duration = if !history.is_empty() {
            let total_duration: Duration = history.iter().map(|task| task.duration).sum();
            total_duration / history.len() as u32
        } else {
            Duration::from_secs(0)
        };

        ProgressStatistics {
            active_task_count,
            total_completed_tasks,
            successful_tasks,
            failed_tasks,
            cancelled_tasks,
            success_rate: if total_completed_tasks > 0 {
                successful_tasks as f64 / total_completed_tasks as f64
            } else {
                0.0
            },
            average_duration,
        }
    }
}

impl Clone for ProgressTracker {
    fn clone(&self) -> Self {
        Self {
            current_tasks: Arc::clone(&self.current_tasks),
            event_sender: self.event_sender.clone(),
            ui_enabled: self.ui_enabled,
            max_listeners: self.max_listeners,
            task_history: Arc::clone(&self.task_history),
            max_history_size: self.max_history_size,
        }
    }
}

/// Handle for managing a specific task's progress
#[derive(Debug)]
pub struct TaskProgressHandle {
    task_id: TaskId,
    tracker: ProgressTracker,
}

impl TaskProgressHandle {
    /// Update progress percentage
    pub async fn update_progress(&self, percentage: f32, step: &str) -> Result<()> {
        self.tracker
            .update_progress(&self.task_id, percentage, step.to_string(), None)
            .await
    }

    /// Start a substep by index
    pub async fn start_substep(&self, index: usize) -> Result<()> {
        self.tracker.start_substep(&self.task_id, index).await
    }

    /// Complete a substep
    pub async fn complete_substep(&self, index: usize, success: bool, error: Option<String>) -> Result<()> {
        self.tracker.complete_substep(&self.task_id, index, success, error).await
    }

    /// Send a progress message
    pub async fn send_message(&self, message: &str, level: MessageLevel) -> Result<()> {
        self.tracker.send_message(&self.task_id, message.to_string(), level).await
    }

    /// Complete the task
    pub async fn complete(&self, success: bool, summary: Option<String>) -> Result<()> {
        self.tracker.complete_task(&self.task_id, success, summary).await
    }

    /// Get task ID
    pub fn task_id(&self) -> &TaskId {
        &self.task_id
    }
}

/// Statistics about progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStatistics {
    pub active_task_count: usize,
    pub total_completed_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub cancelled_tasks: usize,
    pub success_rate: f64,
    pub average_duration: Duration,
}

impl ProgressListener {
    /// Receive the next progress event
    pub async fn recv(&mut self) -> Result<ProgressEvent, broadcast::error::RecvError> {
        loop {
            let event = self.receiver.recv().await?;
            
            // Apply filter if present
            if let Some(filter) = &self.filter {
                if self.event_matches_filter(&event, filter) {
                    return Ok(event);
                }
                // Continue to next event if filtered out
            } else {
                return Ok(event);
            }
        }
    }

    /// Check if event matches filter criteria
    fn event_matches_filter(&self, event: &ProgressEvent, filter: &ProgressFilter) -> bool {
        // Check task ID filter
        if let Some(task_ids) = &filter.task_ids {
            let event_task_id = match event {
                ProgressEvent::TaskStarted { task_id, .. } |
                ProgressEvent::TaskUpdated { task_id, .. } |
                ProgressEvent::SubStepStarted { task_id, .. } |
                ProgressEvent::SubStepCompleted { task_id, .. } |
                ProgressEvent::TaskCompleted { task_id, .. } |
                ProgressEvent::TaskPaused { task_id, .. } |
                ProgressEvent::TaskResumed { task_id } |
                ProgressEvent::TaskCancelled { task_id, .. } |
                ProgressEvent::ProgressMessage { task_id, .. } => task_id,
            };
            
            if !task_ids.contains(event_task_id) {
                return false;
            }
        }

        // Check message level filter
        if let Some(levels) = &filter.message_levels {
            if let ProgressEvent::ProgressMessage { level, .. } = event {
                if !levels.iter().any(|l| std::mem::discriminant(l) == std::mem::discriminant(level)) {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration as TokioDuration};

    #[tokio::test]
    async fn test_basic_progress_tracking() {
        let tracker = ProgressTracker::new(true);
        
        let handle = tracker
            .start_task(
                "Test Task".to_string(),
                "Testing progress tracking".to_string(),
                vec!["Step 1".to_string(), "Step 2".to_string()],
                Some(Duration::from_secs(10)),
            )
            .await
            .unwrap();

        handle.update_progress(50.0, "Halfway done").await.unwrap();
        
        let status = tracker.get_task_status(handle.task_id()).await.unwrap();
        assert_eq!(status.progress_percentage, 50.0);
        assert_eq!(status.current_step, "Halfway done");

        handle.complete(true, Some("Success!".to_string())).await.unwrap();
        
        // Task should be removed from current tasks
        assert!(tracker.get_task_status(handle.task_id()).await.is_none());
        
        // Should be in history
        let history = tracker.get_task_history(Some(10)).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].final_status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_substep_progress() {
        let tracker = ProgressTracker::new(true);
        
        let handle = tracker
            .start_task(
                "Substep Task".to_string(),
                "Testing substep progress".to_string(),
                vec!["First".to_string(), "Second".to_string(), "Third".to_string()],
                None,
            )
            .await
            .unwrap();

        handle.start_substep(0).await.unwrap();
        handle.complete_substep(0, true, None).await.unwrap();
        
        handle.start_substep(1).await.unwrap();
        handle.complete_substep(1, true, None).await.unwrap();
        
        let status = tracker.get_task_status(handle.task_id()).await.unwrap();
        assert!(status.progress_percentage > 60.0); // Should be around 66%
        
        handle.complete(true, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_progress_events() {
        let tracker = ProgressTracker::new(true);
        let mut listener = tracker.create_listener(None);
        
        let handle = tracker
            .start_task(
                "Event Task".to_string(),
                "Testing events".to_string(),
                vec!["Step".to_string()],
                None,
            )
            .await
            .unwrap();

        // Should receive task started event
        let event = timeout(TokioDuration::from_millis(100), listener.recv()).await;
        assert!(event.is_ok());
        if let Ok(Ok(ProgressEvent::TaskStarted { task_id, title, .. })) = event {
            assert_eq!(task_id, *handle.task_id());
            assert_eq!(title, "Event Task");
        }

        handle.update_progress(25.0, "Quarter done").await.unwrap();
        
        // Should receive update event
        let event = timeout(TokioDuration::from_millis(100), listener.recv()).await;
        assert!(event.is_ok());

        handle.complete(true, None).await.unwrap();
        
        // Should receive completion event
        let event = timeout(TokioDuration::from_millis(100), listener.recv()).await;
        assert!(event.is_ok());
        if let Ok(Ok(ProgressEvent::TaskCompleted { success, .. })) = event {
            assert!(success);
        }
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let tracker = ProgressTracker::new(true);
        
        let handle = tracker
            .start_task(
                "Cancel Task".to_string(),
                "Testing cancellation".to_string(),
                vec!["Step".to_string()],
                None,
            )
            .await
            .unwrap();

        tracker.cancel_task(handle.task_id(), "User requested".to_string()).await.unwrap();
        
        // Task should be removed from current tasks
        assert!(tracker.get_task_status(handle.task_id()).await.is_none());
        
        // Should be in history as cancelled
        let history = tracker.get_task_history(Some(10)).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].final_status, TaskStatus::Cancelled);
    }
}