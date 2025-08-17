use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::logger::{log_debug, log_info, log_warn};

/// Advanced workflow state management system
/// Provides sophisticated workflow persistence, recovery, and coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStateConfig {
    /// Enable persistent workflow storage
    pub enable_persistence: bool,
    /// Workflow state storage directory
    pub state_directory: PathBuf,
    /// Auto-save interval (seconds)
    pub auto_save_interval: u64,
    /// Maximum workflow history to keep
    pub max_workflow_history: usize,
    /// Enable workflow checkpointing
    pub enable_checkpointing: bool,
    /// Checkpoint interval (operations)
    pub checkpoint_interval: usize,
    /// Enable state compression
    pub enable_compression: bool,
}

impl Default for WorkflowStateConfig {
    fn default() -> Self {
        Self {
            enable_persistence: true,
            state_directory: PathBuf::from("./workflow_states"),
            auto_save_interval: 30,
            max_workflow_history: 100,
            enable_checkpointing: true,
            checkpoint_interval: 10,
            enable_compression: false,
        }
    }
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowState {
    /// Workflow is being initialized
    Initializing,
    /// Workflow is actively running
    Running,
    /// Workflow is paused (can be resumed)
    Paused,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed with error
    Failed { error: String },
    /// Workflow was cancelled
    Cancelled,
    /// Workflow is waiting for external input
    WaitingForInput { prompt: String },
    /// Workflow is in recovery mode
    Recovering { attempt: usize },
}

/// Workflow checkpoint for state recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCheckpoint {
    pub id: String,
    pub workflow_id: String,
    pub timestamp: SystemTime,
    pub state: WorkflowState,
    pub completed_steps: Vec<String>,
    pub current_step: Option<String>,
    pub next_steps: Vec<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub execution_log: Vec<WorkflowLogEntry>,
    pub metrics: WorkflowMetrics,
}

/// Workflow log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub message: String,
    pub step_id: Option<String>,
    pub duration: Option<Duration>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Workflow execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub total_steps: usize,
    pub completed_steps: usize,
    pub failed_steps: usize,
    pub total_execution_time: Duration,
    pub average_step_time: Duration,
    pub success_rate: f64,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_peak_mb: f64,
    pub cpu_time_seconds: f64,
    pub network_requests: usize,
    pub file_operations: usize,
}

