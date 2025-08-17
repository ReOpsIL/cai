use std::path::{Path, PathBuf};
use std::env;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use crate::logger::{log_debug, log_info, log_warn, log_error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_root: PathBuf,
    pub project_name: String,
    pub current_working_dir: PathBuf,
    pub relative_paths: bool,
}

impl Default for ProjectContext {
    fn default() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            project_root: current_dir.clone(),
            project_name: "default".to_string(),
            current_working_dir: current_dir,
            relative_paths: true,
        }
    }
}

pub struct PathManager {
    project_context: Option<ProjectContext>,
    mcp_workspace_root: Option<PathBuf>,
}

impl PathManager {
    pub fn new() -> Self {
        Self {
            project_context: None,
            mcp_workspace_root: Self::detect_mcp_workspace_root(),
        }
    }

    /// Detect MCP workspace root from environment or configuration
    fn detect_mcp_workspace_root() -> Option<PathBuf> {
        // Check for Docker MCP workspace
        if let Ok(workspace) = env::var("MCP_WORKSPACE_ROOT") {
            return Some(PathBuf::from(workspace));
        }
        
        // Default Docker MCP workspace
        if Path::new("/workspace").exists() {
            return Some(PathBuf::from("/workspace"));
        }
        
        None
    }

    /// Set the current project context
    pub fn set_project_context(&mut self, project_root: PathBuf, project_name: String) -> Result<()> {
        log_info!("path_manager", "🗂️ Setting project context: {} at {}", project_name, project_root.display());
        
        // Validate project root exists or can be created
        if !project_root.exists() {
            log_warn!("path_manager", "⚠️ Project root does not exist: {}", project_root.display());
        }

        self.project_context = Some(ProjectContext {
            project_root: project_root.clone(),
            project_name,
            current_working_dir: project_root,
            relative_paths: true,
        });

        log_debug!("path_manager", "✅ Project context established");
        Ok(())
    }

    /// Resolve a path relative to the current project context
    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let input_path = path.as_ref();
        
        // If absolute path, use as-is (but validate if needed)
        if input_path.is_absolute() {
            return Ok(input_path.to_path_buf());
        }

        // Get project context or use current directory
        let base_dir = match &self.project_context {
            Some(ctx) => &ctx.current_working_dir,
            None => {
                log_warn!("path_manager", "⚠️ No project context set, using current directory");
                &env::current_dir().context("Failed to get current directory")?
            }
        };

        let resolved = base_dir.join(input_path);
        log_debug!("path_manager", "📁 Resolved '{}' → '{}'", input_path.display(), resolved.display());
        
