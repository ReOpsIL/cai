use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use crate::logger::{log_debug, log_error, log_info, log_warn};
use crate::path_manager::get_path_manager;
use crate::project_state_manager::{get_project_state_manager, ExecutionResult, ExecutionStatus};
use crate::tool_safety::{get_global_safety_validator, OperationType};

/// Parameters for shell command execution (equivalent to gemini-cli's ShellToolParams)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellParams {
    pub command: String,
    pub description: Option<String>,
    pub directory: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
}

/// Result of shell command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    pub success: bool,
    pub command: String,
    pub directory: PathBuf,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub execution_time: f64,
    pub background_pids: Vec<u32>,
}

/// Shell command allowlist for approved commands
#[derive(Debug, Clone)]
pub struct CommandAllowlist {
    allowed_commands: Arc<Mutex<HashSet<String>>>,
    allowed_patterns: Arc<Mutex<HashSet<String>>>,
}

impl CommandAllowlist {
    pub fn new() -> Self {
        let mut allowed_commands = HashSet::new();
        let mut allowed_patterns = HashSet::new();

        // Add common safe development commands
        allowed_commands.insert("ls".to_string());
        allowed_commands.insert("pwd".to_string());
        allowed_commands.insert("echo".to_string());
        allowed_commands.insert("cat".to_string());
        allowed_commands.insert("head".to_string());
        allowed_commands.insert("tail".to_string());
        allowed_commands.insert("grep".to_string());
        allowed_commands.insert("find".to_string());
        allowed_commands.insert("wc".to_string());
        allowed_commands.insert("sort".to_string());
        allowed_commands.insert("uniq".to_string());

        // Development tools
        allowed_commands.insert("npm".to_string());
        allowed_commands.insert("node".to_string());
        allowed_commands.insert("yarn".to_string());
        allowed_commands.insert("pnpm".to_string());
        allowed_commands.insert("cargo".to_string());
        allowed_commands.insert("rustc".to_string());
        allowed_commands.insert("git".to_string());
        allowed_commands.insert("python".to_string());
        allowed_commands.insert("python3".to_string());
        allowed_commands.insert("pip".to_string());
        allowed_commands.insert("pip3".to_string());
        allowed_commands.insert("go".to_string());
        allowed_commands.insert("java".to_string());
        allowed_commands.insert("javac".to_string());
        allowed_commands.insert("mvn".to_string());
        allowed_commands.insert("gradle".to_string());

        // Build and test tools
        allowed_commands.insert("make".to_string());
        allowed_commands.insert("cmake".to_string());
        allowed_commands.insert("docker".to_string());
        allowed_commands.insert("docker-compose".to_string());

        // Add safe patterns
        allowed_patterns.insert("npm run *".to_string());
        allowed_patterns.insert("cargo *".to_string());
        allowed_patterns.insert("git *".to_string());
        allowed_patterns.insert("node *".to_string());
        allowed_patterns.insert("python *".to_string());

        Self {
            allowed_commands: Arc::new(Mutex::new(allowed_commands)),
            allowed_patterns: Arc::new(Mutex::new(allowed_patterns)),
        }
    }

    pub async fn is_command_allowed(&self, command: &str) -> bool {
        let command_root = self.extract_command_root(command);
        
        // Check exact command match
        let allowed_commands = self.allowed_commands.lock().await;
        if allowed_commands.contains(&command_root) {
            return true;
        }
        drop(allowed_commands);

        // Check pattern match
        let allowed_patterns = self.allowed_patterns.lock().await;
        for pattern in allowed_patterns.iter() {
            if self.matches_pattern(command, pattern) {
                return true;
            }
        }

        false
    }

    pub async fn allow_command(&self, command: &str) {
        let command_root = self.extract_command_root(command);
        let mut allowed_commands = self.allowed_commands.lock().await;
        allowed_commands.insert(command_root);
    }

    pub async fn allow_pattern(&self, pattern: &str) {
        let mut allowed_patterns = self.allowed_patterns.lock().await;
        allowed_patterns.insert(pattern.to_string());
    }

    fn extract_command_root(&self, command: &str) -> String {
        command.trim()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    }

    fn matches_pattern(&self, command: &str, pattern: &str) -> bool {
        if pattern.ends_with(" *") {
            let prefix = &pattern[..pattern.len() - 2];
            command.starts_with(prefix)
        } else {
            command == pattern
        }
    }
}

/// Shell execution manager with safety and monitoring capabilities
pub struct ShellExecutionManager {
    allowlist: CommandAllowlist,
}

impl ShellExecutionManager {
    pub fn new() -> Self {
        Self {
            allowlist: CommandAllowlist::new(),
        }
    }

