use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Hook system for extensible pre/post-execution validation and customization
#[derive(Debug)]
pub struct HookManager {
    /// Registered hooks by event type
    hooks: Arc<RwLock<HashMap<HookEvent, Vec<Box<dyn ExecutionHook>>>>>,
    /// Hook configuration
    config: HookConfig,
    /// Hook execution statistics
    stats: Arc<RwLock<HookStatistics>>,
    /// Custom hook scripts directory
    scripts_directory: PathBuf,
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Enable hook system
    pub enabled: bool,
    /// Maximum execution time for hooks (seconds)
    pub timeout: u64,
    /// Continue on hook failures
    pub continue_on_failure: bool,
    /// Parallel execution of hooks
    pub parallel_execution: bool,
    /// Maximum number of concurrent hooks
    pub max_concurrent_hooks: usize,
    /// Hook script directories
    pub script_directories: Vec<PathBuf>,
    /// Environment variables for hook execution
    pub environment: HashMap<String, String>,
}

/// Hook execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStatistics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time: f64,
    pub hook_performance: HashMap<String, HookPerformance>,
}

/// Performance metrics for individual hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPerformance {
    pub executions: u64,
    pub success_rate: f64,
    pub average_duration: f64,
    pub last_execution: Option<std::time::SystemTime>,
}

/// Events that can trigger hooks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    // Application lifecycle
    ApplicationStartup,
    ApplicationShutdown,
    
    // Session management
    SessionStarted,
    SessionEnded,
    SessionRestored,
    
    // Task execution
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskCancelled,
    
    // File operations
    FileRead,
    FileWrite,
    FileDelete,
    FileCreate,
    
    // MCP operations
    MCPServerStarted,
    MCPServerStopped,
    MCPToolCalled,
    
    // Workflow operations
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowStepStarted,
    WorkflowStepCompleted,
    
    // Git operations
    GitCommit,
    GitBranchCreated,
    GitPullRequestCreated,
    
    // Error handling
    ErrorOccurred,
    ErrorRecovered,
    
    // Configuration changes
    ConfigurationChanged,
    ConfigurationReloaded,
    
    // Custom events
    Custom(String),
}

/// Context provided to hooks during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub execution_id: String,
    /// Event that triggered the hook
    pub event: HookEvent,
    /// Timestamp of execution
    pub timestamp: std::time::SystemTime,
    /// Session ID
    pub session_id: String,
    /// User ID (if available)
    pub user_id: Option<String>,
    /// Operation details
    pub operation: OperationDetails,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Previous hook results in this execution chain
    pub previous_results: Vec<HookResult>,
}

/// Details about the operation being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDetails {
    /// Type of operation
    pub operation_type: String,
    /// Operation parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Files involved in the operation
    pub files_involved: Vec<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Result of hook execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Success status
    pub success: bool,
    /// Exit code (for process-based operations)
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: Option<String>,
    /// Standard error
    pub stderr: Option<String>,
    /// Duration of operation
    pub duration: std::time::Duration,
    /// Files created/modified
    pub files_affected: Vec<PathBuf>,
    /// Additional result data
    pub data: HashMap<String, serde_json::Value>,
}

/// Result of hook execution with action to take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookResult {
    /// Continue with normal execution
    Continue,
    /// Block the operation
    Block { reason: String },
    /// Modify the execution context
    Modify { context: ExecutionContext },
    /// Skip remaining hooks and continue
    Skip,
    /// Retry the operation
    Retry { max_attempts: u32 },
    /// Execute alternative action
    Alternative { action: String, parameters: HashMap<String, serde_json::Value> },
}

/// Trait for implementing execution hooks
#[async_trait]
pub trait ExecutionHook: Send + Sync + std::fmt::Debug {
    /// Name of the hook
    fn name(&self) -> &str;
    
    /// Description of what the hook does
    fn description(&self) -> &str;
    
    /// Events this hook should respond to
    fn events(&self) -> Vec<HookEvent>;
    
    /// Priority of this hook (higher = executed first)
    fn priority(&self) -> i32 { 0 }
    
    /// Pre-execution hook
    async fn pre_execute(&self, context: &ExecutionContext) -> Result<HookResult> {
        Ok(HookResult::Continue)
    }
    
    /// Post-execution hook
    async fn post_execute(&self, context: &ExecutionContext, result: &ExecutionResult) -> Result<HookResult> {
        Ok(HookResult::Continue)
    }
    
