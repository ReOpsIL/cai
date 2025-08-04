use anyhow::Result;
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::mcp_config::McpConfig;
use crate::mcp_client::McpClientManager;

/// Global MCP manager for application lifecycle management  
static GLOBAL_MCP_MANAGER: std::sync::LazyLock<Arc<Mutex<Option<McpClientManager>>>> = 
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global MCP manager and start all configured servers
pub async fn initialize_mcp() -> Result<()> {
    // Load MCP configuration
    let config = match McpConfig::load_default()? {
        Some(config) => {
            println!("📋 Loaded MCP configuration with {} server(s)", config.list_servers().len());
            config
        }
        None => {
            println!("📋 No MCP configuration found, creating default config...");
            let default_config = McpConfig::default();
            
            // Save default config
            let config_path = std::path::PathBuf::from("mcp-config.json");
            let config_json = serde_json::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, config_json)?;
            
            println!("✅ Created default MCP configuration at: {}", config_path.display());
            println!("💡 Edit this file to configure your MCP servers.");
            
            default_config
        }
    };

    // Create the manager
    let manager = McpClientManager::new(config);
    
    // Start all servers
    manager.start_all_servers().await?;
    
    // Store in global state
    let mut guard = GLOBAL_MCP_MANAGER.lock().await;
    *guard = Some(manager);
    
    Ok(())
}

/// Shutdown all MCP servers and cleanup global state
pub async fn shutdown_mcp() -> Result<()> {
    let mut guard = GLOBAL_MCP_MANAGER.lock().await;
    
    if let Some(manager) = guard.take() {
        println!("🛑 Shutting down all MCP servers...");
        
        let active_servers = manager.list_active_servers().await;
        if !active_servers.is_empty() {
            println!("🔄 Stopping {} active MCP server(s): {}", 
                     active_servers.len(), 
                     active_servers.join(", "));
            
            manager.shutdown_all().await?;
            println!("✅ All MCP servers shut down gracefully");
        } else {
            println!("ℹ️  No active MCP servers to shut down");
        }
    } else {
        println!("ℹ️  MCP manager was not initialized");
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