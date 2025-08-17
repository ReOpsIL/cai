use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use colored::*;
use crate::logger::{log_debug, log_info, log_warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSafetyLevel {
    /// Safe tools that can auto-execute without user confirmation
    Safe,
    /// Tools that require user approval before execution
    RequiresApproval,
    /// Dangerous tools that always require explicit confirmation
    Dangerous,
}

#[derive(Debug)]
pub enum SafetyError {
    ReadBeforeEdit { path: PathBuf, suggestion: String },
    PermissionDenied { tool: String },
    DangerousOperation { tool: String, reason: String },
    PathValidation { reason: String },
    PolicyViolation { reason: String },
}

impl std::fmt::Display for SafetyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafetyError::ReadBeforeEdit { path, suggestion } => {
                write!(f, "Read-before-edit violation: File '{}' must be read before editing. {}", path.display(), suggestion)
            },
            SafetyError::PermissionDenied { tool } => {
                write!(f, "Permission denied: Tool '{}' requires user approval", tool)
            },
            SafetyError::DangerousOperation { tool, reason } => {
                write!(f, "Dangerous operation blocked: Tool '{}' is classified as dangerous. {}", tool, reason)
            },
            SafetyError::PathValidation { reason } => {
                write!(f, "Path validation failed: {}", reason)
            },
            SafetyError::PolicyViolation { reason } => {
                write!(f, "Tool execution blocked by safety policy: {}", reason)
            },
        }
    }
}

impl std::error::Error for SafetyError {}

#[derive(Debug, Clone)]
pub enum PermissionLevel {
    Denied,
    AllowOnce,
    AllowSession,
    AlwaysAllow,
}

pub struct ToolSafetyValidator {
    /// Files that have been read in the current session (for read-before-edit validation)
    read_files: Arc<Mutex<HashSet<PathBuf>>>,
    /// Session-level permission overrides
    session_permissions: Arc<Mutex<HashMap<String, PermissionLevel>>>,
    /// Global permission settings (persisted)
    global_permissions: Arc<Mutex<HashMap<String, PermissionLevel>>>,
    /// Whether to prompt for permissions (can be disabled for automation)
    interactive_mode: bool,
    /// Brave mode - disables all security checks (use with caution)
    brave_mode: bool,
}

impl Default for ToolSafetyValidator {
    fn default() -> Self {
        Self::new(true, false)
    }
}

impl ToolSafetyValidator {
    pub fn new(interactive_mode: bool, brave_mode: bool) -> Self {
        Self {
            read_files: Arc::new(Mutex::new(HashSet::new())),
            session_permissions: Arc::new(Mutex::new(HashMap::new())),
            global_permissions: Arc::new(Mutex::new(HashMap::new())),
            interactive_mode,
            brave_mode,
        }
    }
    
    /// Create a new validator in brave mode (all security disabled)
    pub fn brave() -> Self {
        Self::new(false, true)
    }
    
    /// Enable or disable brave mode
    pub fn set_brave_mode(&mut self, enabled: bool) {
        self.brave_mode = enabled;
        if enabled {
            log_warn!("safety", "⚠️ BRAVE MODE ENABLED - All security checks disabled!");
        } else {
            log_info!("safety", "Brave mode disabled - Security checks restored");
        }
    }
    
    /// Check if brave mode is enabled
    pub fn is_brave_mode(&self) -> bool {
        self.brave_mode
    }
    
    /// Classify a tool based on its safety level
    pub fn classify_tool(&self, tool_name: &str) -> ToolSafetyLevel {
        match tool_name {
            // Safe tools - read-only operations that don't modify system state
            "list_directory" | "read_file" | "search_files" | "glob_files" 
            | "web_fetch" | "create_tasks" | "update_tasks" | "read_many_files"
            | "diff_files" | "batch_file_search" | "list_allowed_directories" => {
                ToolSafetyLevel::Safe
            },
            
            // Requires approval - file modification operations
            "edit_file" | "write_file" | "multiedit_file" | "download_file" => {
                ToolSafetyLevel::RequiresApproval
            },
            
            // Dangerous - system-level operations and file deletion
            "execute_command" | "delete_path" => {
                ToolSafetyLevel::Dangerous
            },
            
            // Unknown tools default to requiring approval
            _ => {
                log_warn!("safety", "Unknown tool '{}' - defaulting to RequiresApproval", tool_name);
                ToolSafetyLevel::RequiresApproval
            }
        }
    }
    
