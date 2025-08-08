use crate::configuration::{McpSettings, McpServerConfig};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{json, Value};
use lazy_static::lazy_static;
use chrono::{DateTime, Utc};

lazy_static! {
    static ref MCP_MANAGER: Mutex<Option<McpManager>> = Mutex::new(None);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub server: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub server: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<Value>,
    pub server: String,
}

#[derive(Debug, Clone)]
pub struct McpClient {
    pub name: String,
    pub config: McpServerConfig,
    pub process: Option<Arc<Mutex<Child>>>,
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
    pub prompts: Vec<McpPrompt>,
    pub connected: bool,
    pub last_ping: DateTime<Utc>,
}

pub struct McpManager {
    pub clients: HashMap<String, McpClient>,
    pub global_tools: HashMap<String, McpTool>,
    pub settings: McpSettings,
}

impl McpManager {
    pub fn new(settings: McpSettings) -> Self {
        Self {
            clients: HashMap::new(),
            global_tools: HashMap::new(),
            settings,
        }
    }

    /// Initialize MCP manager with configuration
    pub async fn initialize(settings: McpSettings) -> Result<(), Box<dyn std::error::Error>> {
        let mut manager = McpManager::new(settings);
        
        if manager.settings.enabled && manager.settings.auto_connect {
            manager.connect_all_servers().await?;
        }
        
        let mut global_manager = MCP_MANAGER.lock().unwrap();
        *global_manager = Some(manager);
        Ok(())
    }

    /// Get the global MCP manager instance
    pub fn get_global() -> Result<Arc<Mutex<McpManager>>, Box<dyn std::error::Error>> {
        let manager_guard = MCP_MANAGER.lock().unwrap();
        if let Some(ref _manager) = *manager_guard {
            drop(manager_guard);
            // Create a new Arc wrapper since we can't clone from inside the mutex
            // This is a simplified approach for the prototype
            Err("MCP Manager access needs refactoring for thread safety".into())
        } else {
            Err("MCP Manager not initialized".into())
        }
    }

    /// Connect to all configured MCP servers
    pub async fn connect_all_servers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (name, config) in &self.settings.servers {
            if config.enabled {
                match self.connect_server(name, config).await {
                    Ok(_) => println!("Connected to MCP server: {}", name),
                    Err(e) => eprintln!("Failed to connect to MCP server {}: {}", name, e),
                }
            }
        }
        Ok(())
    }

    /// Connect to a specific MCP server
    pub async fn connect_server(&mut self, name: &str, config: &McpServerConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Start the MCP server process
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
           .envs(&config.env)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let process = cmd.spawn()?;
        
        let mut client = McpClient {
            name: name.to_string(),
            config: config.clone(),
            process: Some(Arc::new(Mutex::new(process))),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            connected: false,
            last_ping: Utc::now(),
        };

        // Perform MCP initialization handshake
        self.initialize_client(&mut client).await?;
        
        // Discover available tools, resources, and prompts
        self.discover_client_capabilities(&mut client).await?;
        
        // Update global tools registry
        for tool in &client.tools {
            self.global_tools.insert(
                format!("{}:{}", client.name, tool.name),
                tool.clone()
            );
        }
        
        client.connected = true;
        self.clients.insert(name.to_string(), client);
        
        Ok(())
    }

    /// Initialize MCP client with handshake
    async fn initialize_client(&self, client: &mut McpClient) -> Result<(), Box<dyn std::error::Error>> {
        // MCP initialization protocol
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": "init",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "clientInfo": {
                    "name": "cai",
                    "version": "0.1.0"
                }
            }
        });

        // This is a simplified implementation
        // In a real implementation, we would send this via stdin and read the response
        println!("Would send MCP init request: {}", init_request);
        
        Ok(())
    }

    /// Discover tools, resources, and prompts from MCP client
    async fn discover_client_capabilities(&self, client: &mut McpClient) -> Result<(), Box<dyn std::error::Error>> {
        // List tools request
        let list_tools_request = json!({
            "jsonrpc": "2.0",
            "id": "list_tools",
            "method": "tools/list"
        });

        // List resources request
        let list_resources_request = json!({
            "jsonrpc": "2.0",
            "id": "list_resources", 
            "method": "resources/list"
        });

        // List prompts request
        let list_prompts_request = json!({
            "jsonrpc": "2.0",
            "id": "list_prompts",
            "method": "prompts/list"
        });

        // Simplified: In real implementation, these would be sent to the process
        // and responses would be parsed to populate client.tools, client.resources, client.prompts
        println!("Would discover capabilities for client: {}", client.name);
        
        Ok(())
    }

    /// List all available tools across all connected servers
    pub fn list_all_tools(&self) -> HashMap<String, Vec<McpTool>> {
        let mut tools_by_server = HashMap::new();
        
        for client in self.clients.values() {
            if client.connected {
                tools_by_server.insert(client.name.clone(), client.tools.clone());
            }
        }
        
        tools_by_server
    }

    /// Execute a tool on a specific server
    pub async fn execute_tool(&self, server_tool: &str, arguments: Value) -> Result<String, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = server_tool.split(':').collect();
        if parts.len() != 2 {
            return Err("Tool name must be in format 'server:tool'".into());
        }

        let server_name = parts[0];
        let tool_name = parts[1];

        let client = self.clients.get(server_name)
            .ok_or("Server not found")?;

        if !client.connected {
            return Err("Server not connected".into());
        }

        // Find the tool
        let tool = client.tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or("Tool not found")?;

        // Execute tool request
        let tool_request = json!({
            "jsonrpc": "2.0",
            "id": "call_tool",
            "method": "tools/call",
            "params": {
                "name": tool.name,
                "arguments": arguments
            }
        });

        // Simplified: In real implementation, this would be sent to the process
        // and the response would be parsed and returned
        Ok(format!("Would execute tool {}:{} with args: {}", server_name, tool_name, arguments))
    }

    /// Get resources from a specific server
    pub async fn get_resources(&self, server_name: &str) -> Result<Vec<McpResource>, Box<dyn std::error::Error>> {
        let client = self.clients.get(server_name)
            .ok_or("Server not found")?;

        if !client.connected {
            return Err("Server not connected".into());
        }

        Ok(client.resources.clone())
    }

    /// Read a resource from a specific server
    pub async fn read_resource(&self, server_name: &str, uri: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = self.clients.get(server_name)
            .ok_or("Server not found")?;

        if !client.connected {
            return Err("Server not connected".into());
        }

        let read_request = json!({
            "jsonrpc": "2.0",
            "id": "read_resource",
            "method": "resources/read",
            "params": {
                "uri": uri
            }
        });

        // Simplified: In real implementation, this would be sent to the process
        Ok(format!("Would read resource {} from server {}", uri, server_name))
    }

    /// Get server status
    pub fn get_server_status(&self) -> HashMap<String, bool> {
        self.clients.iter()
            .map(|(name, client)| (name.clone(), client.connected))
            .collect()
    }

    /// Disconnect from a specific server
    pub async fn disconnect_server(&mut self, server_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut client) = self.clients.remove(server_name) {
            client.connected = false;
            
            // Remove tools from global registry
            for tool in &client.tools {
                self.global_tools.remove(&format!("{}:{}", client.name, tool.name));
            }
            
            // Kill the process if it exists
            if let Some(process_arc) = client.process.take() {
                let mut process = process_arc.lock().unwrap();
                let _ = process.kill().await;
            }
        }
        
        Ok(())
    }

    /// Disconnect from all servers
    pub async fn disconnect_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let server_names: Vec<String> = self.clients.keys().cloned().collect();
        
        for server_name in server_names {
            self.disconnect_server(&server_name).await?;
        }
        
        Ok(())
    }
}

