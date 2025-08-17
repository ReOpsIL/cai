use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use crate::logger::{log_debug, log_info, log_warn, log_error};
use crate::path_manager::{get_path_manager, set_global_project_context};
use std::env;
use std::fs;

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Try to continue with current directory
    UseCurrent,
    /// Create missing directories
    CreateMissing,
    /// Prompt user for correct path
    PromptUser,
    /// Auto-detect project from filesystem
    AutoDetect,
}

pub struct ProjectRecoveryManager {
    recovery_strategies: Vec<RecoveryStrategy>,
}

impl ProjectRecoveryManager {
    pub fn new() -> Self {
        Self {
            recovery_strategies: vec![
                RecoveryStrategy::AutoDetect,
                RecoveryStrategy::CreateMissing,
                RecoveryStrategy::UseCurrent,
            ],
        }
    }

    /// Attempt to recover from path resolution failures
    pub async fn recover_from_path_failure(
        &self, 
        failed_path: &Path, 
        operation: &str
    ) -> Result<PathBuf> {
        log_warn!("recovery", "🔧 Attempting recovery from path failure: {} (operation: {})", 
                 failed_path.display(), operation);

        for strategy in &self.recovery_strategies {
            if let Ok(recovered_path) = self.try_recovery_strategy(strategy, failed_path, operation).await {
                log_info!("recovery", "✅ Successfully recovered using strategy: {:?}", strategy);
                return Ok(recovered_path);
            }
        }

        anyhow::bail!("All recovery strategies failed for path: {}", failed_path.display())
    }

    async fn try_recovery_strategy(
        &self,
        strategy: &RecoveryStrategy,
        failed_path: &Path,
        operation: &str,
    ) -> Result<PathBuf> {
        match strategy {
            RecoveryStrategy::UseCurrent => {
                let current_dir = env::current_dir()
                    .context("Failed to get current directory")?;
                let relative_path = failed_path.file_name()
                    .map(|name| current_dir.join(name))
                    .unwrap_or(current_dir.clone());
                
                log_debug!("recovery", "💡 Using current directory approach: {}", relative_path.display());
                Ok(relative_path)
            },

            RecoveryStrategy::CreateMissing => {
                self.create_missing_directories(failed_path, operation).await
            },

            RecoveryStrategy::AutoDetect => {
                self.auto_detect_and_create_project(failed_path).await
            },

            RecoveryStrategy::PromptUser => {
                self.prompt_user_for_path(failed_path, operation).await
            },
        }
    }

    async fn create_missing_directories(&self, failed_path: &Path, operation: &str) -> Result<PathBuf> {
        log_info!("recovery", "📁 Attempting to create missing directories for: {}", failed_path.display());

        // If it's a file path, get the parent directory
        let target_dir = if operation.contains("file") || failed_path.extension().is_some() {
            failed_path.parent()
                .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory"))?
        } else {
            failed_path
        };

        // Create the directory structure
        fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create directory: {}", target_dir.display()))?;

        log_info!("recovery", "✅ Created directory structure: {}", target_dir.display());
        Ok(failed_path.to_path_buf())
    }

    async fn auto_detect_and_create_project(&self, failed_path: &Path) -> Result<PathBuf> {
        log_info!("recovery", "🔍 Auto-detecting project structure");

        // Look for project indicators in the current directory or parents
        let current_dir = env::current_dir()
            .context("Failed to get current directory")?;

        // Check if the failed path suggests a project structure
        if let Some(project_name) = self.extract_project_name(failed_path) {
            let project_root = current_dir.join(&project_name);
            
            log_info!("recovery", "🏗️ Creating project structure: {}", project_root.display());
            
            // Create the project root
            fs::create_dir_all(&project_root)
                .with_context(|| format!("Failed to create project root: {}", project_root.display()))?;

            // Update the global project context
            set_global_project_context(project_root.clone(), project_name.clone())
                .context("Failed to set global project context")?;

            // Resolve the failed path relative to the new project root
            let relative_to_project = failed_path.strip_prefix(&project_name)
                .unwrap_or(failed_path);
            let recovered_path = project_root.join(relative_to_project);

            // Create parent directories if needed
            if let Some(parent) = recovered_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create parent directories: {}", parent.display()))?;
            }

            log_info!("recovery", "✅ Project recovery successful: {}", recovered_path.display());
            return Ok(recovered_path);
        }