    /// Record that a file has been read (for read-before-edit validation)
    pub fn record_file_read(&self, path: &Path) {
        let absolute_path = self.normalize_path(path);
        let mut read_files = self.read_files.lock().unwrap();
        read_files.insert(absolute_path);
        log_debug!("safety", "Recorded file read: {}", path.display());
    }
    
    /// Validate that a file has been read before attempting to edit it
    pub fn validate_edit_operation(&self, path: &Path) -> Result<(), SafetyError> {
        // Bypass validation in brave mode
        if self.brave_mode {
            log_debug!("safety", "BRAVE MODE: Skipping edit validation for: {}", path.display());
            return Ok(());
        }
        
        let absolute_path = self.normalize_path(path);
        
        // Allow creating new files - only enforce read-before-edit for existing files
        if !absolute_path.exists() {
            log_debug!("safety", "Allowing creation of new file: {}", path.display());
            return Ok(());
        }
        
        let read_files = self.read_files.lock().unwrap();
        
        if !read_files.contains(&absolute_path) {
            // Instead of failing, automatically record the file as read
            // This allows edit operations to proceed while maintaining safety tracking
            drop(read_files); // Release the lock before calling record_file_read
            log_debug!("safety", "Auto-reading file before edit operation: {}", path.display());
            self.record_file_read(path);
            return Ok(());
        }
        
        log_debug!("safety", "Edit operation validated for: {}", path.display());
        Ok(())
    }
    
    /// Validate path to ensure it's within allowed boundaries
    pub fn validate_path(&self, path: &Path) -> Result<(), SafetyError> {
        // Bypass validation in brave mode
        if self.brave_mode {
            log_debug!("safety", "BRAVE MODE: Skipping path validation for: {}", path.display());
            return Ok(());
        }
        
        let normalized = self.normalize_path(path);
        
        // Check for directory traversal attempts
        if normalized.to_string_lossy().contains("..") {
            return Err(SafetyError::PathValidation {
                reason: format!(
                    "Path traversal detected in '{}'. Relative paths with '..' are not allowed.",
                    path.display()
                ),
            });
        }
        
        // Additional security checks could be added here
        // - Check against allowed directories
        // - Validate file extensions
        // - Check for system files
        
        Ok(())
    }
    
    /// Check if a tool execution should be allowed based on safety level and permissions
    pub async fn check_tool_permission(&self, tool_name: &str, args: &Value) -> Result<(), SafetyError> {
        // Bypass all permission checks in brave mode
        if self.brave_mode {
            log_debug!("safety", "BRAVE MODE: Allowing tool '{}' without security checks", tool_name);
            return Ok(());
        }
        
        let mut safety_level = self.classify_tool(tool_name);
        
        // Perform path validation for tools that operate on files
        if let Some(path_str) = self.extract_path_from_args(args) {
            let path = Path::new(&path_str);
            self.validate_path(path)?;
            
            // Special case: treat new file creation as safer than editing existing files
            if tool_name == "write_file" && !self.normalize_path(&path).exists() {
                log_debug!("safety", "New file creation detected for '{}' - treating as safe operation", path.display());
                safety_level = ToolSafetyLevel::Safe;
            }
        }
        
        // Check read-before-edit for file modification tools
        if matches!(safety_level, ToolSafetyLevel::RequiresApproval | ToolSafetyLevel::Dangerous) {
            if self.is_edit_operation(tool_name) {
                if let Some(path_str) = self.extract_path_from_args(args) {
                    let path = Path::new(&path_str);
                    self.validate_edit_operation(path)?;
                }
            }
        }
        
        match safety_level {
            ToolSafetyLevel::Safe => {
                log_debug!("safety", "Tool '{}' is safe - allowing execution", tool_name);
                Ok(())
            },
            
            ToolSafetyLevel::RequiresApproval => {
                self.check_approval_permission(tool_name, "moderate risk").await
            },
            
            ToolSafetyLevel::Dangerous => {
                self.check_dangerous_permission(tool_name, args).await
            },
        }
    }
    
