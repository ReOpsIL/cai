use anyhow::Result;
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::mcp_config::McpConfig;
use crate::mcp_client::McpClientManager;
use crate::logger::{log_info, log_debug, log_warn, log_error, ops};

/// Global MCP manager for application lifecycle management  
static GLOBAL_MCP_MANAGER: std::sync::LazyLock<Arc<Mutex<Option<McpClientManager>>>> = 
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global MCP manager and start configured servers if a config exists
pub async fn initialize_mcp() -> Result<()> {
    // Load MCP configuration if present; otherwise, do nothing
    let Some(config) = McpConfig::load_default()? else {
        return Ok(());
    };

    let manager = McpClientManager::new(config);
    manager.start_all_servers().await?;

    let mut guard = GLOBAL_MCP_MANAGER.lock().await;
    *guard = Some(manager);
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

/// Shutdown all MCP servers and cleanup global state
pub async fn shutdown_mcp() -> Result<()> {
    let mut guard = GLOBAL_MCP_MANAGER.lock().await;
    
    if let Some(manager) = guard.take() {
        log_error!("mcp","🛑 Shutting down all MCP servers...");
        
        let active_servers = manager.list_active_servers().await;
        if !active_servers.is_empty() {
            log_error!("mcp","🔄 Stopping {} active MCP server(s): {}",
                     active_servers.len(), 
                     active_servers.join(", "));
            
            manager.shutdown_all().await?;
            log_error!("mcp","✅ All MCP servers shut down gracefully");
        } else {
            log_error!("mcp","ℹ️  No active MCP servers to shut down");
        }
    } else {
        log_error!("mcp","ℹ️  MCP manager was not initialized");
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
