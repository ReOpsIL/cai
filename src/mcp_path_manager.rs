use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use crate::logger::{log_debug, log_info, log_warn, log_error};
use crate::path_manager::get_path_manager;
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPathMapping {
    /// Host path (e.g., /Users/user/project)
    pub host_path: PathBuf,
    /// Container path (e.g., /workspace)
    pub container_path: PathBuf,
    /// Whether this mapping is read-only
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpWorkspaceConfig {
    /// Path mappings for Docker containers
    pub path_mappings: Vec<McpPathMapping>,
    /// Default workspace root in containers
    pub default_workspace: PathBuf,
    /// Project-specific workspace mappings
    pub project_mappings: HashMap<String, PathBuf>,
}

impl Default for McpWorkspaceConfig {
    fn default() -> Self {
        Self {
            path_mappings: vec![],
            default_workspace: PathBuf::from("/workspace"),
            project_mappings: HashMap::new(),
        }
    }
}

pub struct McpPathManager {
    workspace_config: McpWorkspaceConfig,
    current_mappings: HashMap<String, McpPathMapping>,
}

impl McpPathManager {
    pub fn new() -> Self {
        Self {
            workspace_config: McpWorkspaceConfig::default(),
            current_mappings: HashMap::new(),
        }
    }

    /// Initialize with current project context
    pub fn with_project_context(&mut self) -> Result<()> {
        let path_manager = get_path_manager();
        
        if let Some(project_ctx) = path_manager.get_project_context() {
            log_info!("mcp_path", "🔧 Configuring MCP paths for project: {}", project_ctx.project_name);
            
            // Create mapping for current project
            let mapping = McpPathMapping {
                host_path: project_ctx.project_root.clone(),
                container_path: self.workspace_config.default_workspace.clone(),
                read_only: false,
            };
            
            self.current_mappings.insert(project_ctx.project_name.clone(), mapping);
            self.workspace_config.project_mappings.insert(
                project_ctx.project_name.clone(), 
                self.workspace_config.default_workspace.clone()
            );
        } else {
            log_warn!("mcp_path", "⚠️ No project context found, using default workspace mapping");
            
            // Create default mapping for current directory
            let current_dir = std::env::current_dir()
                .context("Failed to get current directory")?;
            
            let mapping = McpPathMapping {
                host_path: current_dir,
                container_path: self.workspace_config.default_workspace.clone(),
                read_only: false,
            };
            
            self.current_mappings.insert("default".to_string(), mapping);
        }
        
        Ok(())
    }

    /// Transform a host path to container path
    pub fn host_to_container_path(&self, host_path: &Path) -> Result<PathBuf> {
        log_debug!("mcp_path", "🔄 Transforming host path: {}", host_path.display());
        
        // Try to find the best matching mapping
        for (name, mapping) in &self.current_mappings {
            if host_path.starts_with(&mapping.host_path) {
                let relative_path = host_path.strip_prefix(&mapping.host_path)
                    .context("Failed to strip host path prefix")?;
                let container_path = mapping.container_path.join(relative_path);
                
                log_debug!("mcp_path", "✅ Mapped using '{}': {} → {}", 
                          name, host_path.display(), container_path.display());
                return Ok(container_path);
            }
        }
        
        // Fallback: use default workspace and relative path
        let file_name = host_path.file_name()
            .context("Path has no file name component")?;
        let fallback_path = self.workspace_config.default_workspace.join(file_name);
        
        log_warn!("mcp_path", "⚠️ No mapping found, using fallback: {} → {}", 
                 host_path.display(), fallback_path.display());
        Ok(fallback_path)
    }

    /// Transform a container path back to host path
    pub fn container_to_host_path(&self, container_path: &Path) -> Result<PathBuf> {
        log_debug!("mcp_path", "🔄 Transforming container path: {}", container_path.display());
        
        // Try to find the mapping that contains this container path
        for (name, mapping) in &self.current_mappings {
            if container_path.starts_with(&mapping.container_path) {
                let relative_path = container_path.strip_prefix(&mapping.container_path)
                    .context("Failed to strip container path prefix")?;
                let host_path = mapping.host_path.join(relative_path);
                
                log_debug!("mcp_path", "✅ Mapped using '{}': {} → {}", 
                          name, container_path.display(), host_path.display());
                return Ok(host_path);
            }
        }
        
        anyhow::bail!("No mapping found for container path: {}", container_path.display())
    }

    /// Get Docker volume mount arguments for current mappings
    pub fn get_docker_volume_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        
        for mapping in self.current_mappings.values() {
            let mount_arg = if mapping.read_only {
                format!("{}:{}:ro", mapping.host_path.display(), mapping.container_path.display())
            } else {
                format!("{}:{}", mapping.host_path.display(), mapping.container_path.display())
            };
            args.push("-v".to_string());
            args.push(mount_arg);
        }
        
        log_debug!("mcp_path", "📋 Generated Docker volume args: {:?}", args);
        args
    }

    /// Update MCP configuration with current path mappings
    pub async fn update_mcp_config(&self, config_path: &Path) -> Result<()> {
        log_info!("mcp_path", "🔧 Updating MCP configuration with path mappings");
        
        // Read existing config
        let config_content = tokio::fs::read_to_string(config_path).await
            .context("Failed to read MCP config file")?;
        
        let mut config: serde_json::Value = serde_json::from_str(&config_content)
            .context("Failed to parse MCP config JSON")?;
        
        // Update filesystem server configuration
        if let Some(servers) = config.get_mut("mcpServers") {
            if let Some(filesystem) = servers.get_mut("filesystem") {
                if let Some(args) = filesystem.get_mut("args") {
                    if let Some(args_array) = args.as_array_mut() {
                        // Clear existing volume mounts (keep other args)
                        args_array.retain(|arg| {
                            if let Some(arg_str) = arg.as_str() {
                                !arg_str.starts_with("-v") && !arg_str.contains(':')
                            } else {
                                true
                            }
                        });
                        
                        // Add new volume mounts
                        let volume_args = self.get_docker_volume_args();
                        for arg in volume_args {
                            args_array.push(serde_json::Value::String(arg));
                        }
                        
                        log_info!("mcp_path", "✅ Updated filesystem server volume mounts");
                    }
                }
            }
        }
        
        // Write updated config
        let updated_config = serde_json::to_string_pretty(&config)
            .context("Failed to serialize updated config")?;
        
        tokio::fs::write(config_path, updated_config).await
            .context("Failed to write updated MCP config")?;
        
        log_info!("mcp_path", "✅ MCP configuration updated successfully");
        Ok(())
    }

    /// Add a custom path mapping
    pub fn add_path_mapping(&mut self, name: String, host_path: PathBuf, container_path: PathBuf, read_only: bool) {
        let mapping = McpPathMapping {
            host_path,
            container_path,
            read_only,
        };
        
        log_info!("mcp_path", "➕ Added path mapping '{}': {} → {} (ro: {})", 
                 name, mapping.host_path.display(), mapping.container_path.display(), read_only);
        
        self.current_mappings.insert(name, mapping);
    }

    /// Remove a path mapping
    pub fn remove_path_mapping(&mut self, name: &str) -> Option<McpPathMapping> {
        if let Some(mapping) = self.current_mappings.remove(name) {
            log_info!("mcp_path", "➖ Removed path mapping '{}'", name);
            Some(mapping)
        } else {
            log_warn!("mcp_path", "⚠️ Path mapping '{}' not found", name);
            None
        }
    }

    /// Get all current path mappings
    pub fn list_mappings(&self) -> &HashMap<String, McpPathMapping> {
        &self.current_mappings
    }

    /// Validate that all host paths in mappings exist
    pub fn validate_mappings(&self) -> Result<()> {
        for (name, mapping) in &self.current_mappings {
            if !mapping.host_path.exists() {
                log_error!("mcp_path", "❌ Host path for mapping '{}' does not exist: {}", 
                          name, mapping.host_path.display());
                anyhow::bail!("Host path for mapping '{}' does not exist: {}", name, mapping.host_path.display());
            }
        }
        
        log_info!("mcp_path", "✅ All path mappings validated successfully");
        Ok(())
    }

    /// Auto-configure mappings based on project structure
    pub async fn auto_configure_project_mappings(&mut self) -> Result<()> {
        log_info!("mcp_path", "🔍 Auto-configuring project path mappings");
        
        let path_manager = get_path_manager();
        if let Some(project_ctx) = path_manager.get_project_context() {
            // Clear existing mappings
            self.current_mappings.clear();
            
            // Add main project mapping
            self.add_path_mapping(
                project_ctx.project_name.clone(),
                project_ctx.project_root.clone(),
                PathBuf::from("/workspace"),
                false
            );
            
            // Look for common subdirectories and add them
            let common_dirs = ["src", "docs", "tests", "assets", "config"];
            for dir_name in &common_dirs {
                let dir_path = project_ctx.project_root.join(dir_name);
                if dir_path.exists() {
                    self.add_path_mapping(
                        format!("{}_{}", project_ctx.project_name, dir_name),
                        dir_path,
                        PathBuf::from("/workspace").join(dir_name),
                        false
                    );
                }
            }
            
            log_info!("mcp_path", "✅ Auto-configured {} path mappings for project '{}'", 
                     self.current_mappings.len(), project_ctx.project_name);
        }
        
        Ok(())
    }
}

