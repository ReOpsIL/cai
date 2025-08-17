use crate::logger::{log_debug, log_info, log_warn};
use crate::path_manager::{ProjectContext, get_path_manager};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for managing chat session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub last_workflow_id: Option<String>,
    pub project_context: Option<ProjectContext>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            last_workflow_id: None,
            project_context: None,
            created_at: now,
            updated_at: now,
        }
    }
}

pub struct SessionManager {
    config_path: PathBuf,
    config: SessionConfig,
}

impl SessionManager {
    /// Create a new session manager with the default config path
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_config(&config_path)?;
        
        log_debug!("session", "📁 Session manager initialized with config path: {:?}", config_path);
        
        Ok(Self {
            config_path,
            config,
        })
    }

    /// Get the configuration file path (in user's home or current directory)
    fn get_config_path() -> Result<PathBuf> {
        // Try to use user's home directory first, fallback to current directory
        let config_dir = dirs::home_dir()
            .map(|home| home.join(".config").join("cai"))
            .unwrap_or_else(|| PathBuf::from("."));
        
        // Ensure the directory exists
        if let Some(parent) = config_dir.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        Ok(config_dir.join("session.json"))
    }

    /// Load session config from file, creating default if it doesn't exist
    fn load_config(config_path: &PathBuf) -> Result<SessionConfig> {
        if config_path.exists() {
            log_debug!("session", "📄 Loading existing session config from: {:?}", config_path);
            let content = fs::read_to_string(config_path)
                .context("Failed to read session config file")?;
            
            let config: SessionConfig = serde_json::from_str(&content)
                .context("Failed to parse session config JSON")?;
            
            log_info!("session", "✅ Loaded session config with last workflow: {:?}", config.last_workflow_id);
            Ok(config)
        } else {
            log_debug!("session", "📄 Creating new session config at: {:?}", config_path);
            let config = SessionConfig::default();
            
            // Save the default config
            Self::save_config_to_path(&config, config_path)?;
            
            Ok(config)
        }
    }

    /// Save current config to file
    fn save_config(&mut self) -> Result<()> {
        self.config.updated_at = chrono::Utc::now();
        Self::save_config_to_path(&self.config, &self.config_path)
    }

    /// Save config to specific path
    fn save_config_to_path(config: &SessionConfig, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        let content = serde_json::to_string_pretty(config)
            .context("Failed to serialize session config")?;
        
        fs::write(path, content)
            .context("Failed to write session config file")?;
        
        log_debug!("session", "💾 Saved session config to: {:?}", path);
        Ok(())
    }

    /// Get the last workflow ID, if any
    pub fn get_last_workflow_id(&self) -> Option<String> {
        log_debug!("session", "🔍 Getting last workflow ID: {:?}", self.config.last_workflow_id);
        self.config.last_workflow_id.clone()
    }

    /// Set the last workflow ID and save to disk
    pub fn set_last_workflow_id(&mut self, workflow_id: String) -> Result<()> {
        log_info!("session", "📝 Setting last workflow ID to: {}", workflow_id);
        self.config.last_workflow_id = Some(workflow_id);
        self.save_config()
            .context("Failed to save session config with new workflow ID")?;
        
        Ok(())
    }

    /// Clear the last workflow ID and save to disk
    pub fn clear_last_workflow_id(&mut self) -> Result<()> {
        log_info!("session", "🗑️ Clearing last workflow ID");
        self.config.last_workflow_id = None;
        self.save_config()
            .context("Failed to save session config after clearing workflow ID")?;
        
        Ok(())
    }

    /// Check if there is a stored workflow ID
    pub fn has_last_workflow(&self) -> bool {
        self.config.last_workflow_id.is_some()
    }

    /// Get the stored project context, if any
    pub fn get_project_context(&self) -> Option<&ProjectContext> {
        self.config.project_context.as_ref()
    }

    /// Set the project context and save to disk
    pub fn set_project_context(&mut self, project_context: ProjectContext) -> Result<()> {
        log_info!("session", "🗂️ Setting project context: {} at {}", 
                 project_context.project_name, project_context.project_root.display());
        
        // Also update the global path manager
        {
            let mut path_manager = get_path_manager();
            if let Err(e) = path_manager.set_project_context(
                project_context.project_root.clone(), 
                project_context.project_name.clone()
            ) {
                log_warn!("session", "⚠️ Failed to update global path manager: {}", e);
            }
        }

        self.config.project_context = Some(project_context);
        self.save_config()
            .context("Failed to save session config with new project context")?;
        
        Ok(())
    }

    /// Clear the project context and save to disk
    pub fn clear_project_context(&mut self) -> Result<()> {
        log_info!("session", "🗑️ Clearing project context");
        self.config.project_context = None;
        self.save_config()
            .context("Failed to save session config after clearing project context")?;
        
        Ok(())
    }

    /// Check if there is a stored project context
    pub fn has_project_context(&self) -> bool {
        self.config.project_context.is_some()
    }

    /// Restore project context to the global path manager
    pub fn restore_project_context(&self) -> Result<()> {
        if let Some(ref project_ctx) = self.config.project_context {
            log_info!("session", "🔄 Restoring project context: {}", project_ctx.project_name);
            
            let mut path_manager = get_path_manager();
            path_manager.set_project_context(
                project_ctx.project_root.clone(),
                project_ctx.project_name.clone()
            ).context("Failed to restore project context to path manager")?;
            
            log_info!("session", "✅ Project context restored successfully");
        } else {
            log_debug!("session", "No project context to restore");
        }
        
        Ok(())
    }

    /// Update project context from current path manager state
    pub fn sync_project_context(&mut self) -> Result<()> {
        let path_manager = get_path_manager();
        if let Some(current_ctx) = path_manager.get_project_context() {
            log_debug!("session", "🔄 Syncing project context from path manager");
            self.config.project_context = Some(current_ctx.clone());
            self.save_config()
                .context("Failed to save session config after syncing project context")?;
            log_debug!("session", "✅ Project context synced");
        }
        
        Ok(())
    }
}