use crate::commands_registry::{Command, CommandType, register_command};
use crate::mcp_client;
use regex::Regex;
use serde_json::Value;

/// Register all MCP-related commands
pub fn register_mcp_commands() {
    // List MCP servers command
    register_command(Command {
        name: "mcp-list-servers".to_string(),
        pattern: Regex::new(r"@mcp:list-servers\(\s*\)").unwrap(),
        description: "List all MCP servers and their connection status".to_string(),
        usage_example: "@mcp:list-servers()".to_string(),
        handler: |_params| {
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:list-servers", &[]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some("No MCP servers configured".to_string())),
                    Err(e) => Ok(Some(format!("Error listing MCP servers: {}", e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // List MCP tools command
    register_command(Command {
        name: "mcp-list-tools".to_string(),
        pattern: Regex::new(r"@mcp:list-tools\(\s*\)").unwrap(),
        description: "List all available MCP tools across all servers".to_string(),
        usage_example: "@mcp:list-tools()".to_string(),
        handler: |_params| {
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:list-tools", &[]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some("No MCP tools available".to_string())),
                    Err(e) => Ok(Some(format!("Error listing MCP tools: {}", e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // List tools for specific server
    register_command(Command {
        name: "mcp-list-server-tools".to_string(),
        pattern: Regex::new(r"@mcp:list-tools\(\s*(\S+)\s*\)").unwrap(),
        description: "List MCP tools for a specific server".to_string(),
        usage_example: "@mcp:list-tools(server-name)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @mcp:list-tools(server-name)".to_string()));
            }
            
            let server_name = &params[0];
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:list-tools", &[server_name.to_string()]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some(format!("No tools found for server: {}", server_name))),
                    Err(e) => Ok(Some(format!("Error listing tools for server {}: {}", server_name, e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Connect to MCP server
    register_command(Command {
        name: "mcp-connect".to_string(),
        pattern: Regex::new(r"@mcp:connect\(\s*(\S+)\s*\)").unwrap(),
        description: "Connect to an MCP server".to_string(),
        usage_example: "@mcp:connect(server-name)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @mcp:connect(server-name)".to_string()));
            }
            
            let server_name = &params[0];
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:connect", &[server_name.to_string()]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some(format!("Failed to connect to server: {}", server_name))),
                    Err(e) => Ok(Some(format!("Error connecting to server {}: {}", server_name, e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Disconnect from MCP server
    register_command(Command {
        name: "mcp-disconnect".to_string(),
        pattern: Regex::new(r"@mcp:disconnect\(\s*(\S+)\s*\)").unwrap(),
        description: "Disconnect from an MCP server".to_string(),
        usage_example: "@mcp:disconnect(server-name)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @mcp:disconnect(server-name)".to_string()));
            }
            
            let server_name = &params[0];
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:disconnect", &[server_name.to_string()]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some(format!("Failed to disconnect from server: {}", server_name))),
                    Err(e) => Ok(Some(format!("Error disconnecting from server {}: {}", server_name, e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Call MCP tool
    register_command(Command {
        name: "mcp-call-tool".to_string(),
        pattern: Regex::new(r"@mcp:call\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Call an MCP tool with arguments".to_string(),
        usage_example: r#"@mcp:call(server:tool, {"arg":"value"})"#.to_string(),
        handler: |params| {
            if params.len() < 2 {
                return Ok(Some(r#"Usage: @mcp:call(server:tool, {"arg":"value"})"#.to_string()));
            }
            
            let server_tool = &params[0];
            let args_str = &params[1];
            
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:call", &[server_tool.to_string(), args_str.to_string()]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some(format!("Failed to execute tool: {}", server_tool))),
                    Err(e) => Ok(Some(format!("Error executing tool {}: {}", server_tool, e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: None,
    });

    // Simple MCP tool call without JSON args
    register_command(Command {
        name: "mcp-call-simple".to_string(),
        pattern: Regex::new(r"@mcp:call\(\s*(\S+)\s*\)").unwrap(),
        description: "Call an MCP tool without arguments".to_string(),
        usage_example: "@mcp:call(server:tool)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: @mcp:call(server:tool)".to_string()));
            }
            
            let server_tool = &params[0];
            
            tokio::runtime::Handle::current().block_on(async {
                match mcp_client::execute_mcp_command("mcp:call", &[server_tool.to_string(), "{}".to_string()]).await {
                    Ok(Some(result)) => Ok(Some(result)),
                    Ok(None) => Ok(Some(format!("Failed to execute tool: {}", server_tool))),
                    Err(e) => Ok(Some(format!("Error executing tool {}: {}", server_tool, e))),
                }
            })
        },
        section: "mcp".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: None,
    });

    // MCP status command
    register_command(Command {
        name: "mcp-status".to_string(),
        pattern: Regex::new(r"@mcp:status\(\s*\)").unwrap(),
        description: "Show MCP system status and configuration".to_string(),
        usage_example: "@mcp:status()".to_string(),
        handler: |_params| {
            let config = match crate::configuration::get_effective_config() {
                Ok(config) => config,
                Err(e) => return Ok(Some(format!("Error loading config: {}", e))),
            };
            
            let status = format!(
                "MCP System Status:\n\
                 - Enabled: {}\n\
                 - Auto-connect: {}\n\
                 - Global timeout: {}s\n\
                 - Configured servers: {}\n\n\
                 Configured MCP Servers:\n{}",
                config.mcp.enabled,
                config.mcp.auto_connect,
                config.mcp.global_timeout_seconds,
                config.mcp.servers.len(),
                config.mcp.servers.iter()
                    .map(|(name, server_config)| format!(
                        "  - {}: {} {} ({})",
                        name,
                        server_config.command,
                        server_config.args.join(" "),
                        if server_config.enabled { "enabled" } else { "disabled" }
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            
            Ok(Some(status))
        },
        section: "mcp".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });
}