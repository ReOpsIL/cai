use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

use crate::logger::{log_info, log_warn};

/// Docker Environment Detection and Configuration
/// Automatically detects Docker setup and configures optimal paths for MCP servers
#[derive(Debug, Clone)]
pub struct DockerEnvironment {
    pub is_available: bool,
    pub version: Option<String>,
    pub platform: DockerPlatform,
    pub mount_capabilities: MountCapabilities,
    pub recommended_config: HashMap<String, DockerServerConfig>,
}

#[derive(Debug, Clone)]
pub enum DockerPlatform {
    DockerDesktop,
    DockerEngine,
    Podman,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct MountCapabilities {
    pub supports_host_mounts: bool,
    pub max_mount_points: usize,
    pub preferred_mount_strategy: MountStrategy,
}

#[derive(Debug, Clone)]
pub enum MountStrategy {
    DirectMount,
    VolumeMount,
    CopyStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub working_directory: PathBuf,
}

pub struct DockerDetector {
    current_dir: PathBuf,
    detected_environment: Option<DockerEnvironment>,
}

impl DockerDetector {
    pub fn new() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Ok(Self {
            current_dir,
            detected_environment: None,
        })
    }

    /// Detect Docker environment and capabilities
    pub async fn detect_environment(&mut self) -> Result<&DockerEnvironment> {
        log_info!("docker", "🔍 Detecting Docker environment...");

        let mut environment = DockerEnvironment {
            is_available: false,
            version: None,
            platform: DockerPlatform::Unknown,
            mount_capabilities: MountCapabilities {
                supports_host_mounts: false,
                max_mount_points: 0,
                preferred_mount_strategy: MountStrategy::CopyStrategy,
            },
            recommended_config: HashMap::new(),
        };

        // Check if Docker is available
        if let Ok(docker_info) = self.check_docker_availability().await {
            environment.is_available = true;
            environment.version = Some(docker_info.version);
            environment.platform = docker_info.platform;
            environment.mount_capabilities = self.detect_mount_capabilities().await?;
            environment.recommended_config = self.generate_recommended_configs(&environment).await?;
            
            log_info!("docker", "✅ Docker detected: {} ({})", 
                     environment.version.as_ref().unwrap_or(&"unknown".to_string()),
                     format!("{:?}", environment.platform));
        } else {
            log_warn!("docker", "❌ Docker not available, using fallback configurations");
            environment.recommended_config = self.generate_fallback_configs().await?;
        }

        self.detected_environment = Some(environment);
        Ok(self.detected_environment.as_ref().unwrap())
    }