    /// Validation hook (called before pre_execute)
    async fn validate(&self, context: &ExecutionContext) -> Result<bool> {
        Ok(true)
    }
    
    /// Cleanup hook (called if operation is cancelled or fails)
    async fn cleanup(&self, context: &ExecutionContext) -> Result<()> {
        Ok(())
    }
}

/// Script-based hook implementation
#[derive(Debug)]
pub struct ScriptHook {
    name: String,
    description: String,
    script_path: PathBuf,
    events: Vec<HookEvent>,
    priority: i32,
    timeout: std::time::Duration,
}

/// Built-in security validation hook
#[derive(Debug)]
pub struct SecurityValidationHook {
    name: String,
    blocked_patterns: Vec<String>,
    allowed_directories: Vec<PathBuf>,
    max_file_size: u64,
}

/// Built-in logging hook
#[derive(Debug)]
pub struct LoggingHook {
    name: String,
    log_level: String,
    log_format: String,
}

/// Built-in backup hook
#[derive(Debug)]
pub struct BackupHook {
    name: String,
    backup_directory: PathBuf,
    max_backups: usize,
}

impl HookManager {
    /// Create a new hook manager
    pub async fn new(config: HookConfig) -> Result<Self> {
        let scripts_directory = config.script_directories.first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("./hooks"));

