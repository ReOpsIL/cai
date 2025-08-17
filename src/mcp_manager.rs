use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::logger::{log_info, log_warn};
use crate::mcp_client::McpClientManager;
use crate::mcp_config::McpConfig;
use crate::docker_detection::{DockerDetector, initialize_docker_optimization};
use crate::mcp_path_manager::{initialize_mcp_path_manager, auto_configure_mcp_paths};

/// Global MCP manager for application lifecycle management  
static GLOBAL_MCP_MANAGER: std::sync::LazyLock<Arc<Mutex<Option<McpClientManager>>>> = 
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global MCP manager and start configured servers if a config exists
pub async fn initialize_mcp() -> Result<()> {
    log_info!("mcp", "🚀 Initializing MCP with Docker optimization and path management...");
    
    // Initialize MCP path manager first
    if let Err(e) = initialize_mcp_path_manager() {
        log_warn!("mcp", "⚠️ MCP path manager initialization failed: {}", e);
    }
    
    // Auto-configure project-specific path mappings
    if let Err(e) = auto_configure_mcp_paths().await {
        log_warn!("mcp", "⚠️ Auto-configuration of MCP paths failed: {}", e);
    }
    
    // Optimize Docker configuration before loading MCP config
    if let Err(e) = initialize_docker_optimization().await {
        log_warn!("mcp", "⚠️ Docker optimization failed (continuing with existing config): {}", e);
    }
    
    // Load MCP configuration if present; otherwise, do nothing
    let Some(config) = McpConfig::load_default()? else {
        log_info!("mcp", "ℹ️ No MCP config found, initialization complete");
        return Ok(());
    };

    log_info!("mcp", "⏳ Starting MCP servers (this may take a few seconds for Docker initialization)...");
    let manager = McpClientManager::new(config);
    manager.start_all_servers().await?;

    let mut guard = GLOBAL_MCP_MANAGER.lock().await;
    *guard = Some(manager);
    log_info!("mcp", "✅ MCP initialization completed successfully");
    Ok(())
}

/// Ensure the global MCP manager is initialized (no-op if already present)
pub async fn ensure_initialized() -> Result<()> {
    let need_init = {
        let guard = GLOBAL_MCP_MANAGER.lock().await;
        guard.is_none()
    };
    if need_init {
        initialize_mcp().await?;
    }
    Ok(())
}

/// Create a default MCP config file if none exists (does not start servers)
pub fn init_default_config_file() -> Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from("mcp-config.json");
    if path.exists() {
        return Ok(path);
    }
    let default_config = McpConfig::default();
    let config_json = serde_json::to_string_pretty(&default_config)?;
    std::fs::write(&path, config_json)?;
    Ok(path)
}

/// Create an optimized MCP config file using Docker detection
pub async fn init_optimized_config_file() -> Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from("mcp-config.json");
    
    log_info!("mcp", "🔧 Creating optimized MCP configuration...");
    
    // Use Docker detection to create optimal config
    let mut detector = DockerDetector::new()?;
    let _environment = detector.detect_environment().await?;
    
    // Apply the optimized configuration
    detector.apply_to_mcp_config(&path).await?;
    
    log_info!("mcp", "✅ Optimized MCP configuration created");
    Ok(path)
}

/// Shutdown all MCP servers and cleanup global state
pub async fn shutdown_mcp() -> Result<()> {
    let mut guard = GLOBAL_MCP_MANAGER.lock().await;
    
    if let Some(manager) = guard.take() {
        log_info!("mcp","🛑 Shutting down all MCP servers...");
        
        let active_servers = manager.list_active_servers().await;
        if !active_servers.is_empty() {
            log_info!("mcp","🔄 Stopping {} active MCP server(s): {}",
                     active_servers.len(), 
                     active_servers.join(", "));
            
            manager.shutdown_all().await?;
            log_info!("mcp","✅ All MCP servers shut down gracefully");
        } else {
            log_info!("mcp","ℹ️  No active MCP servers to shut down");
        }
    } else {
        log_info!("mcp","ℹ️  MCP manager was not initialized");
    }
    
    Ok(())
}

/// Get a reference to the global MCP manager for command operations
pub fn get_mcp_manager() -> Arc<Mutex<Option<McpClientManager>>> {
    GLOBAL_MCP_MANAGER.clone()
}

/// Execute an MCP command using the global manager
pub async fn execute_mcp_command<F, R>(operation: F) -> Result<R>
where
    F: FnOnce(&McpClientManager) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>>,
    R: Send,
{
    let global_manager = get_mcp_manager();
    
    let guard = global_manager.lock().await;
    let manager = guard.as_ref()
        .ok_or_else(|| anyhow::anyhow!("MCP manager not available"))?;
    
    operation(manager).await
}