    /// Check Docker availability and get basic info
    async fn check_docker_availability(&self) -> Result<DockerInfo> {
        // Try Docker first
        if let Ok(output) = Command::new("docker").args(&["--version"]).output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = self.parse_docker_version(&version_output);
                let platform = self.detect_docker_platform().await;
                
                return Ok(DockerInfo { version, platform });
            }
        }

        // Try Podman as fallback
        if let Ok(output) = Command::new("podman").args(&["--version"]).output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = self.parse_podman_version(&version_output);
                
                return Ok(DockerInfo { 
                    version, 
                    platform: DockerPlatform::Podman 
                });
            }
        }

        Err(anyhow!("Docker/Podman not available"))
    }

    /// Detect Docker platform (Desktop vs Engine)
    async fn detect_docker_platform(&self) -> DockerPlatform {
        // Try to detect Docker Desktop vs Docker Engine
        if let Ok(output) = Command::new("docker").args(&["info"]).output() {
            let info_output = String::from_utf8_lossy(&output.stdout);
            if info_output.contains("Docker Desktop") {
                return DockerPlatform::DockerDesktop;
            } else if info_output.contains("Docker Engine") {
                return DockerPlatform::DockerEngine;
            }
        }
        DockerPlatform::Unknown
    }

    /// Detect mount capabilities
    async fn detect_mount_capabilities(&self) -> Result<MountCapabilities> {
        let mut capabilities = MountCapabilities {
            supports_host_mounts: false,
            max_mount_points: 1,
            preferred_mount_strategy: MountStrategy::CopyStrategy,
        };

        // Test if we can mount the current directory
        let test_mount = format!("{}:/test", self.current_dir.display());
        
        if let Ok(output) = Command::new("docker")
            .args(&["run", "--rm", "-v", &test_mount, "alpine:latest", "ls", "/test"])
            .output() 
        {
            if output.status.success() {
                capabilities.supports_host_mounts = true;
                capabilities.max_mount_points = 5; // Conservative estimate
                capabilities.preferred_mount_strategy = MountStrategy::DirectMount;
                
                log_info!("docker", "✅ Host mount capability confirmed");
            } else {
                log_warn!("docker", "❌ Host mount test failed: {}", 
                         String::from_utf8_lossy(&output.stderr));
            }
        }

        Ok(capabilities)
    }

    /// Generate recommended Docker configurations for MCP servers
    async fn generate_recommended_configs(&self, env: &DockerEnvironment) -> Result<HashMap<String, DockerServerConfig>> {
        let mut configs = HashMap::new();

        // Filesystem server configuration
        let filesystem_config = if env.mount_capabilities.supports_host_mounts {
            DockerServerConfig {
                command: "docker".to_string(),
                args: vec![
                    "run".to_string(),
                    "-i".to_string(),
                    "--rm".to_string(),
                    "-v".to_string(),
                    format!("{}:/workspace", self.current_dir.display()),
                    "mcp/filesystem".to_string(),
                    "/workspace".to_string(),
                ],
                env: HashMap::new(),
                cwd: Some(self.current_dir.clone()),
                working_directory: self.current_dir.clone(),
            }
        } else {
            // Fallback configuration without host mounts
            DockerServerConfig {
                command: "docker".to_string(),
                args: vec![
                    "run".to_string(),
                    "-i".to_string(),
                    "--rm".to_string(),
                    "mcp/filesystem".to_string(),
                    "/app".to_string(),
                ],
                env: HashMap::new(),
                cwd: Some(self.current_dir.clone()),
                working_directory: self.current_dir.clone(),
            }
        };

        configs.insert("filesystem".to_string(), filesystem_config);

        // Web server configuration disabled (mcp/web image not available)
        // Uncomment below if mcp/web image becomes available
        /*
        if env.mount_capabilities.supports_host_mounts {
            let web_config = DockerServerConfig {
                command: "docker".to_string(),
                args: vec![
                    "run".to_string(),
                    "-i".to_string(),
                    "--rm".to_string(),
                    "-p".to_string(),
                    "8080:8080".to_string(),
                    "mcp/web".to_string(),
                ],
                env: HashMap::new(),
                cwd: Some(self.current_dir.clone()),
                working_directory: self.current_dir.clone(),
            };
            configs.insert("web".to_string(), web_config);
        }
        */

        log_info!("docker", "📋 Generated {} recommended Docker configurations", configs.len());
        Ok(configs)
    }

    /// Generate fallback configurations when Docker is not available
    async fn generate_fallback_configs(&self) -> Result<HashMap<String, DockerServerConfig>> {
        let mut configs = HashMap::new();

        // Native filesystem access (if available)
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            let native_config = DockerServerConfig {
                command: "mcp-filesystem-native".to_string(),
                args: vec![self.current_dir.display().to_string()],
                env: HashMap::new(),
                cwd: Some(self.current_dir.clone()),
                working_directory: self.current_dir.clone(),
            };
            configs.insert("filesystem".to_string(), native_config);
        }

        log_warn!("docker", "⚠️  Using fallback configurations (Docker unavailable)");
        Ok(configs)
    }

    /// Apply detected configuration to MCP config
    pub async fn apply_to_mcp_config(&self, config_path: &Path) -> Result<()> {
        let environment = self.detected_environment.as_ref()
            .ok_or_else(|| anyhow!("Docker environment not detected. Call detect_environment() first."))?;

        // Read existing config
        let config_content = if config_path.exists() {
            fs::read_to_string(config_path).await?
        } else {
            "{}".to_string()
        };

        let mut mcp_config: serde_json::Value = serde_json::from_str(&config_content)
            .unwrap_or_else(|_| serde_json::json!({}));

        // Update with recommended configurations
        if let Some(servers) = mcp_config.get_mut("mcpServers") {
            for (server_name, docker_config) in &environment.recommended_config {
                let server_config = serde_json::json!({
                    "command": docker_config.command,
                    "args": docker_config.args,
                    "env": docker_config.env,
                    "cwd": docker_config.cwd
                });
                
                servers[server_name] = server_config;
                log_info!("docker", "✅ Updated MCP server config: {}", server_name);
            }
        } else {
            // Create new mcpServers section
            let mut servers = serde_json::Map::new();
            for (server_name, docker_config) in &environment.recommended_config {
                let server_config = serde_json::json!({
                    "command": docker_config.command,
                    "args": docker_config.args,
                    "env": docker_config.env,
                    "cwd": docker_config.cwd
                });
                servers.insert(server_name.clone(), server_config);
            }
            mcp_config["mcpServers"] = serde_json::Value::Object(servers);
        }

        // Write updated config
        let updated_config = serde_json::to_string_pretty(&mcp_config)?;
        fs::write(config_path, updated_config).await?;

        log_info!("docker", "💾 MCP configuration updated with Docker-optimized settings");
        Ok(())
    }

    /// Get current detected environment
    pub fn get_environment(&self) -> Option<&DockerEnvironment> {
        self.detected_environment.as_ref()
    }

    /// Parse Docker version from output
    fn parse_docker_version(&self, output: &str) -> String {
        // Extract version from "Docker version 20.10.8, build 3967b7d"
        if let Some(captures) = regex::Regex::new(r"Docker version (\S+)")
            .ok()
            .and_then(|re| re.captures(output)) 
        {
            captures.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        }
    }

    /// Parse Podman version from output
    fn parse_podman_version(&self, output: &str) -> String {
        // Extract version from "podman version 3.4.2"
        if let Some(captures) = regex::Regex::new(r"podman version (\S+)")
            .ok()
            .and_then(|re| re.captures(output)) 
        {
            captures.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        }
    }
}