/// Advanced workflow state manager
pub struct AdvancedWorkflowStateManager {
    config: WorkflowStateConfig,
    active_workflows: Arc<Mutex<HashMap<String, WorkflowCheckpoint>>>,
    workflow_history: Arc<Mutex<VecDeque<WorkflowCheckpoint>>>,
    auto_save_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl AdvancedWorkflowStateManager {
    pub async fn new(config: WorkflowStateConfig) -> Result<Self> {
        // Ensure state directory exists
        if config.enable_persistence {
            fs::create_dir_all(&config.state_directory).await?;
        }

        let manager = Self {
            config,
            active_workflows: Arc::new(Mutex::new(HashMap::new())),
            workflow_history: Arc::new(Mutex::new(VecDeque::new())),
            auto_save_handle: Arc::new(Mutex::new(None)),
        };

        // Load existing workflows if persistence is enabled
        if manager.config.enable_persistence {
            manager.load_workflows().await?;
        }

        // Start auto-save task
        manager.start_auto_save_task().await;

        Ok(manager)
    }

    pub fn default() -> Result<Self> {
        tokio::runtime::Handle::current().block_on(Self::new(WorkflowStateConfig::default()))
    }

    /// Create a new workflow checkpoint
    pub async fn create_workflow(&self, workflow_id: String, initial_context: HashMap<String, serde_json::Value>) -> Result<String> {
        let checkpoint_id = Uuid::new_v4().to_string();
        let checkpoint = WorkflowCheckpoint {
            id: checkpoint_id.clone(),
            workflow_id: workflow_id.clone(),
            timestamp: SystemTime::now(),
            state: WorkflowState::Initializing,
            completed_steps: Vec::new(),
            current_step: None,
            next_steps: Vec::new(),
            context: initial_context,
            execution_log: Vec::new(),
            metrics: WorkflowMetrics {
                total_steps: 0,
                completed_steps: 0,
                failed_steps: 0,
                total_execution_time: Duration::from_secs(0),
                average_step_time: Duration::from_secs(0),
                success_rate: 0.0,
                resource_usage: ResourceUsage {
                    memory_peak_mb: 0.0,
                    cpu_time_seconds: 0.0,
                    network_requests: 0,
                    file_operations: 0,
                },
            },
        };

        let mut workflows = self.active_workflows.lock().await;
        workflows.insert(workflow_id.clone(), checkpoint);

        log_info!("workflow_state", "📝 Created workflow checkpoint: {} for workflow: {}", checkpoint_id, workflow_id);
        Ok(checkpoint_id)
    }

    /// Update workflow state
    pub async fn update_workflow_state(&self, workflow_id: &str, new_state: WorkflowState) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            checkpoint.state = new_state;
            checkpoint.timestamp = SystemTime::now();
            
            self.log_workflow_event(
                workflow_id,
                LogLevel::Info,
                format!("Workflow state updated to: {:?}", checkpoint.state),
                None,
            ).await;

            log_debug!("workflow_state", "🔄 Updated workflow state: {} -> {:?}", workflow_id, checkpoint.state);
        }
        Ok(())
    }

    /// Add a step to the workflow
    pub async fn add_workflow_step(&self, workflow_id: &str, step_id: String, step_description: String) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            checkpoint.next_steps.push(step_id.clone());
            checkpoint.metrics.total_steps += 1;
            
            self.log_workflow_event(
                workflow_id,
                LogLevel::Info,
                format!("Added step: {} - {}", step_id, step_description),
                Some(step_id.clone()),
            ).await;

            log_debug!("workflow_state", "➕ Added step to workflow {}: {}", workflow_id, step_id);
        }
        Ok(())
    }

    /// Mark a step as completed
    pub async fn complete_workflow_step(&self, workflow_id: &str, step_id: &str, execution_time: Duration) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            // Move step from next_steps to completed_steps
            if let Some(pos) = checkpoint.next_steps.iter().position(|x| x == step_id) {
                checkpoint.next_steps.remove(pos);
            }
            checkpoint.completed_steps.push(step_id.to_string());
            checkpoint.current_step = None;
            
            // Update metrics
            checkpoint.metrics.completed_steps += 1;
            checkpoint.metrics.total_execution_time += execution_time;
            if checkpoint.metrics.completed_steps > 0 {
                checkpoint.metrics.average_step_time = checkpoint.metrics.total_execution_time / checkpoint.metrics.completed_steps as u32;
                checkpoint.metrics.success_rate = (checkpoint.metrics.completed_steps as f64) / (checkpoint.metrics.total_steps as f64) * 100.0;
            }

            self.log_workflow_event(
                workflow_id,
                LogLevel::Info,
                format!("Completed step: {} in {:?}", step_id, execution_time),
                Some(step_id.to_string()),
            ).await;

            // Create checkpoint if enabled
            if self.config.enable_checkpointing && checkpoint.metrics.completed_steps % self.config.checkpoint_interval == 0 {
                self.create_checkpoint(workflow_id).await?;
            }

            log_info!("workflow_state", "✅ Completed step {} in workflow {} ({:?})", step_id, workflow_id, execution_time);
        }
        Ok(())
    }

    /// Mark a step as failed
    pub async fn fail_workflow_step(&self, workflow_id: &str, step_id: &str, error: &str, execution_time: Duration) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            // Remove from next_steps
            if let Some(pos) = checkpoint.next_steps.iter().position(|x| x == step_id) {
                checkpoint.next_steps.remove(pos);
            }
            checkpoint.current_step = None;
            
            // Update metrics
            checkpoint.metrics.failed_steps += 1;
            checkpoint.metrics.total_execution_time += execution_time;
            if checkpoint.metrics.completed_steps + checkpoint.metrics.failed_steps > 0 {
                checkpoint.metrics.success_rate = (checkpoint.metrics.completed_steps as f64) / ((checkpoint.metrics.completed_steps + checkpoint.metrics.failed_steps) as f64) * 100.0;
            }

            self.log_workflow_event(
                workflow_id,
                LogLevel::Error,
                format!("Failed step: {} - Error: {} (Duration: {:?})", step_id, error, execution_time),
                Some(step_id.to_string()),
            ).await;

            log_warn!("workflow_state", "❌ Failed step {} in workflow {}: {} ({:?})", step_id, workflow_id, error, execution_time);
        }
        Ok(())
    }

    /// Set current step being executed
    pub async fn set_current_step(&self, workflow_id: &str, step_id: String) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            checkpoint.current_step = Some(step_id.clone());
            
            self.log_workflow_event(
                workflow_id,
                LogLevel::Info,
                format!("Started executing step: {}", step_id),
                Some(step_id.clone()),
            ).await;

            log_debug!("workflow_state", "▶️ Set current step for workflow {}: {}", workflow_id, step_id);
        }
        Ok(())
    }

    /// Update workflow context
    pub async fn update_workflow_context(&self, workflow_id: &str, key: String, value: serde_json::Value) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            checkpoint.context.insert(key.clone(), value);
            
            log_debug!("workflow_state", "📝 Updated context for workflow {}: {}", workflow_id, key);
        }
        Ok(())
    }

    /// Get workflow context
    pub async fn get_workflow_context(&self, workflow_id: &str) -> Option<HashMap<String, serde_json::Value>> {
        let workflows = self.active_workflows.lock().await;
        workflows.get(workflow_id).map(|checkpoint| checkpoint.context.clone())
    }

    /// Get workflow state
    pub async fn get_workflow_state(&self, workflow_id: &str) -> Option<WorkflowState> {
        let workflows = self.active_workflows.lock().await;
        workflows.get(workflow_id).map(|checkpoint| checkpoint.state.clone())
    }

    /// Get workflow metrics
    pub async fn get_workflow_metrics(&self, workflow_id: &str) -> Option<WorkflowMetrics> {
        let workflows = self.active_workflows.lock().await;
        workflows.get(workflow_id).map(|checkpoint| checkpoint.metrics.clone())
    }

    /// Create a checkpoint (manual or automatic)
    pub async fn create_checkpoint(&self, workflow_id: &str) -> Result<String> {
        let checkpoint_id = Uuid::new_v4().to_string();
        
        if let Some(checkpoint) = {
            let workflows = self.active_workflows.lock().await;
            workflows.get(workflow_id).cloned()
        } {
            // Save checkpoint to history
            let mut history = self.workflow_history.lock().await;
            let mut checkpoint_copy = checkpoint.clone();
            checkpoint_copy.id = checkpoint_id.clone();
            history.push_back(checkpoint_copy);
            
            // Limit history size
            while history.len() > self.config.max_workflow_history {
                history.pop_front();
            }

            // Persist if enabled
            if self.config.enable_persistence {
                self.save_checkpoint(&checkpoint).await?;
            }

            log_info!("workflow_state", "💾 Created checkpoint {} for workflow {}", checkpoint_id, workflow_id);
        }

        Ok(checkpoint_id)
    }

    /// Restore workflow from checkpoint
    pub async fn restore_workflow(&self, workflow_id: &str, checkpoint_id: &str) -> Result<bool> {
        let history = self.workflow_history.lock().await;
        
        if let Some(checkpoint) = history.iter().find(|c| c.id == checkpoint_id && c.workflow_id == workflow_id) {
            let mut workflows = self.active_workflows.lock().await;
            workflows.insert(workflow_id.to_string(), checkpoint.clone());
            
            log_info!("workflow_state", "🔄 Restored workflow {} from checkpoint {}", workflow_id, checkpoint_id);
            return Ok(true);
        }

        // Try loading from disk if not in memory
        if self.config.enable_persistence {
            if let Ok(checkpoint) = self.load_checkpoint(checkpoint_id).await {
                if checkpoint.workflow_id == workflow_id {
                    let mut workflows = self.active_workflows.lock().await;
                    workflows.insert(workflow_id.to_string(), checkpoint);
                    
                    log_info!("workflow_state", "💿 Restored workflow {} from disk checkpoint {}", workflow_id, checkpoint_id);
                    return Ok(true);
                }
            }
        }

        log_warn!("workflow_state", "❌ Could not restore workflow {} from checkpoint {}", workflow_id, checkpoint_id);
        Ok(false)
    }

    /// List all active workflows
    pub async fn list_active_workflows(&self) -> Vec<String> {
        let workflows = self.active_workflows.lock().await;
        workflows.keys().cloned().collect()
    }

    /// Get workflow progress
    pub async fn get_workflow_progress(&self, workflow_id: &str) -> Option<f64> {
        let workflows = self.active_workflows.lock().await;
        workflows.get(workflow_id).map(|checkpoint| {
            if checkpoint.metrics.total_steps > 0 {
                (checkpoint.metrics.completed_steps as f64) / (checkpoint.metrics.total_steps as f64) * 100.0
            } else {
                0.0
            }
        })
    }

    /// Remove completed or failed workflows
    pub async fn cleanup_workflows(&self) -> Result<usize> {
        let mut workflows = self.active_workflows.lock().await;
        let initial_count = workflows.len();
        
        workflows.retain(|_, checkpoint| {
            !matches!(checkpoint.state, WorkflowState::Completed | WorkflowState::Failed { .. } | WorkflowState::Cancelled)
        });
        
        let removed_count = initial_count - workflows.len();
        log_info!("workflow_state", "🧹 Cleaned up {} completed/failed workflows", removed_count);
        Ok(removed_count)
    }

    /// Log workflow event
    async fn log_workflow_event(&self, workflow_id: &str, level: LogLevel, message: String, step_id: Option<String>) {
        let mut workflows = self.active_workflows.lock().await;
        if let Some(checkpoint) = workflows.get_mut(workflow_id) {
            let entry = WorkflowLogEntry {
                timestamp: SystemTime::now(),
                level,
                message,
                step_id,
                duration: None,
                metadata: HashMap::new(),
            };
            checkpoint.execution_log.push(entry);
            
            // Limit log size
            if checkpoint.execution_log.len() > 1000 {
                checkpoint.execution_log.remove(0);
            }
        }
    }

    /// Start auto-save task
    async fn start_auto_save_task(&self) {
        if !self.config.enable_persistence || self.config.auto_save_interval == 0 {
            return;
        }

        let workflows = self.active_workflows.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.auto_save_interval));
            
            loop {
                interval.tick().await;
                
                let workflows_clone = {
                    let workflows_guard = workflows.lock().await;
                    workflows_guard.clone()
                };
                
                for (workflow_id, checkpoint) in workflows_clone {
                    if let Err(e) = Self::save_checkpoint_static(&checkpoint, &config).await {
                        log_warn!("workflow_state", "⚠️ Failed to auto-save workflow {}: {}", workflow_id, e);
                    }
                }
            }
        });

        let mut auto_save_handle = self.auto_save_handle.lock().await;
        *auto_save_handle = Some(handle);
    }

    /// Save checkpoint to disk
    async fn save_checkpoint(&self, checkpoint: &WorkflowCheckpoint) -> Result<()> {
        Self::save_checkpoint_static(checkpoint, &self.config).await
    }

    /// Static method for saving checkpoint
    async fn save_checkpoint_static(checkpoint: &WorkflowCheckpoint, config: &WorkflowStateConfig) -> Result<()> {
        let file_path = config.state_directory.join(format!("{}.json", checkpoint.id));
        let serialized = serde_json::to_string_pretty(checkpoint)?;
        
        if config.enable_compression {
            // In a real implementation, would compress the data
            fs::write(file_path, serialized).await?;
        } else {
            fs::write(file_path, serialized).await?;
        }
        
        Ok(())
    }

    /// Load checkpoint from disk
    async fn load_checkpoint(&self, checkpoint_id: &str) -> Result<WorkflowCheckpoint> {
        let file_path = self.config.state_directory.join(format!("{}.json", checkpoint_id));
        let content = fs::read_to_string(file_path).await?;
        let checkpoint: WorkflowCheckpoint = serde_json::from_str(&content)?;
        Ok(checkpoint)
    }

    /// Load all workflows from disk
    async fn load_workflows(&self) -> Result<()> {
        if !self.config.state_directory.exists() {
            return Ok(());
        }

        let mut dir = fs::read_dir(&self.config.state_directory).await?;
        let mut loaded_count = 0;
        
        while let Some(entry) = dir.next_entry().await? {
            if let Some(extension) = entry.path().extension() {
                if extension == "json" {
                    if let Ok(checkpoint) = self.load_checkpoint(
                        &entry.path().file_stem().unwrap().to_string_lossy()
                    ).await {
                        let mut workflows = self.active_workflows.lock().await;
                        workflows.insert(checkpoint.workflow_id.clone(), checkpoint);
                        loaded_count += 1;
                    }
                }
            }
        }

        log_info!("workflow_state", "📂 Loaded {} workflows from disk", loaded_count);
        Ok(())
    }

    /// Get comprehensive statistics
    pub async fn get_statistics(&self) -> WorkflowStatistics {
        let workflows = self.active_workflows.lock().await;
        let history = self.workflow_history.lock().await;
        
        let active_count = workflows.len();
        let total_count = active_count + history.len();
        
        let states: HashMap<String, usize> = workflows.values()
            .map(|w| format!("{:?}", w.state))
            .fold(HashMap::new(), |mut acc, state| {
                *acc.entry(state).or_insert(0) += 1;
                acc
            });

        let avg_completion_rate = if !workflows.is_empty() {
            workflows.values()
                .map(|w| w.metrics.success_rate)
                .sum::<f64>() / workflows.len() as f64
        } else {
            0.0
        };

        WorkflowStatistics {
            active_workflows: active_count,
            total_workflows: total_count,
            workflow_states: states,
            average_completion_rate: avg_completion_rate,
            checkpoints_created: history.len(),
        }
    }
}

