use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use crate::logger::{log_debug, log_info, log_warn};
use crate::path_manager::get_path_manager;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::fs;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Project identifier
    pub project_id: String,
    /// Project name
    pub project_name: String,
    /// Project root directory
    pub project_root: PathBuf,
    /// Current working directory within project
    pub current_working_dir: PathBuf,
    /// Last active workflow ID
    pub active_workflow_id: Option<String>,
    /// Project-specific settings
    pub settings: ProjectSettings,
    /// File operation history for this session
    pub file_operations: Vec<FileOperation>,
    /// Task execution context
    pub task_context: TaskContext,
    /// State timestamp
    pub last_updated: DateTime<Utc>,
    /// Session metadata
    pub session_metadata: SessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Preferred MCP servers for this project
    pub preferred_mcp_servers: Vec<String>,
    /// Tool permissions specific to this project
    pub tool_permissions: HashMap<String, String>,
    /// Project-specific environment variables
    pub environment_vars: HashMap<String, String>,
    /// Build/test commands for this project
    pub commands: ProjectCommands,
    /// File patterns to watch/ignore
    pub file_patterns: FilePatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCommands {
    pub build_command: Option<String>,
    pub test_command: Option<String>,
    pub lint_command: Option<String>,
    pub format_command: Option<String>,
    pub dev_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatterns {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub watch_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperationType {
    Read,
    Write,
    Edit,
    Delete,
    Create,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub path: PathBuf,
    pub operation_type: FileOperationType,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: usize,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Currently executing tasks
    pub active_tasks: Vec<String>,
    /// Recently completed tasks
    pub completed_tasks: Vec<String>,
    /// Failed tasks with error information
    pub failed_tasks: Vec<(String, String)>,
    /// Task execution statistics
    pub execution_stats: ExecutionStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_tasks_executed: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time_ms: f64,
    pub last_execution_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub command: String,
    pub working_directory: PathBuf,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_seconds: f64,
    pub timestamp: DateTime<Utc>,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub total_commands: u64,
    pub user_context: HashMap<String, String>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            preferred_mcp_servers: vec!["filesystem".to_string()],
            tool_permissions: HashMap::new(),
            environment_vars: HashMap::new(),
            commands: ProjectCommands::default(),
            file_patterns: FilePatterns::default(),
        }
    }
}

impl Default for ProjectCommands {
    fn default() -> Self {
        Self {
            build_command: None,
            test_command: None,
            lint_command: None,
            format_command: None,
            dev_command: None,
        }
    }
}

impl Default for FilePatterns {
    fn default() -> Self {
        Self {
            include_patterns: vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
            exclude_patterns: vec!["target/**".to_string(), ".git/**".to_string()],
            watch_patterns: vec!["src/**".to_string()],
        }
    }
}

impl Default for TaskContext {
    fn default() -> Self {
        Self {
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            failed_tasks: Vec::new(),
            execution_stats: ExecutionStats::default(),
        }
    }
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self {
            total_tasks_executed: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            average_execution_time_ms: 0.0,
            last_execution_time: None,
        }
    }
}

impl ProjectState {
    pub fn new(project_name: String, project_root: PathBuf) -> Self {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let project_id = format!("{}_{}", project_name, &uuid_str[..8]);
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        Self {
            project_id,
            project_name,
            current_working_dir: project_root.clone(),
            project_root,
            active_workflow_id: None,
            settings: ProjectSettings::default(),
            file_operations: Vec::new(),
            task_context: TaskContext::default(),
            last_updated: now,
            session_metadata: SessionMetadata {
                session_id,
                started_at: now,
                last_activity: now,
                total_commands: 0,
                user_context: HashMap::new(),
            },
        }
    }