    /// Execute shell command with comprehensive safety checks and monitoring
    pub async fn execute_command(&self, params: ShellParams) -> Result<ShellResult> {
        log_info!("shell_exec", "🔧 Executing shell command: {}", params.command);

        // Validate command safety
        self.validate_command(&params).await?;

        // Check permissions with safety validator
        let safety_validator = get_global_safety_validator();
        if !safety_validator.is_operation_allowed(&OperationType::ShellCommand(params.command.clone())).await {
            anyhow::bail!("Shell command not permitted by safety validator");
        }

        // Resolve working directory
        let working_dir = self.resolve_working_directory(&params.directory).await?;

        // Start timing
        let start_time = std::time::Instant::now();

        // Execute command with timeout
        let timeout_duration = Duration::from_secs(params.timeout_seconds.unwrap_or(300)); // Default 5 minutes
        let result = timeout(timeout_duration, self.execute_command_internal(&params, &working_dir)).await
            .context("Command execution timed out")?
            .context("Command execution failed")?;

        let execution_time = start_time.elapsed().as_secs_f64();

        // Record execution in project state
        self.record_execution(&params, &result, execution_time).await?;

        log_info!("shell_exec", "✅ Command completed: {} ({}s)", params.command, execution_time);
        Ok(result)
    }

    async fn validate_command(&self, params: &ShellParams) -> Result<()> {
        // Check if command is empty
        if params.command.trim().is_empty() {
            anyhow::bail!("Command cannot be empty");
        }

        // Check command allowlist
        if !self.allowlist.is_command_allowed(&params.command).await {
            anyhow::bail!("Command '{}' is not in the allowlist. Use workflow permissions to approve development commands.", params.command);
        }

        // Validate directory if specified
        if let Some(ref dir) = params.directory {
            if dir.is_absolute() {
                anyhow::bail!("Directory must be relative to project root");
            }
        }

        // Check for dangerous patterns
        self.check_dangerous_patterns(&params.command)?;

        Ok(())
    }

    fn check_dangerous_patterns(&self, command: &str) -> Result<()> {
        let dangerous_patterns = [
            "rm -rf",
            "sudo rm",
            "format",
            "fdisk",
            "mkfs",
            "dd if=",
            ":(){ :|:& };:",  // Fork bomb
            "chmod 777",
            "chown -R",
            "passwd",
            "su -",
            "sudo su",
        ];

        for pattern in &dangerous_patterns {
            if command.contains(pattern) {
                anyhow::bail!("Command contains dangerous pattern: {}", pattern);
            }
        }

        Ok(())
    }

    async fn resolve_working_directory(&self, directory: &Option<PathBuf>) -> Result<PathBuf> {
        let path_manager = get_path_manager();
        
        match directory {
            Some(dir) => {
                let resolved = path_manager.resolve_path(dir)
                    .context("Failed to resolve command directory")?;
                
                if !resolved.exists() {
                    anyhow::bail!("Directory does not exist: {}", resolved.display());
                }
                
                if !resolved.is_dir() {
                    anyhow::bail!("Path is not a directory: {}", resolved.display());
                }
                
                Ok(resolved)
            }
            None => {
                // Use current working directory from path manager
                let current_context = path_manager.get_project_context();
                match current_context {
                    Some(ctx) => Ok(ctx.current_working_dir.clone()),
                    None => Ok(std::env::current_dir().context("Failed to get current directory")?),
                }
            }
        }
    }

    async fn execute_command_internal(&self, params: &ShellParams, working_dir: &Path) -> Result<ShellResult> {
        log_debug!("shell_exec", "📁 Working directory: {}", working_dir.display());

        let mut cmd = TokioCommand::new("bash");
        cmd.arg("-c")
           .arg(&params.command)
           .current_dir(working_dir)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .stdin(Stdio::null());

        let mut child = cmd.spawn()
            .context("Failed to spawn command")?;

        // Capture stdout and stderr concurrently
        let stdout_handle = {
            let stdout = child.stdout.take().context("Failed to take stdout")?;
            let reader = BufReader::new(stdout);
            tokio::spawn(async move {
                let mut lines = reader.lines();
                let mut output = String::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            })
        };

        let stderr_handle = {
            let stderr = child.stderr.take().context("Failed to take stderr")?;
            let reader = BufReader::new(stderr);
            tokio::spawn(async move {
                let mut lines = reader.lines();
                let mut output = String::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            })
        };

        // Wait for command completion
        let exit_status = child.wait().await
            .context("Failed to wait for command completion")?;

        // Collect outputs
        let stdout = stdout_handle.await
            .context("Failed to join stdout task")?;
        let stderr = stderr_handle.await
            .context("Failed to join stderr task")?;

        let success = exit_status.success();
        let exit_code = exit_status.code();

        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            exit_status.signal().map(|s| format!("Signal {}", s))
        };