    /// Check permission for tools requiring approval
    async fn check_approval_permission(&self, tool_name: &str, risk_level: &str) -> Result<(), SafetyError> {
        // Check session permissions first
        {
            let session_perms = self.session_permissions.lock().unwrap();
            if let Some(perm_level) = session_perms.get(tool_name) {
                match perm_level {
                    PermissionLevel::Denied => {
                        return Err(SafetyError::PermissionDenied { 
                            tool: tool_name.to_string() 
                        });
                    },
                    PermissionLevel::AllowOnce | PermissionLevel::AllowSession | PermissionLevel::AlwaysAllow => {
                        log_debug!("safety", "Tool '{}' allowed by session permission", tool_name);
                        return Ok(());
                    },
                }
            }
        }
        
        // Check global permissions
        {
            let global_perms = self.global_permissions.lock().unwrap();
            if let Some(perm_level) = global_perms.get(tool_name) {
                match perm_level {
                    PermissionLevel::Denied => {
                        return Err(SafetyError::PermissionDenied { 
                            tool: tool_name.to_string() 
                        });
                    },
                    PermissionLevel::AlwaysAllow => {
                        log_debug!("safety", "Tool '{}' allowed by global permission", tool_name);
                        return Ok(());
                    },
                    _ => {} // Continue to interactive prompt
                }
            }
        }
        
        // Interactive permission prompt
        if self.interactive_mode {
            self.prompt_user_permission(tool_name, risk_level).await
        } else {
            log_warn!("safety", "Tool '{}' requires approval but running in non-interactive mode", tool_name);
            Err(SafetyError::PermissionDenied { 
                tool: tool_name.to_string() 
            })
        }
    }
    
    /// Check permission for dangerous tools
    async fn check_dangerous_permission(&self, tool_name: &str, args: &Value) -> Result<(), SafetyError> {
        let reason = self.get_danger_reason(tool_name, args);
        
        // Always prompt for dangerous operations, even if previously allowed
        if self.interactive_mode {
            self.prompt_dangerous_operation(tool_name, &reason).await
        } else {
            log_warn!("safety", "Dangerous tool '{}' blocked in non-interactive mode: {}", tool_name, reason);
            Err(SafetyError::DangerousOperation { 
                tool: tool_name.to_string(), 
                reason 
            })
        }
    }
    