    /// Update the last activity timestamp
    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
        self.session_metadata.last_activity = Utc::now();
    }


    /// Add an active task
    pub fn add_active_task(&mut self, task_id: String) {
        self.task_context.active_tasks.push(task_id);
        self.touch();
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, task_id: String, execution_time_ms: f64) {
        self.task_context.active_tasks.retain(|id| id != &task_id);
        self.task_context.completed_tasks.push(task_id);
        
        // Update execution statistics
        self.task_context.execution_stats.total_tasks_executed += 1;
        self.task_context.execution_stats.successful_tasks += 1;
        self.task_context.execution_stats.last_execution_time = Some(Utc::now());
        
        // Update average execution time
        let total = self.task_context.execution_stats.total_tasks_executed as f64;
        let current_avg = self.task_context.execution_stats.average_execution_time_ms;
        self.task_context.execution_stats.average_execution_time_ms = 
            (current_avg * (total - 1.0) + execution_time_ms) / total;
        
        self.touch();
        
        // Keep only the last 50 completed tasks
        if self.task_context.completed_tasks.len() > 50 {
            self.task_context.completed_tasks.remove(0);
        }
    }

    /// Mark a task as failed
    pub fn fail_task(&mut self, task_id: String, error: String) {
        self.task_context.active_tasks.retain(|id| id != &task_id);
        self.task_context.failed_tasks.push((task_id, error));
        
        // Update execution statistics
        self.task_context.execution_stats.total_tasks_executed += 1;
        self.task_context.execution_stats.failed_tasks += 1;
        self.task_context.execution_stats.last_execution_time = Some(Utc::now());
        
        self.touch();
        
        // Keep only the last 20 failed tasks
        if self.task_context.failed_tasks.len() > 20 {
            self.task_context.failed_tasks.remove(0);
        }
    }

    /// Get success rate as a percentage
    pub fn get_success_rate(&self) -> f64 {
        let total = self.task_context.execution_stats.total_tasks_executed;
        if total == 0 {
            return 100.0;
        }
        
        let successful = self.task_context.execution_stats.successful_tasks;
        (successful as f64 / total as f64) * 100.0
    }

    /// Auto-detect project commands by looking at common files
    pub async fn auto_detect_commands(&mut self) -> Result<()> {
        log_info!("project_state", "🔍 Auto-detecting project commands for: {}", self.project_name);
        
        // Check for Rust project
        if self.project_root.join("Cargo.toml").exists() {
            self.settings.commands.build_command = Some("cargo build".to_string());
            self.settings.commands.test_command = Some("cargo test".to_string());
            self.settings.commands.lint_command = Some("cargo clippy".to_string());
            self.settings.commands.format_command = Some("cargo fmt".to_string());
            self.settings.commands.dev_command = Some("cargo run".to_string());
            log_info!("project_state", "📦 Detected Rust project");
        }
        // Check for Node.js project
        else if self.project_root.join("package.json").exists() {
            if let Ok(content) = fs::read_to_string(self.project_root.join("package.json")).await {
                if let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
                        if scripts.contains_key("build") {
                            self.settings.commands.build_command = Some("npm run build".to_string());
                        }
                        if scripts.contains_key("test") {
                            self.settings.commands.test_command = Some("npm test".to_string());
                        }
                        if scripts.contains_key("lint") {
                            self.settings.commands.lint_command = Some("npm run lint".to_string());
                        }
                        if scripts.contains_key("dev") {
                            self.settings.commands.dev_command = Some("npm run dev".to_string());
                        }
                    }
                }
            }
            log_info!("project_state", "📦 Detected Node.js project");
        }
        // Check for Python project
        else if self.project_root.join("requirements.txt").exists() || self.project_root.join("pyproject.toml").exists() {
            self.settings.commands.test_command = Some("python -m pytest".to_string());
            self.settings.commands.lint_command = Some("flake8 .".to_string());
            self.settings.commands.format_command = Some("black .".to_string());
            log_info!("project_state", "🐍 Detected Python project");
        }

        self.touch();
        Ok(())
    }
}

pub struct ProjectStateManager {
    current_state: Option<ProjectState>,
    state_file_path: PathBuf,
}

impl ProjectStateManager {
    pub fn new() -> Self {
        let state_file_path = Self::get_state_file_path();
        Self {
            current_state: None,
            state_file_path,
        }
    }

