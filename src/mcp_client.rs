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
use crate::logger::{log_info, log_debug, log_warn, log_error, ops};

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
        
        log_error!("mcp","🚀 Starting {} MCP server(s)...", server_names.len());
        
        let mut started_count = 0;
        let mut failed_servers = Vec::new();

        for server_name in &server_names {
            match self.start_server(server_name).await {
                Ok(_) => {
                    log_error!("mcp","✅ Started MCP server: {}", server_name);
                    started_count += 1;
                }
                Err(e) => {
                    log_error!("mcp","❌ Failed to start MCP server '{}': {}", server_name, e);
                    failed_servers.push(server_name.clone());
                }
            }
        }

        if started_count > 0 {
            log_error!("mcp","🎉 Successfully started {}/{} MCP servers", started_count, server_names.len());
            
            // Perform health checks on started servers
            log_error!("mcp","🔍 Performing health checks on started servers...");
            self.perform_health_checks(&server_names, &failed_servers).await;
        }

        if !failed_servers.is_empty() {
            log_error!("mcp","⚠️  Failed to start servers: {}", failed_servers.join(", "));
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
            log_error!("mcp","Server '{}' is already running", server_name);
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

        log_error!("mcp","Successfully created and started MCP transport for: {}", server_name);
        
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
            log_error!("mcp","Stopped MCP server: {}", server_name);
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

    /// Get detailed tool information including parameters and descriptions
    pub async fn get_detailed_tools(&self, server_name: &str) -> Result<Vec<serde_json::Value>> {
        let active_clients = self.active_clients.lock().await;
        
        let instance = active_clients.get(server_name)
            .ok_or_else(|| anyhow!("Server '{}' is not running", server_name))?;

        // Use the actual MCP client to list tools with full details
        let tools_response = instance.client.list_all_tools().await
            .map_err(|e| anyhow!("Failed to list tools from server '{}': {}", server_name, e))?;

        // Convert tools to JSON values for easier handling
        let detailed_tools: Vec<serde_json::Value> = tools_response.iter()
            .map(|tool| serde_json::to_value(tool).unwrap_or(serde_json::Value::Null))
            .collect();

        Ok(detailed_tools)
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
                log_error!("mcp","Failed to stop server '{}': {}", server_name, e);
            }
        }

        Ok(())
    }

    /// Perform health checks on started servers by attempting to list tools
    async fn perform_health_checks(&self, all_servers: &[String], failed_servers: &[String]) {
        let mut healthy_count = 0;
        let mut unhealthy_servers = Vec::new();

        for server_name in all_servers {
            // Skip servers that failed to start
            if failed_servers.contains(server_name) {
                continue;
            }

            // Give the server a moment to fully initialize
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            match tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                self.list_tools(server_name)
            ).await {
                Ok(Ok(tools)) => {
                    log_error!("mcp","✅ Server '{}' is healthy ({} tools available)", server_name, tools.len());
                    healthy_count += 1;
                }
                Ok(Err(e)) => {
                    log_error!("mcp","⚠️  Server '{}' started but not responding: {}", server_name, e);
                    unhealthy_servers.push(server_name.clone());
                }
                Err(_) => {
                    log_error!("mcp","⚠️  Server '{}' health check timed out", server_name);
                    unhealthy_servers.push(server_name.clone());
                }
            }
        }

        let started_servers = all_servers.len() - failed_servers.len();
        if healthy_count == started_servers && started_servers > 0 {
            log_error!("mcp","🎉 All {} started servers are healthy and responding!", healthy_count);
        } else if healthy_count > 0 {
            log_error!("mcp","✅ {}/{} started servers are healthy", healthy_count, started_servers);
            if !unhealthy_servers.is_empty() {
                log_error!("mcp","⚠️  Unhealthy servers: {}", unhealthy_servers.join(", "));
            }
        } else if started_servers > 0 {
            log_error!("mcp","❌ None of the started servers are responding to health checks");
        }
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