        #[cfg(not(unix))]
        let signal = None;

        Ok(ShellResult {
            success,
            command: params.command.clone(),
            directory: working_dir.to_path_buf(),
            stdout: stdout.trim_end().to_string(),
            stderr: stderr.trim_end().to_string(),
            exit_code,
            signal,
            execution_time: 0.0, // Will be set by caller
            background_pids: vec![], // TODO: Implement background process detection
        })
    }

    async fn record_execution(&self, params: &ShellParams, result: &ShellResult, execution_time: f64) -> Result<()> {
        let mut project_state = get_project_state_manager();

        let execution_result = ExecutionResult {
            command: params.command.clone(),
            working_directory: result.directory.clone(),
            exit_code: result.exit_code,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
            execution_time_seconds: execution_time,
            timestamp: chrono::Utc::now(),
            status: if result.success {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed
            },
        };

        project_state.record_execution(execution_result).await
            .context("Failed to record command execution")?;

        Ok(())
    }

    /// Approve a command for future execution without prompting
    pub async fn approve_command(&self, command: &str) {
        log_info!("shell_exec", "✅ Approving command for future use: {}", command);
        self.allowlist.allow_command(command).await;
    }

    /// Approve a command pattern for future execution without prompting
    pub async fn approve_pattern(&self, pattern: &str) {
        log_info!("shell_exec", "✅ Approving command pattern for future use: {}", pattern);
        self.allowlist.allow_pattern(pattern).await;
    }

    /// Get list of commonly used development commands for workflow approval
    pub fn get_development_commands() -> Vec<&'static str> {
        vec![
            "npm install",
            "npm run build",
            "npm run test",
            "npm run dev",
            "npm run start",
            "yarn install",
            "yarn build",
            "yarn test",
            "cargo build",
            "cargo test",
            "cargo run",
            "cargo check",
            "git add .",
            "git commit -m",
            "git push",
            "git pull",
            "git status",
            "python -m pip install",
            "python setup.py",
            "make",
            "make test",
            "make build",
            "docker build",
            "docker run",
        ]
    }

    /// Approve common development workflow commands
    pub async fn approve_development_workflow(&self) {
        log_info!("shell_exec", "🚀 Approving common development workflow commands");
        
        for command in Self::get_development_commands() {
            self.allowlist.allow_command(command).await;
        }

        // Add development patterns
        let patterns = vec![
            "npm run *",
            "yarn *",
            "cargo *",
            "git *",
            "python *",
            "node *",
            "make *",
        ];

        for pattern in patterns {
            self.allowlist.allow_pattern(pattern).await;
        }

        log_info!("shell_exec", "✅ Development workflow commands approved");
    }
}

/// Global singleton instance
static SHELL_EXECUTION_MANAGER: once_cell::sync::Lazy<std::sync::Arc<ShellExecutionManager>> = 
    once_cell::sync::Lazy::new(|| {
        std::sync::Arc::new(ShellExecutionManager::new())
    });

/// Get global shell execution manager instance
pub fn get_shell_execution_manager() -> std::sync::Arc<ShellExecutionManager> {
    SHELL_EXECUTION_MANAGER.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_allowlist() {
        let allowlist = CommandAllowlist::new();

        // Test allowed commands
        assert!(allowlist.is_command_allowed("ls -la").await);
        assert!(allowlist.is_command_allowed("git status").await);
        assert!(allowlist.is_command_allowed("npm install").await);

        // Test disallowed commands
        assert!(!allowlist.is_command_allowed("rm -rf /").await);
        assert!(!allowlist.is_command_allowed("sudo rm -rf").await);

        // Test adding new command
        allowlist.allow_command("custom-tool").await;
        assert!(allowlist.is_command_allowed("custom-tool --help").await);
    }

    #[test]
    fn test_dangerous_pattern_detection() {
        let manager = ShellExecutionManager::new();

        // Test dangerous patterns
        assert!(manager.check_dangerous_patterns("rm -rf /").is_err());
        assert!(manager.check_dangerous_patterns("sudo rm something").is_err());
        assert!(manager.check_dangerous_patterns("chmod 777 .").is_err());

        // Test safe commands
        assert!(manager.check_dangerous_patterns("ls -la").is_ok());
        assert!(manager.check_dangerous_patterns("npm install").is_ok());
        assert!(manager.check_dangerous_patterns("git status").is_ok());
    }
}