/// Workflow statistics
#[derive(Debug, Clone)]
pub struct WorkflowStatistics {
    pub active_workflows: usize,
    pub total_workflows: usize,
    pub workflow_states: HashMap<String, usize>,
    pub average_completion_rate: f64,
    pub checkpoints_created: usize,
}

/// Global workflow state manager
use once_cell::sync::Lazy;

static GLOBAL_WORKFLOW_STATE_MANAGER: Lazy<Mutex<Option<AdvancedWorkflowStateManager>>> = 
    Lazy::new(|| Mutex::new(None));

/// Initialize global workflow state manager
pub async fn initialize_workflow_state_manager(config: WorkflowStateConfig) -> Result<()> {
    let manager = AdvancedWorkflowStateManager::new(config).await?;
    let mut global = GLOBAL_WORKFLOW_STATE_MANAGER.lock().await;
    *global = Some(manager);
    log_info!("workflow_state", "🗃️ Advanced workflow state management initialized");
    Ok(())
}

/// Create workflow using global manager
pub async fn create_global_workflow(workflow_id: String, initial_context: HashMap<String, serde_json::Value>) -> Result<String> {
    let manager_guard = GLOBAL_WORKFLOW_STATE_MANAGER.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        manager.create_workflow(workflow_id, initial_context).await
    } else {
        log_warn!("workflow_state", "⚠️ Workflow state manager not initialized");
        Ok(Uuid::new_v4().to_string())
    }
}