        let manager = Self {
            hooks: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(HookStatistics::default())),
            scripts_directory,
        };

        // Load script hooks from directories
        manager.load_script_hooks().await?;
        
        // Register built-in hooks
        manager.register_builtin_hooks().await?;

        Ok(manager)
    }

    /// Register a hook
    pub async fn register_hook(&self, hook: Box<dyn ExecutionHook>) -> Result<()> {
        let mut hooks = self.hooks.write().await;
        
        for event in hook.events() {
            let hook_list = hooks.entry(event).or_insert_with(Vec::new);
            hook_list.push(hook.as_ref().into());
            // Sort by priority (highest first)
            hook_list.sort_by(|a, b| b.priority().cmp(&a.priority()));
        }

        Ok(())
    }

    /// Execute pre-execution hooks
    pub async fn execute_pre_hooks(
        &self,
        event: HookEvent,
        context: &ExecutionContext,
    ) -> Result<Vec<HookResult>> {
        if !self.config.enabled {
            return Ok(vec![HookResult::Continue]);
        }

        let hooks = self.hooks.read().await;
        let event_hooks = match hooks.get(&event) {
            Some(hooks) => hooks,
            None => return Ok(vec![HookResult::Continue]),
        };

        let mut results = Vec::new();
        let start_time = std::time::Instant::now();

        if self.config.parallel_execution {
            results = self.execute_hooks_parallel(event_hooks, context, true).await?;
        } else {
            results = self.execute_hooks_sequential(event_hooks, context, true).await?;
        }

        // Update statistics
        self.update_statistics(event_hooks.len(), start_time.elapsed(), &results).await;

        Ok(results)
    }

    /// Execute post-execution hooks
    pub async fn execute_post_hooks(
        &self,
        event: HookEvent,
        context: &ExecutionContext,
        result: &ExecutionResult,
    ) -> Result<Vec<HookResult>> {
        if !self.config.enabled {
            return Ok(vec![HookResult::Continue]);
        }

        let hooks = self.hooks.read().await;
        let event_hooks = match hooks.get(&event) {
            Some(hooks) => hooks,
            None => return Ok(vec![HookResult::Continue]),
        };

        let mut results = Vec::new();
        let start_time = std::time::Instant::now();

        if self.config.parallel_execution {
            results = self.execute_post_hooks_parallel(event_hooks, context, result).await?;
        } else {
            results = self.execute_post_hooks_sequential(event_hooks, context, result).await?;
        }

        // Update statistics
        self.update_statistics(event_hooks.len(), start_time.elapsed(), &results).await;

        Ok(results)
    }

    /// Execute hooks sequentially
    async fn execute_hooks_sequential(
        &self,
        hooks: &[Box<dyn ExecutionHook>],
        context: &ExecutionContext,
        is_pre_execution: bool,
    ) -> Result<Vec<HookResult>> {
        let mut results = Vec::new();

        for hook in hooks {
            // Validate hook
            if !hook.validate(context).await? {
                continue;
            }

            // Execute hook with timeout
            let hook_result = if is_pre_execution {
                tokio::time::timeout(
                    std::time::Duration::from_secs(self.config.timeout),
                    hook.pre_execute(context),
                ).await??
            } else {
                // For post-execution, we need the execution result
                return Err(anyhow!("Post-execution hooks need ExecutionResult"));
            };

            match &hook_result {
                HookResult::Block { reason } => {
                    if !self.config.continue_on_failure {
                        return Ok(vec![hook_result]);
                    }
                }
                HookResult::Skip => {
                    results.push(hook_result);
                    break;
                }
                _ => {}
            }

            results.push(hook_result);
        }

        Ok(results)
    }

    /// Execute post-hooks sequentially
    async fn execute_post_hooks_sequential(
        &self,
        hooks: &[Box<dyn ExecutionHook>],
        context: &ExecutionContext,
        result: &ExecutionResult,
    ) -> Result<Vec<HookResult>> {
        let mut results = Vec::new();

        for hook in hooks {
            // Validate hook
            if !hook.validate(context).await? {
                continue;
            }

            // Execute hook with timeout
            let hook_result = tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout),
                hook.post_execute(context, result),
            ).await??;

            results.push(hook_result);
        }

        Ok(results)
    }

    /// Execute hooks in parallel
    async fn execute_hooks_parallel(
        &self,
        hooks: &[Box<dyn ExecutionHook>],
        context: &ExecutionContext,
        is_pre_execution: bool,
    ) -> Result<Vec<HookResult>> {
        use tokio::sync::Semaphore;

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_hooks));
        let mut tasks = Vec::new();

        for hook in hooks {
            let hook_ref = hook.as_ref();
            let context_clone = context.clone();
            let semaphore_clone = Arc::clone(&semaphore);
            let timeout = self.config.timeout;

            let task = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                // Validate hook
                if !hook_ref.validate(&context_clone).await.unwrap_or(false) {
                    return Ok(HookResult::Continue);
                }

                // Execute hook with timeout
                if is_pre_execution {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(timeout),
                        hook_ref.pre_execute(&context_clone),
                    ).await?
                } else {
                    // For parallel execution of pre-hooks only
                    Ok(HookResult::Continue)
                }
            });

            tasks.push(task);
        }

        let mut results = Vec::new();
        for task in tasks {
            match task.await? {
                Ok(result) => results.push(result),
                Err(e) => {
                    if !self.config.continue_on_failure {
                        return Err(e);
                    }
                    results.push(HookResult::Continue);
                }
            }
        }

        Ok(results)
    }

    /// Execute post-hooks in parallel
    async fn execute_post_hooks_parallel(
        &self,
        hooks: &[Box<dyn ExecutionHook>],
        context: &ExecutionContext,
        result: &ExecutionResult,
    ) -> Result<Vec<HookResult>> {
        use tokio::sync::Semaphore;

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_hooks));
        let mut tasks = Vec::new();

        for hook in hooks {
            let hook_ref = hook.as_ref();
            let context_clone = context.clone();
            let result_clone = result.clone();
            let semaphore_clone = Arc::clone(&semaphore);
            let timeout = self.config.timeout;

            let task = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                // Validate hook
                if !hook_ref.validate(&context_clone).await.unwrap_or(false) {
                    return Ok(HookResult::Continue);
                }

                // Execute hook with timeout
                tokio::time::timeout(
                    std::time::Duration::from_secs(timeout),
                    hook_ref.post_execute(&context_clone, &result_clone),
                ).await?
            });

            tasks.push(task);
        }

        let mut results = Vec::new();
        for task in tasks {
            match task.await? {
                Ok(result) => results.push(result),
                Err(e) => {
                    if !self.config.continue_on_failure {
                        return Err(e);
                    }
                    results.push(HookResult::Continue);
                }
            }
        }

        Ok(results)
    }

    /// Load script hooks from configured directories
    async fn load_script_hooks(&self) -> Result<()> {
        for script_dir in &self.config.script_directories {
            if !script_dir.exists() {
                continue;
            }

            let mut entries = fs::read_dir(script_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() && self.is_executable_script(&path) {
                    let hook = self.create_script_hook(path).await?;
                    self.register_hook(Box::new(hook)).await?;
                }
            }
        }

        Ok(())
    }

    /// Check if file is an executable script
    fn is_executable_script(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            matches!(extension.to_str(), Some("sh") | Some("py") | Some("js") | Some("rb"))
        } else {
            // Check if file is executable (Unix systems)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(path) {
                    let permissions = metadata.permissions();
                    permissions.mode() & 0o111 != 0
                } else {
                    false
                }
            }
            #[cfg(not(unix))]
            {
                false
            }
        }
    }

    /// Create a script hook from a file
    async fn create_script_hook(&self, script_path: PathBuf) -> Result<ScriptHook> {
        // Parse hook metadata from script comments
        let content = fs::read_to_string(&script_path).await?;
        let metadata = self.parse_hook_metadata(&content);

        Ok(ScriptHook {
            name: metadata.get("name").cloned()
                .unwrap_or_else(|| script_path.file_stem().unwrap().to_string_lossy().to_string()),
            description: metadata.get("description").cloned()
                .unwrap_or_else(|| "Script-based hook".to_string()),
            script_path,
            events: metadata.get("events").map(|events_str| {
                events_str.split(',')
                    .filter_map(|event| self.parse_hook_event(event.trim()))
                    .collect()
            }).unwrap_or_else(|| vec![HookEvent::Custom("default".to_string())]),
            priority: metadata.get("priority")
                .and_then(|p| p.parse().ok())
                .unwrap_or(0),
            timeout: std::time::Duration::from_secs(
                metadata.get("timeout")
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(30)
            ),
        })
    }

    /// Parse hook metadata from script comments
    fn parse_hook_metadata(&self, content: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("#") || line.starts_with("//") {
                if let Some(meta_line) = line.strip_prefix("#").or_else(|| line.strip_prefix("//")) {
                    let meta_line = meta_line.trim();
                    if meta_line.starts_with("@") {
                        if let Some((key, value)) = meta_line[1..].split_once(':') {
                            metadata.insert(key.trim().to_string(), value.trim().to_string());
                        }
                    }
                }
            }
        }

        metadata
    }

    /// Parse hook event from string
    fn parse_hook_event(&self, event_str: &str) -> Option<HookEvent> {
        match event_str.to_lowercase().as_str() {
            "application_startup" => Some(HookEvent::ApplicationStartup),
            "application_shutdown" => Some(HookEvent::ApplicationShutdown),
            "session_started" => Some(HookEvent::SessionStarted),
            "session_ended" => Some(HookEvent::SessionEnded),
            "task_started" => Some(HookEvent::TaskStarted),
            "task_completed" => Some(HookEvent::TaskCompleted),
            "file_read" => Some(HookEvent::FileRead),
            "file_write" => Some(HookEvent::FileWrite),
            "git_commit" => Some(HookEvent::GitCommit),
            "error_occurred" => Some(HookEvent::ErrorOccurred),
            _ => Some(HookEvent::Custom(event_str.to_string())),
        }
    }

    /// Register built-in hooks
    async fn register_builtin_hooks(&self) -> Result<()> {
        // Security validation hook
        let security_hook = SecurityValidationHook {
            name: "security_validation".to_string(),
            blocked_patterns: vec![
                "*.exe".to_string(),
                "*.dll".to_string(),
                "/etc/passwd".to_string(),
                "/etc/shadow".to_string(),
            ],
            allowed_directories: vec![
                PathBuf::from("./"),
                PathBuf::from("./src/"),
                PathBuf::from("./docs/"),
            ],
            max_file_size: 100 * 1024 * 1024, // 100 MB
        };
        self.register_hook(Box::new(security_hook)).await?;

        // Logging hook
        let logging_hook = LoggingHook {
            name: "execution_logging".to_string(),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
        };
        self.register_hook(Box::new(logging_hook)).await?;

        // Backup hook
        let backup_hook = BackupHook {
            name: "file_backup".to_string(),
            backup_directory: PathBuf::from("./.cai_backups"),
            max_backups: 10,
        };
        self.register_hook(Box::new(backup_hook)).await?;

        Ok(())
    }

    /// Update execution statistics
    async fn update_statistics(
        &self,
        hook_count: usize,
        duration: std::time::Duration,
        results: &[HookResult],
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_executions += hook_count as u64;
        
        let successful = results.iter()
            .filter(|r| !matches!(r, HookResult::Block { .. }))
            .count() as u64;
        
        stats.successful_executions += successful;
        stats.failed_executions += (hook_count as u64) - successful;
        
        // Update average execution time
        let total_time = stats.average_execution_time * (stats.total_executions - hook_count as u64) as f64;
        stats.average_execution_time = (total_time + duration.as_secs_f64()) / stats.total_executions as f64;
    }

    /// Get hook statistics
    pub async fn get_statistics(&self) -> HookStatistics {
        self.stats.read().await.clone()
    }

    /// Trigger a custom event
    pub async fn trigger_custom_event(
        &self,
        event_name: String,
        context: ExecutionContext,
    ) -> Result<Vec<HookResult>> {
        let custom_event = HookEvent::Custom(event_name);
        self.execute_pre_hooks(custom_event, &context).await
    }
}