// Global MCP path manager instance
static GLOBAL_MCP_PATH_MANAGER: Lazy<Mutex<McpPathManager>> = Lazy::new(|| {
    let mut manager = McpPathManager::new();
    if let Err(e) = manager.with_project_context() {
        log_warn!("mcp_path", "⚠️ Failed to initialize with project context: {}", e);
    }
    Mutex::new(manager)
});

/// Get the global MCP path manager instance
pub fn get_mcp_path_manager() -> std::sync::MutexGuard<'static, McpPathManager> {
    GLOBAL_MCP_PATH_MANAGER.lock().unwrap()
}

/// Initialize MCP path manager with current project context
pub fn initialize_mcp_path_manager() -> Result<()> {
    let mut manager = get_mcp_path_manager();
    manager.with_project_context()
        .context("Failed to initialize MCP path manager with project context")?;
    
    log_info!("mcp_path", "✅ MCP path manager initialized successfully");
    Ok(())
}

/// Transform host path to container path using global manager
pub fn transform_host_to_container(host_path: &Path) -> Result<PathBuf> {
    let manager = get_mcp_path_manager();
    manager.host_to_container_path(host_path)
}

/// Transform container path to host path using global manager
pub fn transform_container_to_host(container_path: &Path) -> Result<PathBuf> {
    let manager = get_mcp_path_manager();
    manager.container_to_host_path(container_path)
}

/// Update the MCP configuration file with current path mappings
pub async fn update_mcp_config_file() -> Result<()> {
    let config_path = PathBuf::from("mcp-config.json");
    let manager = get_mcp_path_manager();
    manager.update_mcp_config(&config_path).await
}

/// Auto-configure MCP path mappings for the current project
pub async fn auto_configure_mcp_paths() -> Result<()> {
    let mut manager = get_mcp_path_manager();
    manager.auto_configure_project_mappings().await
        .context("Failed to auto-configure MCP path mappings")?;
    
    // Update the MCP config file
    drop(manager); // Release lock before async operation
    update_mcp_config_file().await
        .context("Failed to update MCP config file with new mappings")?;
    
    log_info!("mcp_path", "✅ MCP paths auto-configured and config updated");
    Ok(())
}