/// Update workflow state using global manager
pub async fn update_global_workflow_state(workflow_id: &str, new_state: WorkflowState) -> Result<()> {
    let manager_guard = GLOBAL_WORKFLOW_STATE_MANAGER.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        manager.update_workflow_state(workflow_id, new_state).await
    } else {
        Ok(())
    }
}

/// Get global workflow statistics
pub async fn get_global_workflow_statistics() -> Option<WorkflowStatistics> {
    let manager_guard = GLOBAL_WORKFLOW_STATE_MANAGER.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        Some(manager.get_statistics().await)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_workflow_state_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = WorkflowStateConfig {
            enable_persistence: true,
            state_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = AdvancedWorkflowStateManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let config = WorkflowStateConfig {
            enable_persistence: false, // Disable for test speed
            state_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = AdvancedWorkflowStateManager::new(config).await.unwrap();
        
        // Create workflow
        let workflow_id = "test_workflow".to_string();
        let checkpoint_id = manager.create_workflow(workflow_id.clone(), HashMap::new()).await.unwrap();
        assert!(!checkpoint_id.is_empty());

        // Add and complete steps
        manager.add_workflow_step(&workflow_id, "step1".to_string(), "Test step".to_string()).await.unwrap();
        manager.set_current_step(&workflow_id, "step1".to_string()).await.unwrap();
        manager.complete_workflow_step(&workflow_id, "step1", Duration::from_millis(100)).await.unwrap();

        // Check progress
        let progress = manager.get_workflow_progress(&workflow_id).await;
        assert!(progress.is_some());
        assert!(progress.unwrap() > 0.0);

        // Update state
        manager.update_workflow_state(&workflow_id, WorkflowState::Completed).await.unwrap();
        let state = manager.get_workflow_state(&workflow_id).await;
        assert!(matches!(state, Some(WorkflowState::Completed)));
    }

    #[tokio::test]
    async fn test_checkpoint_creation_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let config = WorkflowStateConfig {
            enable_persistence: false,
            state_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = AdvancedWorkflowStateManager::new(config).await.unwrap();
        
        let workflow_id = "checkpoint_test".to_string();
        manager.create_workflow(workflow_id.clone(), HashMap::new()).await.unwrap();
        
        // Add some steps and create checkpoint
        manager.add_workflow_step(&workflow_id, "step1".to_string(), "Step 1".to_string()).await.unwrap();
        manager.add_workflow_step(&workflow_id, "step2".to_string(), "Step 2".to_string()).await.unwrap();
        
        let checkpoint_id = manager.create_checkpoint(&workflow_id).await.unwrap();
        assert!(!checkpoint_id.is_empty());

        // Modify workflow
        manager.complete_workflow_step(&workflow_id, "step1", Duration::from_millis(50)).await.unwrap();
        
        // Restore from checkpoint
        let restored = manager.restore_workflow(&workflow_id, &checkpoint_id).await.unwrap();
        assert!(restored);
    }
}