// Implementation of built-in hooks
#[async_trait]
impl ExecutionHook for ScriptHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn events(&self) -> Vec<HookEvent> {
        self.events.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    async fn pre_execute(&self, context: &ExecutionContext) -> Result<HookResult> {
        self.execute_script(context, "pre").await
    }

    async fn post_execute(&self, context: &ExecutionContext, result: &ExecutionResult) -> Result<HookResult> {
        self.execute_script(context, "post").await
    }
}

impl ScriptHook {
    async fn execute_script(&self, context: &ExecutionContext, phase: &str) -> Result<HookResult> {
        let mut command = Command::new(&self.script_path);
        
        // Set environment variables
        command.env("CAI_HOOK_PHASE", phase);
        command.env("CAI_EXECUTION_ID", &context.execution_id);
        command.env("CAI_SESSION_ID", &context.session_id);
        command.env("CAI_EVENT", format!("{:?}", context.event));
        
        // Add operation details as JSON
        let operation_json = serde_json::to_string(&context.operation)?;
        command.env("CAI_OPERATION", operation_json);

        // Execute with timeout
        let output = tokio::time::timeout(self.timeout, command.output()).await??;

        if output.status.success() {
            // Parse script output for hook result
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.parse_script_result(&stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(HookResult::Block {
                reason: format!("Script failed: {}", stderr),
            })
        }
    }

    fn parse_script_result(&self, output: &str) -> Result<HookResult> {
        // Look for special output formats
        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("CAI_HOOK_RESULT:") {
                let result_str = line.strip_prefix("CAI_HOOK_RESULT:").unwrap().trim();
                match result_str.to_lowercase().as_str() {
                    "continue" => return Ok(HookResult::Continue),
                    "skip" => return Ok(HookResult::Skip),
                    result if result.starts_with("block:") => {
                        let reason = result.strip_prefix("block:").unwrap_or("Script blocked execution");
                        return Ok(HookResult::Block { reason: reason.to_string() });
                    }
                    _ => {}
                }
            }
        }

        // Default to continue if no specific result found
        Ok(HookResult::Continue)
    }
}