#[derive(Debug)]
struct DockerInfo {
    version: String,
    platform: DockerPlatform,
}

/// Initialize Docker detection and apply optimal configuration
pub async fn initialize_docker_optimization() -> Result<()> {
    log_info!("docker", "🚀 Initializing Docker optimization...");

    let mut detector = DockerDetector::new()?;
    detector.detect_environment().await?;

    // Apply configuration to MCP config file
    let config_path = PathBuf::from("mcp-config.json");
    detector.apply_to_mcp_config(&config_path).await?;

    // Get environment info for logging after applying config
    let environment = detector.get_environment()
        .ok_or_else(|| anyhow!("Environment detection failed"))?;

    log_info!("docker", "✅ Docker optimization completed successfully");
    if environment.is_available {
        log_info!("docker", "🐳 Docker available: {} servers configured", 
                 environment.recommended_config.len());
    } else {
        log_warn!("docker", "⚠️  Docker unavailable: using native fallbacks");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_docker_detector_creation() {
        let detector = DockerDetector::new();
        assert!(detector.is_ok());
    }

    #[tokio::test]
    async fn test_fallback_config_generation() {
        let mut detector = DockerDetector::new().unwrap();
        let configs = detector.generate_fallback_configs().await.unwrap();
        
        // Should generate at least one fallback config
        assert!(!configs.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_config_application() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test-mcp-config.json");
        
        let mut detector = DockerDetector::new().unwrap();
        let _environment = detector.detect_environment().await.unwrap();
        
        let result = detector.apply_to_mcp_config(&config_path).await;
        assert!(result.is_ok());
        
        // Config file should be created
        assert!(config_path.exists());
    }
}