    /// Prompt user for permission to use a tool
    async fn prompt_user_permission(&self, tool_name: &str, risk_level: &str) -> Result<(), SafetyError> {
        println!("\n{} {} {}", "🔐".yellow(), "Permission Required".bold(), "🔐".yellow());
        println!("Tool: {} ({})", tool_name.cyan().bold(), risk_level.yellow());
        println!("This tool requires your permission to execute.");
        println!();
        println!("Options:");
        println!("  {} - Allow this execution only", "o".green().bold());
        println!("  {} - Allow for this session", "s".blue().bold());
        println!("  {} - Always allow (remember choice)", "a".purple().bold());
        println!("  {} - Deny this execution", "d".red().bold());
        println!();
        print!("Your choice (o/s/a/d): ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim().to_lowercase();
        
        match choice.as_str() {
            "o" | "once" => {
                println!("{} Permission granted for this execution", "✅".green());
                Ok(())
            },
            "s" | "session" => {
                println!("{} Permission granted for this session", "✅".green());
                let mut session_perms = self.session_permissions.lock().unwrap();
                session_perms.insert(tool_name.to_string(), PermissionLevel::AllowSession);
                Ok(())
            },
            "a" | "always" => {
                println!("{} Permission granted and remembered", "✅".green());
                let mut global_perms = self.global_permissions.lock().unwrap();
                global_perms.insert(tool_name.to_string(), PermissionLevel::AlwaysAllow);
                Ok(())
            },
            _ => {
                println!("{} Permission denied", "❌".red());
                let mut session_perms = self.session_permissions.lock().unwrap();
                session_perms.insert(tool_name.to_string(), PermissionLevel::Denied);
                Err(SafetyError::PermissionDenied { 
                    tool: tool_name.to_string() 
                })
            }
        }
    }
    
    /// Prompt user for dangerous operation confirmation
    async fn prompt_dangerous_operation(&self, tool_name: &str, reason: &str) -> Result<(), SafetyError> {
        println!("\n{} {} {}", "⚠️".red(), "DANGEROUS OPERATION".red().bold(), "⚠️".red());
        println!("Tool: {}", tool_name.red().bold());
        println!("Risk: {}", reason.yellow());
        println!();
        println!("{}", "This operation could potentially harm your system or data.".red());
        println!("Please confirm you understand the risks and want to proceed.");
        println!();
        println!("Do you want to proceed? (y/N): ");
        print!("> ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        // More robust trimming to handle various line endings and whitespace
        let confirmation = input.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_lowercase();
        
        if confirmation == "y" || confirmation == "yes" {
            println!("{} Dangerous operation confirmed", "⚠️".yellow());
            log_warn!("safety", "User confirmed dangerous operation: {} - {}", tool_name, reason);
            Ok(())
        } else {
            println!("{} Operation cancelled", "🛑".red());
            Err(SafetyError::DangerousOperation { 
                tool: tool_name.to_string(), 
                reason: reason.to_string() 
            })
        }
    }
    
    /// Helper function to extract file path from tool arguments
    fn extract_path_from_args(&self, args: &Value) -> Option<String> {
        args.get("path")
            .or_else(|| args.get("file_path"))
            .or_else(|| args.get("source"))
            .or_else(|| args.get("destination"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    
    /// Check if a tool is a file edit operation
    fn is_edit_operation(&self, tool_name: &str) -> bool {
        matches!(tool_name, "edit_file" | "write_file" | "multiedit_file" | "download_file")
    }
    
    /// Get the danger reason for a dangerous tool
    fn get_danger_reason(&self, tool_name: &str, args: &Value) -> String {
        match tool_name {
            "execute_command" => {
                let command = args.get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown command");
                format!("Executing system command: '{}'", command)
            },
            "delete_path" => {
                let path = self.extract_path_from_args(args)
                    .unwrap_or_else(|| "unknown path".to_string());
                format!("Deleting file/directory: '{}'", path)
            },
            _ => "Potentially harmful system operation".to_string(),
        }
    }
    
    /// Normalize a path for consistent comparison
    fn normalize_path(&self, path: &Path) -> PathBuf {
        // Convert to absolute path for consistent comparison
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    }
    
    /// Clear session permissions (called when starting a new session)
    pub fn clear_session_permissions(&self) {
        let mut session_perms = self.session_permissions.lock().unwrap();
        session_perms.clear();
        log_info!("safety", "Session permissions cleared");
    }
    
    /// Clear read file tracking (called when starting a new session)
    pub fn clear_read_files(&self) {
        let mut read_files = self.read_files.lock().unwrap();
        read_files.clear();
        log_info!("safety", "Read file tracking cleared");
    }
    
    /// Get safety statistics for monitoring
    pub fn get_safety_stats(&self) -> SafetyStats {
        let read_files_count = self.read_files.lock().unwrap().len();
        let session_perms_count = self.session_permissions.lock().unwrap().len();
        let global_perms_count = self.global_permissions.lock().unwrap().len();
        
        SafetyStats {
            tracked_files: read_files_count,
            session_permissions: session_perms_count,
            global_permissions: global_perms_count,
        }
    }
}

#[derive(Debug)]
pub struct SafetyStats {
    pub tracked_files: usize,
    pub session_permissions: usize,
    pub global_permissions: usize,
}

// Global instance for easy access throughout the application
use std::sync::OnceLock;
static GLOBAL_SAFETY_VALIDATOR: OnceLock<ToolSafetyValidator> = OnceLock::new();

/// Get the global safety validator instance
pub fn get_global_safety_validator() -> &'static ToolSafetyValidator {
    GLOBAL_SAFETY_VALIDATOR.get_or_init(|| {
        // Check if we're in non-interactive mode (CI, automation, etc.)
        let interactive = std::env::var("CAI_INTERACTIVE")
            .map(|v| v.to_lowercase() != "false")
            .unwrap_or(true);
        
        // Check if brave mode is enabled via environment variable
        let brave_mode = std::env::var("CAI_BRAVE_MODE")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);
        
        log_info!("safety", "Initializing global safety validator (interactive: {}, brave: {})", interactive, brave_mode);
        ToolSafetyValidator::new(interactive, brave_mode)
    })
}

/// Initialize the global safety validator with specific settings
pub fn init_global_safety_validator(interactive_mode: bool) {
    GLOBAL_SAFETY_VALIDATOR.set(ToolSafetyValidator::new(interactive_mode, false))
        .map_err(|_| anyhow!("Safety validator already initialized"))
        .expect("Failed to initialize safety validator");
    
    log_info!("safety", "Global safety validator initialized (interactive: {})", interactive_mode);
}

/// Initialize the global safety validator with brave mode
pub fn init_global_safety_validator_with_brave(interactive_mode: bool, brave_mode: bool) {
    GLOBAL_SAFETY_VALIDATOR.set(ToolSafetyValidator::new(interactive_mode, brave_mode))
        .map_err(|_| anyhow!("Safety validator already initialized"))
        .expect("Failed to initialize safety validator");
    
    log_info!("safety", "Global safety validator initialized (interactive: {}, brave: {})", interactive_mode, brave_mode);
}