    fn get_state_file_path() -> PathBuf {
        // Try to use project-local state file first, fall back to global
        let local_state = PathBuf::from(".cai_project_state.json");
        if local_state.parent().map(|p| p.exists()).unwrap_or(false) {
            local_state
        } else {
            // Use global config directory
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("cai")
                .join("project_state.json")
        }
    }

    /// Initialize project state from current path manager context
    pub async fn initialize_from_context(&mut self) -> Result<()> {
        let path_manager = get_path_manager();
        
        if let Some(project_ctx) = path_manager.get_project_context() {
            log_info!("project_state", "🔄 Initializing project state from context: {}", project_ctx.project_name);
            
            let mut project_state = ProjectState::new(
                project_ctx.project_name.clone(),
                project_ctx.project_root.clone()
            );
            
            // Auto-detect project commands
            if let Err(e) = project_state.auto_detect_commands().await {
                log_warn!("project_state", "⚠️ Failed to auto-detect commands: {}", e);
            }
            
            self.current_state = Some(project_state);
            self.save_state().await?;
            
            log_info!("project_state", "✅ Project state initialized successfully");
        } else {
            log_warn!("project_state", "⚠️ No project context found in path manager");
        }
        
        Ok(())
    }

    /// Load project state from disk
    pub async fn load_state(&mut self) -> Result<()> {
        if !self.state_file_path.exists() {
            log_info!("project_state", "📄 No existing project state file found");
            return Ok(());
        }

        let content = fs::read_to_string(&self.state_file_path).await
            .context("Failed to read project state file")?;
        
        let state: ProjectState = serde_json::from_str(&content)
            .context("Failed to parse project state JSON")?;
        
        log_info!("project_state", "📂 Loaded project state for: {}", state.project_name);
        self.current_state = Some(state);
        
        Ok(())
    }

    /// Save project state to disk
    pub async fn save_state(&self) -> Result<()> {
        let Some(ref state) = self.current_state else {
            log_debug!("project_state", "No project state to save");
            return Ok(());
        };

        // Ensure parent directory exists
        if let Some(parent) = self.state_file_path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create state directory")?;
        }

        let content = serde_json::to_string_pretty(state)
            .context("Failed to serialize project state")?;
        
        fs::write(&self.state_file_path, content).await
            .context("Failed to write project state file")?;
        