#[async_trait]
impl ExecutionHook for SecurityValidationHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Validates file operations for security compliance"
    }

    fn events(&self) -> Vec<HookEvent> {
        vec![
            HookEvent::FileRead,
            HookEvent::FileWrite,
            HookEvent::FileCreate,
            HookEvent::FileDelete,
        ]
    }

    fn priority(&self) -> i32 {
        100 // High priority for security
    }

    async fn pre_execute(&self, context: &ExecutionContext) -> Result<HookResult> {
        // Check files against blocked patterns
        for file in &context.operation.files_involved {
            let file_str = file.to_string_lossy();
            
            // Check blocked patterns
            for pattern in &self.blocked_patterns {
                if glob_match::glob_match(pattern, &file_str) {
                    return Ok(HookResult::Block {
                        reason: format!("File matches blocked pattern: {}", pattern),
                    });
                }
            }

            // Check allowed directories
            let in_allowed_dir = self.allowed_directories.iter()
                .any(|allowed| file.starts_with(allowed));
            
            if !in_allowed_dir {
                return Ok(HookResult::Block {
                    reason: format!("File outside allowed directories: {}", file.display()),
                });
            }

            // Check file size for write operations
            if matches!(context.event, HookEvent::FileWrite | HookEvent::FileCreate) {
                if file.exists() {
                    if let Ok(metadata) = file.metadata() {
                        if metadata.len() > self.max_file_size {
                            return Ok(HookResult::Block {
                                reason: format!("File too large: {} bytes", metadata.len()),
                            });
                        }
                    }
                }
            }
        }

        Ok(HookResult::Continue)
    }
}

