use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::logger::{log_debug, log_info, log_warn};
use crate::session_manager::SessionManager;
use crate::project_state_manager::get_project_state_manager;
use crate::continuous_executor::WorkflowStatus;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContinuityState {
    /// Active workflow sessions
    pub active_workflows: HashMap<String, WorkflowSession>,
    /// Suspended workflows that can be resumed
    pub suspended_workflows: HashMap<String, SuspendedWorkflow>,
    /// Workflow execution history for learning
    pub execution_history: Vec<WorkflowExecution>,
    /// Global workflow settings
    pub settings: WorkflowContinuitySettings,
    /// Last checkpoint timestamp
    pub last_checkpoint: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSession {
    pub workflow_id: String,
    pub session_id: String,
    pub project_context: Option<String>,
    pub current_goal_id: Option<String>,
    pub task_queue: Vec<String>,
    pub completed_tasks: Vec<String>,
    pub context_variables: HashMap<String, String>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: WorkflowSessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowSessionStatus {
    Active,
    Paused,
    WaitingForInput,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendedWorkflow {
    pub workflow_id: String,
    pub suspension_reason: SuspensionReason,
    pub saved_state: WorkflowState,
    pub suspension_time: DateTime<Utc>,
    pub auto_resume_at: Option<DateTime<Utc>>,
    pub context_snapshot: ContextSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuspensionReason {
    UserRequested,
    SystemError,
    DependencyFailure,
    ResourceConstraints,
    ScheduledPause,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub current_goal: String,
    pub sub_goals: Vec<String>,
    pub execution_context: HashMap<String, String>,
    pub progress_markers: Vec<ProgressMarker>,
    pub error_recovery_state: Option<ErrorRecoveryState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMarker {
    pub checkpoint_id: String,
    pub timestamp: DateTime<Utc>,
    pub completion_percentage: f64,
    pub state_snapshot: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryState {
    pub last_error: String,
    pub recovery_attempts: u32,
    pub recovery_strategies: Vec<String>,
    pub next_recovery_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub file_states: HashMap<PathBuf, FileState>,
    pub environment_vars: HashMap<String, String>,
    pub active_tools: Vec<String>,
    pub project_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub last_modified: DateTime<Utc>,
    pub checksum: String,
    pub size: u64,
    pub permissions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub execution_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub execution_metrics: ExecutionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub total_execution_time_ms: u64,
    pub task_success_rate: f64,
    pub average_task_time_ms: f64,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_peak_mb: u64,
    pub disk_operations: u64,
    pub network_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContinuitySettings {
    /// Automatically checkpoint workflows every N tasks
    pub auto_checkpoint_interval: u32,
    /// Maximum time to keep suspended workflows (hours)
    pub max_suspension_hours: u32,
    /// Enable automatic error recovery
    pub enable_auto_recovery: bool,
    /// Maximum number of recovery attempts
    pub max_recovery_attempts: u32,
    /// Enable workflow resumption across sessions
    pub enable_cross_session_resume: bool,
}

impl Default for WorkflowContinuitySettings {
    fn default() -> Self {
        Self {
            auto_checkpoint_interval: 5,
            max_suspension_hours: 24,
            enable_auto_recovery: true,
            max_recovery_attempts: 3,
            enable_cross_session_resume: true,
        }
    }
}

impl Default for WorkflowContinuityState {
    fn default() -> Self {
        Self {
            active_workflows: HashMap::new(),
            suspended_workflows: HashMap::new(),
            execution_history: Vec::new(),
            settings: WorkflowContinuitySettings::default(),
            last_checkpoint: Utc::now(),
        }
    }
}

pub struct WorkflowContinuityManager {
    state: WorkflowContinuityState,
    session_manager: SessionManager,
    state_file_path: PathBuf,
}

impl WorkflowContinuityManager {
    pub fn new() -> Result<Self> {
        let state_file_path = Self::get_continuity_state_path();
        let session_manager = SessionManager::new()?;
        
        Ok(Self {
            state: WorkflowContinuityState::default(),
            session_manager,
            state_file_path,
        })
    }

    fn get_continuity_state_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cai")
            .join("workflow_continuity.json")
    }

    /// Load continuity state from disk
    pub async fn load_state(&mut self) -> Result<()> {
        if !self.state_file_path.exists() {
            log_info!("workflow_continuity", "📄 No existing continuity state found, starting fresh");
            return Ok(());
        }

        let content = fs::read_to_string(&self.state_file_path).await
            .context("Failed to read workflow continuity state file")?;
        
        self.state = serde_json::from_str(&content)
            .context("Failed to parse workflow continuity state")?;
        
        log_info!("workflow_continuity", "📂 Loaded workflow continuity state with {} active workflows", 
                 self.state.active_workflows.len());
        
        Ok(())
    }

    /// Save continuity state to disk
    pub async fn save_state(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.state_file_path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create continuity state directory")?;
        }

        let content = serde_json::to_string_pretty(&self.state)
            .context("Failed to serialize workflow continuity state")?;
        
        fs::write(&self.state_file_path, content).await
            .context("Failed to write workflow continuity state file")?;
        
        log_debug!("workflow_continuity", "💾 Saved workflow continuity state");
        Ok(())
    }

    /// Start a new workflow session
    pub async fn start_workflow_session(&mut self, workflow_id: String, project_context: Option<String>) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let workflow_session = WorkflowSession {
            workflow_id: workflow_id.clone(),
            session_id: session_id.clone(),
            project_context,
            current_goal_id: None,
            task_queue: Vec::new(),
            completed_tasks: Vec::new(),
            context_variables: HashMap::new(),
            started_at: now,
            last_activity: now,
            status: WorkflowSessionStatus::Active,
        };
        
        self.state.active_workflows.insert(workflow_id.clone(), workflow_session);
        self.save_state().await?;
        
        log_info!("workflow_continuity", "🚀 Started workflow session: {} (workflow: {})", session_id, workflow_id);
        Ok(session_id)
    }

    /// Resume a suspended workflow
    pub async fn resume_workflow(&mut self, workflow_id: &str) -> Result<String> {
        let suspended_workflow = self.state.suspended_workflows.remove(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("No suspended workflow found with ID: {}", workflow_id))?;
        
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let workflow_session = WorkflowSession {
            workflow_id: workflow_id.to_string(),
            session_id: session_id.clone(),
            project_context: None, // Will be restored from context snapshot
            current_goal_id: None, // Will be restored from saved state
            task_queue: Vec::new(), // Will be restored from saved state
            completed_tasks: Vec::new(),
            context_variables: suspended_workflow.saved_state.execution_context,
            started_at: now,
            last_activity: now,
            status: WorkflowSessionStatus::Active,
        };
        
        self.state.active_workflows.insert(workflow_id.to_string(), workflow_session);
        self.save_state().await?;
        
        log_info!("workflow_continuity", "🔄 Resumed workflow: {} with new session: {}", workflow_id, session_id);
        Ok(session_id)
    }

    /// Suspend an active workflow
    pub async fn suspend_workflow(&mut self, workflow_id: &str, reason: SuspensionReason) -> Result<()> {
        let workflow_session = self.state.active_workflows.remove(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("No active workflow found with ID: {}", workflow_id))?;
        
        // Create context snapshot
        let context_snapshot = self.create_context_snapshot().await?;
        
        // Create saved state from current workflow session
        let saved_state = WorkflowState {
            current_goal: workflow_session.current_goal_id.unwrap_or_default(),
            sub_goals: Vec::new(), // TODO: Extract from workflow orchestrator
            execution_context: workflow_session.context_variables,
            progress_markers: Vec::new(), // TODO: Create from current progress
            error_recovery_state: None,
        };
        
        let suspended_workflow = SuspendedWorkflow {
            workflow_id: workflow_id.to_string(),
            suspension_reason: reason,
            saved_state,
            suspension_time: Utc::now(),
            auto_resume_at: None,
            context_snapshot,
        };
        
        self.state.suspended_workflows.insert(workflow_id.to_string(), suspended_workflow);
        self.save_state().await?;
        
        log_info!("workflow_continuity", "⏸️ Suspended workflow: {}", workflow_id);
        Ok(())
    }

    /// Create a checkpoint for the current workflow state
    pub async fn create_checkpoint(&mut self, workflow_id: &str) -> Result<String> {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        // Get current workflow session
        let workflow_session = self.state.active_workflows.get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("No active workflow found with ID: {}", workflow_id))?;
        
        // Calculate completion percentage first (before borrowing mutably)
        let completion_percentage = {
            let completion = if workflow_session.task_queue.is_empty() {
                100.0
            } else {
                let total_tasks = workflow_session.task_queue.len() + workflow_session.completed_tasks.len();
                (workflow_session.completed_tasks.len() as f64 / total_tasks as f64) * 100.0
            };
            completion
        };
        
        // Create progress marker
        let _progress_marker = ProgressMarker {
            checkpoint_id: checkpoint_id.clone(),
            timestamp: now,
            completion_percentage,
            state_snapshot: workflow_session.context_variables.clone(),
        };
        
        // Update last checkpoint time
        self.state.last_checkpoint = now;
        workflow_session.last_activity = now;
        
        self.save_state().await?;
        
        log_info!("workflow_continuity", "💾 Created checkpoint: {} for workflow: {}", checkpoint_id, workflow_id);
        Ok(checkpoint_id)
    }

    /// Check if workflows should be auto-checkpointed
    pub async fn check_auto_checkpoint(&mut self) -> Result<()> {
        let interval = self.state.settings.auto_checkpoint_interval as i64;
        let should_checkpoint = Utc::now().signed_duration_since(self.state.last_checkpoint).num_minutes() >= interval;
        
        if should_checkpoint {
            for workflow_id in self.state.active_workflows.keys().cloned().collect::<Vec<_>>() {
                if let Err(e) = self.create_checkpoint(&workflow_id).await {
                    log_warn!("workflow_continuity", "⚠️ Failed to create auto-checkpoint for workflow {}: {}", workflow_id, e);
                }
            }
        }
        
        Ok(())
    }

    /// Cleanup old suspended workflows
    pub async fn cleanup_old_workflows(&mut self) -> Result<u32> {
        let max_hours = self.state.settings.max_suspension_hours as i64;
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_hours);
        
        let mut removed_count = 0;
        self.state.suspended_workflows.retain(|_id, workflow| {
            if workflow.suspension_time < cutoff_time {
                removed_count += 1;
                false
            } else {
                true
            }
        });
        
        if removed_count > 0 {
            self.save_state().await?;
            log_info!("workflow_continuity", "🧹 Cleaned up {} old suspended workflows", removed_count);
        }
        
        Ok(removed_count)
    }

    /// Update workflow session with task progress
    pub async fn update_workflow_progress(&mut self, workflow_id: &str, completed_task: String) -> Result<()> {
        let workflow_session = self.state.active_workflows.get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("No active workflow found with ID: {}", workflow_id))?;
        
        workflow_session.completed_tasks.push(completed_task);
        workflow_session.last_activity = Utc::now();
        
        // Auto-checkpoint if interval reached
        if workflow_session.completed_tasks.len() % self.state.settings.auto_checkpoint_interval as usize == 0 {
            self.create_checkpoint(workflow_id).await?;
        }
        
        self.save_state().await?;
        Ok(())
    }

    /// Get workflow continuity statistics
    pub fn get_continuity_stats(&self) -> ContinuityStats {
        let total_executions = self.state.execution_history.len();
        let completed_executions = self.state.execution_history.iter()
            .filter(|e| matches!(e.status, ExecutionStatus::Completed))
            .count();
        
        let success_rate = if total_executions > 0 {
            (completed_executions as f64 / total_executions as f64) * 100.0
        } else {
            100.0
        };
        
        ContinuityStats {
            active_workflows: self.state.active_workflows.len(),
            suspended_workflows: self.state.suspended_workflows.len(),
            total_executions,
            success_rate,
            last_checkpoint: self.state.last_checkpoint,
        }
    }

    /// List all workflow sessions (active and suspended)
    pub fn list_workflow_sessions(&self) -> Vec<WorkflowSessionInfo> {
        let mut sessions = Vec::new();
        
        // Add active workflows
        for (id, session) in &self.state.active_workflows {
            sessions.push(WorkflowSessionInfo {
                workflow_id: id.clone(),
                session_id: Some(session.session_id.clone()),
                status: WorkflowSessionInfoStatus::Active,
                started_at: session.started_at,
                last_activity: Some(session.last_activity),
                task_count: session.completed_tasks.len(),
            });
        }
        
        // Add suspended workflows
        for (id, suspended) in &self.state.suspended_workflows {
            sessions.push(WorkflowSessionInfo {
                workflow_id: id.clone(),
                session_id: None,
                status: WorkflowSessionInfoStatus::Suspended,
                started_at: suspended.suspension_time,
                last_activity: None,
                task_count: 0, // TODO: Extract from saved state
            });
        }
        
        sessions
    }

    async fn create_context_snapshot(&self) -> Result<ContextSnapshot> {
        // Get current project state
        let project_state = {
            let manager = get_project_state_manager();
            manager.get_state().map(|s| s.project_name.clone())
        };
        
        // Create a basic context snapshot
        // In a full implementation, this would capture file states, environment, etc.
        Ok(ContextSnapshot {
            file_states: HashMap::new(),
            environment_vars: std::env::vars().collect(),
            active_tools: Vec::new(),
            project_state,
        })
    }

    fn calculate_completion_percentage(&self, _workflow_session: &WorkflowSession) -> f64 {
        // TODO: Implement actual completion calculation based on workflow progress
        // For now, return a placeholder
        50.0
    }
}

#[derive(Debug)]
pub struct ContinuityStats {
    pub active_workflows: usize,
    pub suspended_workflows: usize,
    pub total_executions: usize,
    pub success_rate: f64,
    pub last_checkpoint: DateTime<Utc>,
}

#[derive(Debug)]
pub struct WorkflowSessionInfo {
    pub workflow_id: String,
    pub session_id: Option<String>,
    pub status: WorkflowSessionInfoStatus,
    pub started_at: DateTime<Utc>,
    pub last_activity: Option<DateTime<Utc>>,
    pub task_count: usize,
}

#[derive(Debug)]
pub enum WorkflowSessionInfoStatus {
    Active,
    Suspended,
}

// Global workflow continuity manager instance
static GLOBAL_WORKFLOW_CONTINUITY_MANAGER: Lazy<Mutex<Option<WorkflowContinuityManager>>> = Lazy::new(|| {
    Mutex::new(None)
});

/// Initialize the global workflow continuity manager
pub async fn initialize_workflow_continuity() -> Result<()> {
    let mut manager = WorkflowContinuityManager::new()
        .context("Failed to create workflow continuity manager")?;
    
    manager.load_state().await
        .context("Failed to load workflow continuity state")?;
    
    // Cleanup old workflows on startup
    if let Err(e) = manager.cleanup_old_workflows().await {
        log_warn!("workflow_continuity", "⚠️ Failed to cleanup old workflows: {}", e);
    }
    
    let mut global_manager = GLOBAL_WORKFLOW_CONTINUITY_MANAGER.lock().unwrap();
    *global_manager = Some(manager);
    
    log_info!("workflow_continuity", "✅ Workflow continuity manager initialized");
    Ok(())
}

/// Get the global workflow continuity manager
pub fn get_workflow_continuity_manager() -> std::sync::MutexGuard<'static, Option<WorkflowContinuityManager>> {
    GLOBAL_WORKFLOW_CONTINUITY_MANAGER.lock().unwrap()
}

/// Start a new workflow session (global function)
pub async fn start_global_workflow_session(workflow_id: String, project_context: Option<String>) -> Result<String> {
    let mut manager_guard = get_workflow_continuity_manager();
    let manager = manager_guard.as_mut()
        .ok_or_else(|| anyhow::anyhow!("Workflow continuity manager not initialized"))?;
    
    manager.start_workflow_session(workflow_id, project_context).await
}

/// Resume a suspended workflow (global function)
pub async fn resume_global_workflow(workflow_id: &str) -> Result<String> {
    let mut manager_guard = get_workflow_continuity_manager();
    let manager = manager_guard.as_mut()
        .ok_or_else(|| anyhow::anyhow!("Workflow continuity manager not initialized"))?;
    
    manager.resume_workflow(workflow_id).await
}

/// Create a checkpoint for active workflow (global function)
pub async fn create_global_checkpoint(workflow_id: &str) -> Result<String> {
    let mut manager_guard = get_workflow_continuity_manager();
    let manager = manager_guard.as_mut()
        .ok_or_else(|| anyhow::anyhow!("Workflow continuity manager not initialized"))?;
    
    manager.create_checkpoint(workflow_id).await
}