        Ok(resolved)
    }

    /// Resolve path for MCP operations (handle Docker workspace mapping)
    pub fn resolve_mcp_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let resolved_path = self.resolve_path(path)?;
        
        // If we have MCP workspace mapping, translate paths
        if let Some(workspace_root) = &self.mcp_workspace_root {
            // Check if we need to map the path to MCP workspace
            if let Some(project_ctx) = &self.project_context {
                // Map project paths to workspace paths
                if resolved_path.starts_with(&project_ctx.project_root) {
                    let relative_to_project = resolved_path.strip_prefix(&project_ctx.project_root)
                        .context("Failed to get relative path")?;
                    
                    let mcp_path = workspace_root.join(relative_to_project);
                    log_debug!("path_manager", "🐳 MCP path mapping: '{}' → '{}'", 
                              resolved_path.display(), mcp_path.display());
                    return Ok(mcp_path);
                }
            }
        }

        Ok(resolved_path)
    }

    /// Get current project context
    pub fn get_project_context(&self) -> Option<&ProjectContext> {
        self.project_context.as_ref()
    }

    /// Check if a path is within the current project
    pub fn is_within_project<P: AsRef<Path>>(&self, path: P) -> bool {
        if let Some(ctx) = &self.project_context {
            let resolved = match self.resolve_path(path) {
                Ok(p) => p,
                Err(_) => return false,
            };
            resolved.starts_with(&ctx.project_root)
        } else {
            true // No project context means all paths are allowed
        }
    }

    /// Validate that a path operation is safe within project context
    pub fn validate_path_operation<P: AsRef<Path>>(&self, path: P, operation: &str) -> Result<PathBuf> {
        let resolved_path = self.resolve_path(path)?;
        
        // Check if within project bounds
        if !self.is_within_project(&resolved_path) {
            if let Some(ctx) = &self.project_context {
                anyhow::bail!(
                    "Path operation '{}' outside project bounds: '{}' not within '{}'",
                    operation,
                    resolved_path.display(),
                    ctx.project_root.display()
                );
            }
        }

        log_debug!("path_manager", "✅ Path operation '{}' validated for: {}", operation, resolved_path.display());
        Ok(resolved_path)
    }

    /// Auto-detect project root from current directory structure
    pub fn auto_detect_project(&mut self) -> Result<()> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        
        // Look for common project indicators
        let indicators = [
            "package.json",
            "Cargo.toml", 
            ".git",
            "README.md",
            "pyproject.toml",
            "composer.json"
        ];

        let mut search_dir = current_dir.clone();
        loop {
            for indicator in &indicators {
                if search_dir.join(indicator).exists() {
                    let project_name = search_dir.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("detected_project")
                        .to_string();
                    
                    log_info!("path_manager", "🔍 Auto-detected project: {} ({})", project_name, indicator);
                    return self.set_project_context(search_dir, project_name);
                }
            }
            
            // Move up one directory
            if let Some(parent) = search_dir.parent() {
                search_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        log_warn!("path_manager", "⚠️ No project indicators found, using current directory");
        let project_name = current_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("current_dir")
            .to_string();
            
        self.set_project_context(current_dir, project_name)
    }

    /// Create a relative path from project root
    pub fn make_relative<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        
        if let Some(ctx) = &self.project_context {
            if path.starts_with(&ctx.project_root) {
                let relative = path.strip_prefix(&ctx.project_root)
                    .context("Failed to create relative path")?;
                return Ok(relative.to_path_buf());
            }
        }
        
        // If not within project or no context, return as-is
        Ok(path.to_path_buf())
    }

    /// Ensure directory exists, creating it if necessary
    pub fn ensure_directory<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let resolved_path = self.validate_path_operation(path, "create_directory")?;
        
        if !resolved_path.exists() {
            std::fs::create_dir_all(&resolved_path)
                .with_context(|| format!("Failed to create directory: {}", resolved_path.display()))?;
            log_info!("path_manager", "📁 Created directory: {}", resolved_path.display());
        }
        
        Ok(resolved_path)
    }
}

// Global path manager instance
use std::sync::Mutex;
use once_cell::sync::Lazy;

static GLOBAL_PATH_MANAGER: Lazy<Mutex<PathManager>> = Lazy::new(|| {
    Mutex::new(PathManager::new())
});

/// Get the global path manager instance
pub fn get_path_manager() -> std::sync::MutexGuard<'static, PathManager> {
    GLOBAL_PATH_MANAGER.lock().unwrap()
}

/// Initialize path manager with auto-detection
pub fn initialize_path_manager() -> Result<()> {
    log_info!("path_manager", "🚀 Initializing path manager");
    let mut manager = get_path_manager();
    manager.auto_detect_project()?;
    log_info!("path_manager", "✅ Path manager initialized");
    Ok(())
}

/// Set project context globally
pub fn set_global_project_context(project_root: PathBuf, project_name: String) -> Result<()> {
    let mut manager = get_path_manager();
    manager.set_project_context(project_root, project_name)
}

/// Resolve path using global path manager
pub fn resolve_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let manager = get_path_manager();
    manager.resolve_path(path)
}

/// Resolve MCP path using global path manager
pub fn resolve_mcp_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let manager = get_path_manager();
    manager.resolve_mcp_path(path)
}