        log_debug!("project_state", "💾 Saved project state to: {}", self.state_file_path.display());
        Ok(())
    }

    /// Get current project state
    pub fn get_state(&self) -> Option<&ProjectState> {
        self.current_state.as_ref()
    }

    /// Get mutable reference to current project state
    pub fn get_state_mut(&mut self) -> Option<&mut ProjectState> {
        self.current_state.as_mut()
    }

    /// Update project state and save
    pub async fn update_state<F>(&mut self, updater: F) -> Result<()> 
    where
        F: FnOnce(&mut ProjectState),
    {
        if let Some(ref mut state) = self.current_state {
            updater(state);
            state.touch();
            self.save_state().await?;
        }
        Ok(())
    }

    /// Record a file operation
    pub async fn record_file_operation(&mut self, operation: FileOperation) -> Result<()> {
        self.update_state(|state| {
            state.file_operations.push(operation);
            state.session_metadata.last_activity = chrono::Utc::now();
        }).await
    }

    /// Record a command execution
    pub async fn record_execution(&mut self, execution: ExecutionResult) -> Result<()> {
        self.update_state(|state| {
            // Update execution statistics
            state.task_context.execution_stats.total_tasks_executed += 1;
            match execution.status {
                ExecutionStatus::Success => {
                    state.task_context.execution_stats.successful_tasks += 1;
                }
                ExecutionStatus::Failed | ExecutionStatus::Cancelled | ExecutionStatus::Timeout => {
                    state.task_context.execution_stats.failed_tasks += 1;
                }
                ExecutionStatus::Running => {}
            }
            
            // Update average execution time
            let total_time = state.task_context.execution_stats.average_execution_time_ms 
                * (state.task_context.execution_stats.total_tasks_executed - 1) as f64 
                + execution.execution_time_seconds * 1000.0;
            state.task_context.execution_stats.average_execution_time_ms = 
                total_time / state.task_context.execution_stats.total_tasks_executed as f64;
            
            state.task_context.execution_stats.last_execution_time = Some(execution.timestamp);
            state.session_metadata.last_activity = chrono::Utc::now();
            state.session_metadata.total_commands += 1;
        }).await
    }

    /// Add an active task
    pub async fn add_active_task(&mut self, task_id: String) -> Result<()> {
        self.update_state(|state| {
            state.add_active_task(task_id);
        }).await
    }

    /// Complete a task
    pub async fn complete_task(&mut self, task_id: String, execution_time_ms: f64) -> Result<()> {
        self.update_state(|state| {
            state.complete_task(task_id, execution_time_ms);
        }).await
    }

    /// Fail a task
    pub async fn fail_task(&mut self, task_id: String, error: String) -> Result<()> {
        self.update_state(|state| {
            state.fail_task(task_id, error);
        }).await
    }

    /// Set active workflow
    pub async fn set_active_workflow(&mut self, workflow_id: Option<String>) -> Result<()> {
        self.update_state(|state| {
            state.active_workflow_id = workflow_id;
        }).await
    }

    /// Get project statistics
    pub fn get_project_stats(&self) -> Option<ProjectStats> {
        self.current_state.as_ref().map(|state| {
            ProjectStats {
                project_name: state.project_name.clone(),
                total_tasks: state.task_context.execution_stats.total_tasks_executed,
                success_rate: state.get_success_rate(),
                average_execution_time: state.task_context.execution_stats.average_execution_time_ms,
                active_tasks: state.task_context.active_tasks.len(),
                total_file_operations: state.file_operations.len(),
                last_activity: state.session_metadata.last_activity,
            }
        })
    }
}

#[derive(Debug)]
pub struct ProjectStats {
    pub project_name: String,
    pub total_tasks: u64,
    pub success_rate: f64,
    pub average_execution_time: f64,
    pub active_tasks: usize,
    pub total_file_operations: usize,
    pub last_activity: DateTime<Utc>,
}

// Global project state manager instance
static GLOBAL_PROJECT_STATE_MANAGER: Lazy<Mutex<ProjectStateManager>> = Lazy::new(|| {
    Mutex::new(ProjectStateManager::new())
});

/// Get the global project state manager instance
pub fn get_project_state_manager() -> std::sync::MutexGuard<'static, ProjectStateManager> {
    GLOBAL_PROJECT_STATE_MANAGER.lock().unwrap()
}

/// Initialize project state manager
pub async fn initialize_project_state_manager() -> Result<()> {
    let mut manager = get_project_state_manager();
    manager.load_state().await
        .context("Failed to load existing project state")?;
    
    // If no state was loaded, try to initialize from current context
    if manager.get_state().is_none() {
        manager.initialize_from_context().await
            .context("Failed to initialize project state from context")?;
    }
    
    log_info!("project_state", "✅ Project state manager initialized");
    Ok(())
}

/// Record a file operation in global state
pub async fn record_global_file_operation(operation: FileOperation) -> Result<()> {
    let mut manager = get_project_state_manager();
    manager.record_file_operation(operation).await
}

/// Add active task to global state
pub async fn add_global_active_task(task_id: String) -> Result<()> {
    let mut manager = get_project_state_manager();
    manager.add_active_task(task_id).await
}

/// Complete task in global state
pub async fn complete_global_task(task_id: String, execution_time_ms: f64) -> Result<()> {
    let mut manager = get_project_state_manager();
    manager.complete_task(task_id, execution_time_ms).await
}

/// Fail task in global state
pub async fn fail_global_task(task_id: String, error: String) -> Result<()> {
    let mut manager = get_project_state_manager();
    manager.fail_task(task_id, error).await
}