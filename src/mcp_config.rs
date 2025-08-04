use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

impl McpConfig {
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read MCP config file: {:?}", path))?;
        
        let config: McpConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse MCP config file: {:?}", path))?;
        
        Ok(config)
    }

    pub fn load_default() -> Result<Option<Self>> {
        // Try to load from default locations
        let mut possible_paths = vec![
            Some(PathBuf::from("mcp-config.json")),
            Some(PathBuf::from(".mcp-config.json")),
        ];
        
        if let Some(config_dir) = dirs::config_dir() {
            possible_paths.push(Some(config_dir.join("cai").join("mcp-config.json")));
        }
        
        if let Some(home_dir) = dirs::home_dir() {
            possible_paths.push(Some(home_dir.join(".config").join("cai").join("mcp-config.json")));
        }

        for path_opt in possible_paths {
            if let Some(path) = path_opt {
                if path.exists() {
                    return Ok(Some(Self::load_from_file(&path)?));
                }
            }
        }

        Ok(None)
    }

    pub fn get_server(&self, name: &str) -> Option<&McpServerConfig> {
        self.mcp_servers.get(name)
    }

    pub fn list_servers(&self) -> Vec<&String> {
        self.mcp_servers.keys().collect()
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        let mut servers = HashMap::new();
        
        // Add example filesystem server configuration
        servers.insert("filesystem".to_string(), McpServerConfig {
            command: "docker".to_string(),
            args: vec![
                "run".to_string(),
                "-i".to_string(),
                "--rm".to_string(),
                "-v".to_string(),
                "/local-directory:/local-directory".to_string(),
                "mcp/filesystem".to_string(),
                "/local-directory".to_string(),
            ],
            env: HashMap::new(),
            cwd: None,
        });

        Self {
            mcp_servers: servers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_parsing() {
        let json = r#"
        {
          "mcpServers": {
            "filesystem": {
              "command": "docker",
              "args": [
                "run",
                "-i",
                "--rm",
                "-v",
                "/local-directory:/local-directory",
                "mcp/filesystem",
                "/local-directory"
              ]
            }
          }
        }
        "#;
        
        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert!(config.get_server("filesystem").is_some());
    }
    
    #[test]
    fn test_default_config() {
        let config = McpConfig::default();
        assert!(config.get_server("filesystem").is_some());
    }
}