#[async_trait]
impl ExecutionHook for LoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Logs execution events for audit and debugging"
    }

    fn events(&self) -> Vec<HookEvent> {
        vec![
            HookEvent::TaskStarted,
            HookEvent::TaskCompleted,
            HookEvent::TaskFailed,
            HookEvent::FileWrite,
            HookEvent::GitCommit,
            HookEvent::ErrorOccurred,
        ]
    }

    async fn pre_execute(&self, context: &ExecutionContext) -> Result<HookResult> {
        let log_entry = serde_json::json!({
            "timestamp": context.timestamp,
            "event": format!("{:?}", context.event),
            "execution_id": context.execution_id,
            "session_id": context.session_id,
            "operation": context.operation,
            "phase": "pre"
        });

        println!("AUDIT: {}", log_entry);
        Ok(HookResult::Continue)
    }

    async fn post_execute(&self, context: &ExecutionContext, result: &ExecutionResult) -> Result<HookResult> {
        let log_entry = serde_json::json!({
            "timestamp": context.timestamp,
            "event": format!("{:?}", context.event),
            "execution_id": context.execution_id,
            "session_id": context.session_id,
            "operation": context.operation,
            "result": {
                "success": result.success,
                "duration": result.duration.as_secs_f64(),
                "files_affected": result.files_affected
            },
            "phase": "post"
        });

        println!("AUDIT: {}", log_entry);
        Ok(HookResult::Continue)
    }
}

#[async_trait]
impl ExecutionHook for BackupHook {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Creates backups of files before modification"
    }

    fn events(&self) -> Vec<HookEvent> {
        vec![HookEvent::FileWrite, HookEvent::FileDelete]
    }

    fn priority(&self) -> i32 {
        50 // Medium priority
    }

    async fn pre_execute(&self, context: &ExecutionContext) -> Result<HookResult> {
        // Create backup directory if it doesn't exist
        if !self.backup_directory.exists() {
            fs::create_dir_all(&self.backup_directory).await?;
        }

        // Backup files that will be modified
        for file in &context.operation.files_involved {
            if file.exists() {
                let backup_name = format!(
                    "{}_{}.backup",
                    file.file_name().unwrap().to_string_lossy(),
                    context.timestamp.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().as_secs()
                );
                
                let backup_path = self.backup_directory.join(backup_name);
                fs::copy(file, backup_path).await?;
            }
        }

        // Clean up old backups
        self.cleanup_old_backups().await?;

        Ok(HookResult::Continue)
    }
}

impl BackupHook {
    async fn cleanup_old_backups(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.backup_directory).await?;
        let mut backup_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let metadata = entry.metadata().await?;
                backup_files.push((entry.path(), metadata.modified()?));
            }
        }

        // Sort by modification time (newest first)
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove excess backups
        if backup_files.len() > self.max_backups {
            for (path, _) in backup_files.into_iter().skip(self.max_backups) {
                let _ = fs::remove_file(path).await;
            }
        }

        Ok(())
    }
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: 30,
            continue_on_failure: false,
            parallel_execution: false,
            max_concurrent_hooks: 4,
            script_directories: vec![PathBuf::from("./hooks")],
            environment: HashMap::new(),
        }
    }
}

impl Default for HookStatistics {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time: 0.0,
            hook_performance: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hook_manager_creation() {
        let config = HookConfig::default();
        let manager = HookManager::new(config).await.unwrap();
        
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_executions, 0);
    }

    #[tokio::test]
    async fn test_security_hook() {
        let hook = SecurityValidationHook {
            name: "test_security".to_string(),
            blocked_patterns: vec!["*.exe".to_string()],
            allowed_directories: vec![PathBuf::from("./")],
            max_file_size: 1024,
        };

        let context = ExecutionContext {
            execution_id: "test".to_string(),
            event: HookEvent::FileWrite,
            timestamp: std::time::SystemTime::now(),
            session_id: "test_session".to_string(),
            user_id: None,
            operation: OperationDetails {
                operation_type: "file_write".to_string(),
                parameters: HashMap::new(),
                files_involved: vec![PathBuf::from("./malware.exe")],
                metadata: HashMap::new(),
            },
            environment: HashMap::new(),
            previous_results: Vec::new(),
        };

        let result = hook.pre_execute(&context).await.unwrap();
        assert!(matches!(result, HookResult::Block { .. }));
    }

    #[tokio::test]
    async fn test_custom_event() {
        let config = HookConfig::default();
        let manager = HookManager::new(config).await.unwrap();

        let context = ExecutionContext {
            execution_id: "test".to_string(),
            event: HookEvent::Custom("test_event".to_string()),
            timestamp: std::time::SystemTime::now(),
            session_id: "test_session".to_string(),
            user_id: None,
            operation: OperationDetails {
                operation_type: "test".to_string(),
                parameters: HashMap::new(),
                files_involved: Vec::new(),
                metadata: HashMap::new(),
            },
            environment: HashMap::new(),
            previous_results: Vec::new(),
        };

        let results = manager.trigger_custom_event("test_event".to_string(), context).await.unwrap();
        assert!(!results.is_empty());
    }
}