        anyhow::bail!("Could not auto-detect project structure from path: {}", failed_path.display())
    }

    async fn prompt_user_for_path(&self, failed_path: &Path, operation: &str) -> Result<PathBuf> {
        use std::io::{self, Write};
        
        println!("\n{} {} {}", "🔧".yellow(), "Path Recovery Required".bold(), "🔧".yellow());
        println!("Operation: {}", operation.cyan());
        println!("Failed path: {}", failed_path.display().to_string().yellow());
        println!();
        println!("Options:");
        println!("  1. Enter a new path");
        println!("  2. Use current directory");
        println!("  3. Create missing directories");
        println!("  4. Skip this operation");
        println!();
        print!("Your choice (1-4): ");
        
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .context("Failed to read user input")?;
        
        match input.trim() {
            "1" => {
                print!("Enter the correct path: ");
                io::stdout().flush().unwrap();
                
                let mut path_input = String::new();
                io::stdin().read_line(&mut path_input)
                    .context("Failed to read path input")?;
                
                let user_path = PathBuf::from(path_input.trim());
                log_info!("recovery", "User provided path: {}", user_path.display());
                Ok(user_path)
            },
            "2" => {
                let current_dir = env::current_dir()
                    .context("Failed to get current directory")?;
                Ok(current_dir)
            },
            "3" => {
                self.create_missing_directories(failed_path, operation).await
            },
            "4" | _ => {
                anyhow::bail!("User chose to skip operation")
            }
        }
    }

    fn extract_project_name(&self, path: &Path) -> Option<String> {
        // Look for common project patterns in the path
        let path_str = path.to_string_lossy();
        
        // Common project patterns
        let patterns = [
            "TaskFlow", "frontend", "backend", "src", "components",
            // Add more patterns as needed
        ];

        for pattern in &patterns {
            if path_str.contains(pattern) {
                return Some(pattern.to_string());
            }
        }

        // Try to extract the first directory component
        path.components().next()
            .and_then(|comp| comp.as_os_str().to_str())
            .map(|s| s.to_string())
    }

    /// Recover from MCP workspace path issues
    pub async fn recover_mcp_workspace(&self, failed_path: &Path) -> Result<PathBuf> {
        log_info!("recovery", "🐳 Attempting MCP workspace recovery for: {}", failed_path.display());

        // Try to map the path to a valid workspace path
        let workspace_root = PathBuf::from("/workspace");
        
        // Extract relative path components
        let relative_path = if failed_path.is_absolute() {
            // Strip leading path components until we find something meaningful
            failed_path.components()
                .skip_while(|comp| !matches!(comp.as_os_str().to_str(), Some(s) if s.len() > 1))
                .collect::<PathBuf>()
        } else {
            failed_path.to_path_buf()
        };

        let mapped_path = workspace_root.join(relative_path);
        
        // Create parent directories in workspace
        if let Some(parent) = mapped_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create workspace directories: {}", parent.display()))?;
        }

        log_info!("recovery", "✅ MCP workspace path mapped: {} → {}", 
                 failed_path.display(), mapped_path.display());
        
        Ok(mapped_path)
    }

    /// Validate and correct project context
    pub async fn validate_project_context(&self) -> Result<()> {
        let path_manager = get_path_manager();
        
        if let Some(project_ctx) = path_manager.get_project_context() {
            if !project_ctx.project_root.exists() {
                log_warn!("recovery", "⚠️ Project root does not exist: {}", project_ctx.project_root.display());
                
                // Try to create the project root
                fs::create_dir_all(&project_ctx.project_root)
                    .with_context(|| format!("Failed to create project root: {}", project_ctx.project_root.display()))?;
                
                log_info!("recovery", "✅ Created missing project root");
            }
        } else {
            log_info!("recovery", "🔍 No project context found, attempting auto-detection");
            
            // Try to auto-detect project
            drop(path_manager); // Release the lock
            let mut path_manager = get_path_manager();
            path_manager.auto_detect_project()
                .context("Failed to auto-detect project")?;
        }

        Ok(())
    }
}

// Global recovery manager instance
use std::sync::Mutex;
use once_cell::sync::Lazy;
use colored::*;

static GLOBAL_RECOVERY_MANAGER: Lazy<Mutex<ProjectRecoveryManager>> = Lazy::new(|| {
    Mutex::new(ProjectRecoveryManager::new())
});

/// Get the global recovery manager instance
pub fn get_recovery_manager() -> std::sync::MutexGuard<'static, ProjectRecoveryManager> {
    GLOBAL_RECOVERY_MANAGER.lock().unwrap()
}

/// Attempt to recover from a path operation failure
pub async fn recover_from_path_failure(failed_path: &Path, operation: &str) -> Result<PathBuf> {
    let recovery_manager = get_recovery_manager();
    recovery_manager.recover_from_path_failure(failed_path, operation).await
}

/// Validate and recover project context
pub async fn validate_and_recover_project_context() -> Result<()> {
    let recovery_manager = get_recovery_manager();
    recovery_manager.validate_project_context().await
}