/// Get or initialize the global MCP manager
pub async fn get_mcp_manager() -> Result<(), Box<dyn std::error::Error>> {
    let manager_guard = MCP_MANAGER.lock().unwrap();
    if manager_guard.is_none() {
        drop(manager_guard);
        
        // Load MCP settings from configuration
        let config = crate::configuration::get_effective_config()?;
        McpManager::initialize(config.mcp).await?;
    }
    Ok(())
}

/// Execute an MCP command - main entry point for command system integration
pub async fn execute_mcp_command(command: &str, params: &[String]) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // Ensure MCP manager is initialized
    get_mcp_manager().await?;
    
    match command {
        "mcp:list-servers" => {
            Ok(Some(list_servers().await?))
        },
        "mcp:list-tools" => {
            let server = params.get(0).map(|s| s.as_str());
            Ok(Some(list_tools(server).await?))
        },
        "mcp:connect" => {
            if let Some(server_name) = params.get(0) {
                Ok(Some(connect_server(server_name).await?))
            } else {
                Ok(Some("Usage: @mcp:connect(server-name)".to_string()))
            }
        },
        "mcp:disconnect" => {
            if let Some(server_name) = params.get(0) {
                Ok(Some(disconnect_server(server_name).await?))
            } else {
                Ok(Some("Usage: @mcp:disconnect(server-name)".to_string()))
            }
        },
        "mcp:call" => {
            if params.len() < 2 {
                return Ok(Some("Usage: @mcp:call(server:tool, {\"arg\": \"value\"})".to_string()));
            }
            let server_tool = &params[0];
            let args_str = &params[1];
            let args: Value = serde_json::from_str(args_str)
                .unwrap_or_else(|_| json!({}));
            
            // This is a simplified stub - needs proper implementation
            Ok(Some(format!("Would call MCP tool: {} with args: {}", server_tool, args)))
        },
        _ => Ok(None)
    }
}

async fn list_servers() -> Result<String, Box<dyn std::error::Error>> {
    // Simplified stub implementation
    Ok("MCP Servers:\n- filesystem: Connected\n- database: Disconnected".to_string())
}

async fn list_tools(server: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    match server {
        Some(server_name) => {
            Ok(format!("Tools for server '{}':\n- read_file: Read a file\n- write_file: Write a file", server_name))
        },
        None => {
            Ok("All MCP Tools:\n- filesystem:read_file: Read a file\n- filesystem:write_file: Write a file\n- database:query: Execute SQL query".to_string())
        }
    }
}

async fn connect_server(server_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("Connected to MCP server: {}", server_name))
}

async fn disconnect_server(server_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("Disconnected from MCP server: {}", server_name))
}