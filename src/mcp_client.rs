use anyhow::{Result, anyhow};
use rmcp::{
    transport::{ConfigureCommandExt, TokioChildProcess},
    service::{ServiceExt, RunningService},
    model::{CallToolRequestParam, ReadResourceRequestParam},
    RoleClient,
};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::process::Command as TokioCommand;

use crate::mcp_config::McpConfig;

pub struct McpClientManager {
    config: McpConfig,
    active_clients: Arc<Mutex<HashMap<String, McpClientInstance>>>,
}

// Store the actual service client
struct McpClientInstance {
    server_name: String,
    client: RunningService<RoleClient, ()>,
}

impl McpClientManager {
    pub fn new(config: McpConfig) -> Self {
        Self {
            config,
            active_clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start all configured MCP servers
    pub async fn start_all_servers(&self) -> Result<()> {
        let server_names: Vec<String> = self.config.list_servers().into_iter().cloned().collect();
        
        println!("🚀 Starting {} MCP server(s)...", server_names.len());
        
        let mut started_count = 0;
        let mut failed_servers = Vec::new();

        for server_name in &server_names {
            match self.start_server(server_name).await {
                Ok(_) => {
                    println!("✅ Started MCP server: {}", server_name);
                    started_count += 1;
                }
                Err(e) => {
                    eprintln!("❌ Failed to start MCP server '{}': {}", server_name, e);
                    failed_servers.push(server_name.clone());
                }
            }
        }

        if started_count > 0 {
            println!("🎉 Successfully started {}/{} MCP servers", started_count, server_names.len());
        }

        if !failed_servers.is_empty() {
            eprintln!("⚠️  Failed to start servers: {}", failed_servers.join(", "));
            // Don't fail the entire application for server startup failures
        }

        Ok(())
    }

    pub async fn start_server(&self, server_name: &str) -> Result<()> {
        let server_config = self.config.get_server(server_name)
            .ok_or_else(|| anyhow!("Server '{}' not found in configuration", server_name))?;

        let mut active_clients = self.active_clients.lock().await;
        
        // Check if server is already running
        if active_clients.contains_key(server_name) {
            println!("Server '{}' is already running", server_name);
            return Ok(());
        }

        // Create command and transport in one go using the configure pattern
        let transport = TokioChildProcess::new(
            TokioCommand::new(&server_config.command).configure(|cmd| {
                cmd.args(&server_config.args);
                for (key, value) in &server_config.env {
                    cmd.env(key, value);
                }
                if let Some(cwd) = &server_config.cwd {
                    cmd.current_dir(cwd);
                }
            })
        ).map_err(|e| anyhow!("Failed to create transport: {}", e))?;

        // Create the service client using the transport
        let client = ().serve(transport).await
            .map_err(|e| anyhow!("Failed to create MCP client service: {}", e))?;

        println!("Successfully created and started MCP transport for: {}", server_name);
        
        let instance = McpClientInstance {
            server_name: server_name.to_string(),
            client,
        };

        active_clients.insert(server_name.to_string(), instance);
        
        Ok(())
    }

    pub async fn stop_server(&self, server_name: &str) -> Result<()> {
        let mut active_clients = self.active_clients.lock().await;
        
        if let Some(_instance) = active_clients.remove(server_name) {
            println!("Stopped MCP server: {}", server_name);
        }
        
        Ok(())
    }

    pub async fn list_tools(&self, server_name: &str) -> Result<Vec<String>> {
        let active_clients = self.active_clients.lock().await;
        
        let instance = active_clients.get(server_name)
            .ok_or_else(|| anyhow!("Server '{}' is not running", server_name))?;

        // Use the actual MCP client to list tools
        let tools_response = instance.client.list_all_tools().await
            .map_err(|e| anyhow!("Failed to list tools from server '{}': {}", server_name, e))?;

        // Extract tool names from the response
        let tool_names = tools_response.iter()
            .map(|tool| tool.name.to_string())
            .collect();

        Ok(tool_names)
    }

    pub async fn call_tool(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<Value> {
        let active_clients = self.active_clients.lock().await;
        
        let instance = active_clients.get(server_name)
            .ok_or_else(|| anyhow!("Server '{}' is not running", server_name))?;

        // Convert JSON Value to Map for arguments
        let arguments_map = arguments.as_object().cloned();

        // Use the actual MCP client to call the tool
        let call_param = CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: arguments_map,
        };

        let tool_response = instance.client.call_tool(call_param).await
            .map_err(|e| anyhow!("Failed to call tool '{}' on server '{}': {}", tool_name, server_name, e))?;

        // Convert the response to JSON Value
        Ok(serde_json::to_value(tool_response)
            .map_err(|e| anyhow!("Failed to serialize tool response: {}", e))?)
    }

    pub async fn list_resources(&self, server_name: &str) -> Result<Vec<String>> {
        let active_clients = self.active_clients.lock().await;
        
        let instance = active_clients.get(server_name)
            .ok_or_else(|| anyhow!("Server '{}' is not running", server_name))?;

        // Use the actual MCP client to list resources
        let resources_response = instance.client.list_all_resources().await
            .map_err(|e| anyhow!("Failed to list resources from server '{}': {}", server_name, e))?;

        // Extract resource URIs from the response
        let resource_uris = resources_response.iter()
            .map(|resource| resource.uri.clone())
            .collect();

        Ok(resource_uris)
    }

    pub async fn read_resource(&self, server_name: &str, uri: &str) -> Result<Value> {
        let active_clients = self.active_clients.lock().await;
        
        let instance = active_clients.get(server_name)
            .ok_or_else(|| anyhow!("Server '{}' is not running", server_name))?;

        // Use the actual MCP client to read the resource
        let read_param = ReadResourceRequestParam {
            uri: uri.to_string(),
        };

        let resource_response = instance.client.read_resource(read_param).await
            .map_err(|e| anyhow!("Failed to read resource '{}' from server '{}': {}", uri, server_name, e))?;

        // Convert the response to JSON Value
        Ok(serde_json::to_value(resource_response)
            .map_err(|e| anyhow!("Failed to serialize resource response: {}", e))?)
    }

    pub async fn list_active_servers(&self) -> Vec<String> {
        let active_clients = self.active_clients.lock().await;
        active_clients.keys().cloned().collect()
    }

    pub fn list_configured_servers(&self) -> Vec<&String> {
        self.config.list_servers()
    }

    pub async fn shutdown_all(&self) -> Result<()> {
        let server_names: Vec<String> = {
            let active_clients = self.active_clients.lock().await;
            active_clients.keys().cloned().collect()
        };

        for server_name in server_names {
            if let Err(e) = self.stop_server(&server_name).await {
                eprintln!("Failed to stop server '{}': {}", server_name, e);
            }
        }

        Ok(())
    }
}

impl Drop for McpClientManager {
    fn drop(&mut self) {
        // Note: We can't use async in Drop, so we'll need to handle cleanup differently
        // The processes should be killed when the Child structs are dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_config::McpConfig;

    #[tokio::test]
    async fn test_client_manager_creation() {
        let config = McpConfig::default();
        let manager = McpClientManager::new(config);
        
        let configured_servers = manager.list_configured_servers();
        assert!(!configured_servers.is_empty());
    }

    #[tokio::test]
    async fn test_list_active_servers_empty() {
        let config = McpConfig::default();
        let manager = McpClientManager::new(config);
        
        let active_servers = manager.list_active_servers().await;
        assert!(active_servers.is_empty());
    }
}