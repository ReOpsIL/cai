use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Hierarchical configuration manager supporting multiple config layers
#[derive(Debug, Clone)]
pub struct HierarchicalConfig {
    /// Global configuration (~/.config/cai/config.toml)
    global_config: Option<ConfigLayer>,
    /// User data configuration (~/.local/share/cai/config.toml)
    user_data_config: Option<ConfigLayer>,
    /// Project configuration (./cai.toml)
    project_config: Option<ConfigLayer>,
    /// Local configuration (./.cai.toml)
    local_config: Option<ConfigLayer>,
    /// Environment variable overrides
    env_overrides: HashMap<String, String>,
    /// Resolved configuration cache
    resolved_cache: Option<ResolvedConfig>,
}

/// Individual configuration layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigLayer {
    /// Source file path
    pub source_path: PathBuf,
    /// Configuration values
    pub config: ConfigValues,
    /// Layer priority (higher = more important)
    pub priority: u8,
    /// Last modification time
    pub last_modified: std::time::SystemTime,
}

/// Configuration values structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigValues {
    /// Core application settings
    pub core: Option<CoreConfig>,
    /// LLM provider settings
    pub llm: Option<LLMConfig>,
    /// MCP server configurations
    pub mcp: Option<MCPConfig>,
    /// Logging configuration
    pub logging: Option<LoggingConfig>,
    /// UI/UX settings
    pub ui: Option<UIConfig>,
    /// Workflow settings
    pub workflow: Option<WorkflowConfig>,
    /// Security settings
    pub security: Option<SecurityConfig>,
    /// Performance settings
    pub performance: Option<PerformanceConfig>,
    /// Custom user settings
    pub custom: Option<HashMap<String, toml::Value>>,
}

/// Core application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Default session timeout in seconds
    pub session_timeout: Option<u64>,
    /// Maximum concurrent operations
    pub max_concurrent_operations: Option<u32>,
    /// Default prompts directory
    pub prompts_directory: Option<PathBuf>,
    /// Auto-save session state
    pub auto_save_session: Option<bool>,
    /// Session state directory
    pub session_state_directory: Option<PathBuf>,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Default provider (openrouter, anthropic, openai, etc.)
    pub default_provider: Option<String>,
    /// API endpoints for different providers
    pub providers: Option<HashMap<String, ProviderConfig>>,
    /// Default model for each provider
    pub default_models: Option<HashMap<String, String>>,
    /// Request timeout in seconds
    pub request_timeout: Option<u64>,
    /// Maximum retries for failed requests
    pub max_retries: Option<u32>,
    /// Context window management
    pub context_management: Option<ContextConfig>,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API base URL
    pub base_url: Option<String>,
    /// API key (preferably from environment)
    pub api_key_env: Option<String>,
    /// Request headers
    pub headers: Option<HashMap<String, String>>,
    /// Provider-specific settings
    pub settings: Option<HashMap<String, toml::Value>>,
}

/// Context management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum context tokens
    pub max_tokens: Option<u32>,
    /// Context compression threshold (0.0 - 1.0)
    pub compression_threshold: Option<f64>,
    /// Context preservation strategy
    pub preservation_strategy: Option<String>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    /// MCP servers definition
    pub servers: Option<HashMap<String, MCPServerConfig>>,
    /// Global MCP settings
    pub global_timeout: Option<u64>,
    /// Auto-start servers
    pub auto_start: Option<Vec<String>>,
    /// Connection retry settings
    pub retry_config: Option<RetryConfig>,
}

/// Individual MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Server command
    pub command: String,
    /// Command arguments
    pub args: Option<Vec<String>>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Connection timeout
    pub timeout: Option<u64>,
    /// Auto-start with application
    pub auto_start: Option<bool>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: Option<u32>,
    /// Base delay between retries (in milliseconds)
    pub base_delay: Option<u64>,
    /// Maximum delay (in milliseconds)
    pub max_delay: Option<u64>,
    /// Backoff multiplier
    pub backoff_multiplier: Option<f64>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: Option<String>,
    /// Log format (json, compact, pretty)
    pub format: Option<String>,
    /// Log file path
    pub file: Option<PathBuf>,
    /// Rotate logs
    pub rotate: Option<bool>,
    /// Maximum log file size in MB
    pub max_size: Option<u64>,
    /// Maximum number of log files to keep
    pub max_files: Option<u32>,
}

/// UI/UX configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    /// Color scheme (auto, light, dark)
    pub color_scheme: Option<String>,
    /// Show progress bars
    pub show_progress: Option<bool>,
    /// Animation enabled
    pub animations: Option<bool>,
    /// Status line configuration
    pub status_line: Option<StatusLineConfig>,
    /// Prompt display settings
    pub prompts: Option<PromptDisplayConfig>,
}

/// Status line configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusLineConfig {
    /// Show current task
    pub show_task: Option<bool>,
    /// Show session info
    pub show_session: Option<bool>,
    /// Show resource usage
    pub show_resources: Option<bool>,
    /// Update interval in milliseconds
    pub update_interval: Option<u64>,
}

/// Prompt display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDisplayConfig {
    /// Maximum prompt length to display
    pub max_length: Option<usize>,
    /// Show metadata
    pub show_metadata: Option<bool>,
    /// Syntax highlighting
    pub syntax_highlighting: Option<bool>,
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Default workflow timeout in seconds
    pub default_timeout: Option<u64>,
    /// Maximum workflow depth
    pub max_depth: Option<u32>,
    /// Auto-save workflow state
    pub auto_save: Option<bool>,
    /// Workflow templates directory
    pub templates_directory: Option<PathBuf>,
    /// Parallel execution settings
    pub parallel_execution: Option<ParallelConfig>,
}

/// Parallel execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Enable parallel task execution
    pub enabled: Option<bool>,
    /// Maximum parallel tasks
    pub max_parallel_tasks: Option<u32>,
    /// Task scheduling strategy
    pub scheduling_strategy: Option<String>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Permission system settings
    pub permissions: Option<PermissionConfig>,
    /// File access restrictions
    pub file_access: Option<FileAccessConfig>,
    /// Network access restrictions
    pub network_access: Option<NetworkAccessConfig>,
    /// Encryption settings
    pub encryption: Option<EncryptionConfig>,
}

/// Permission system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Default permission mode (strict, moderate, permissive)
    pub default_mode: Option<String>,
    /// Auto-approval for trusted operations
    pub auto_approve_trusted: Option<bool>,
    /// Session permission timeout in seconds
    pub session_timeout: Option<u64>,
    /// Trusted paths
    pub trusted_paths: Option<Vec<PathBuf>>,
}

/// File access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccessConfig {
    /// Allowed directories
    pub allowed_directories: Option<Vec<PathBuf>>,
    /// Blocked directories
    pub blocked_directories: Option<Vec<PathBuf>>,
    /// Maximum file size for operations (in MB)
    pub max_file_size: Option<u64>,
    /// Require read-before-edit
    pub require_read_before_edit: Option<bool>,
}

/// Network access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAccessConfig {
    /// Allow network access
    pub enabled: Option<bool>,
    /// Allowed domains
    pub allowed_domains: Option<Vec<String>>,
    /// Blocked domains
    pub blocked_domains: Option<Vec<String>>,
    /// Proxy settings
    pub proxy: Option<ProxyConfig>,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy URL
    pub url: Option<String>,
    /// Proxy authentication
    pub auth: Option<ProxyAuth>,
}

/// Proxy authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuth {
    /// Username
    pub username: Option<String>,
    /// Password environment variable
    pub password_env: Option<String>,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Encrypt session data
    pub encrypt_sessions: Option<bool>,
    /// Encryption algorithm
    pub algorithm: Option<String>,
    /// Key derivation settings
    pub key_derivation: Option<KeyDerivationConfig>,
}

/// Key derivation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationConfig {
    /// Iterations for PBKDF2
    pub iterations: Option<u32>,
    /// Salt size in bytes
    pub salt_size: Option<u32>,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Thread pool size
    pub thread_pool_size: Option<usize>,
    /// Memory limits
    pub memory_limits: Option<MemoryLimits>,
    /// Cache settings
    pub cache: Option<CacheConfig>,
    /// Optimization settings
    pub optimization: Option<OptimizationConfig>,
}

/// Memory limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Maximum heap size in MB
    pub max_heap_size: Option<u64>,
    /// Memory warning threshold in MB
    pub warning_threshold: Option<u64>,
    /// Garbage collection settings
    pub gc_settings: Option<HashMap<String, toml::Value>>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: Option<bool>,
    /// Cache directory
    pub directory: Option<PathBuf>,
    /// Maximum cache size in MB
    pub max_size: Option<u64>,
    /// Cache TTL in seconds
    pub ttl: Option<u64>,
    /// Cache cleanup interval in seconds
    pub cleanup_interval: Option<u64>,
}

/// Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Enable optimizations
    pub enabled: Option<bool>,
    /// Specific optimizations to enable
    pub enable_list: Option<Vec<String>>,
    /// Specific optimizations to disable
    pub disable_list: Option<Vec<String>>,
}

/// Fully resolved configuration
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub config: ConfigValues,
    pub resolution_order: Vec<String>,
    pub resolution_timestamp: std::time::SystemTime,
}

impl HierarchicalConfig {
    /// Load configuration from all available sources
    pub async fn load() -> Result<Self> {
        let mut config = Self {
            global_config: None,
            user_data_config: None,
            project_config: None,
            local_config: None,
            env_overrides: Self::load_env_overrides(),
            resolved_cache: None,
        };

        // Load configurations in order of priority (lowest to highest)
        config.load_global_config().await?;
        config.load_user_data_config().await?;
        config.load_project_config().await?;
        config.load_local_config().await?;

        // Resolve final configuration
        config.resolve()?;

        Ok(config)
    }

    /// Load global configuration
    async fn load_global_config(&mut self) -> Result<()> {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("cai").join("config.toml");
            self.global_config = Self::load_config_file(&config_path, 10).await?;
        }
        Ok(())
    }

    /// Load user data configuration
    async fn load_user_data_config(&mut self) -> Result<()> {
        if let Some(data_dir) = dirs::data_local_dir() {
            let config_path = data_dir.join("cai").join("config.toml");
            self.user_data_config = Self::load_config_file(&config_path, 20).await?;
        }
        Ok(())
    }

    /// Load project configuration
    async fn load_project_config(&mut self) -> Result<()> {
        let config_path = PathBuf::from("./cai.toml");
        self.project_config = Self::load_config_file(&config_path, 30).await?;
        Ok(())
    }

    /// Load local configuration
    async fn load_local_config(&mut self) -> Result<()> {
        let config_path = PathBuf::from("./.cai.toml");
        self.local_config = Self::load_config_file(&config_path, 40).await?;
        Ok(())
    }

    /// Load configuration from a specific file
    async fn load_config_file(
        path: &Path,
        priority: u8,
    ) -> Result<Option<ConfigLayer>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).await?;
        let config: ConfigValues = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config file {}: {}", path.display(), e))?;

        let metadata = fs::metadata(path).await?;
        let last_modified = metadata.modified()?;

        Ok(Some(ConfigLayer {
            source_path: path.to_path_buf(),
            config,
            priority,
            last_modified,
        }))
    }

    /// Load environment variable overrides
    fn load_env_overrides() -> HashMap<String, String> {
        let mut overrides = HashMap::new();
        
        // Standard CAI environment variables
        if let Ok(value) = env::var("CAI_LOG_LEVEL") {
            overrides.insert("logging.level".to_string(), value);
        }
        if let Ok(value) = env::var("CAI_PROMPTS_DIR") {
            overrides.insert("core.prompts_directory".to_string(), value);
        }
        if let Ok(value) = env::var("CAI_SESSION_TIMEOUT") {
            overrides.insert("core.session_timeout".to_string(), value);
        }
        if let Ok(value) = env::var("OPENROUTER_API_KEY") {
            overrides.insert("llm.providers.openrouter.api_key_env".to_string(), "OPENROUTER_API_KEY".to_string());
        }
        if let Ok(value) = env::var("CAI_MAX_CONCURRENT") {
            overrides.insert("core.max_concurrent_operations".to_string(), value);
        }
        if let Ok(value) = env::var("CAI_UI_THEME") {
            overrides.insert("ui.color_scheme".to_string(), value);
        }

        // Load all CAI_* environment variables
        for (key, value) in env::vars() {
            if key.starts_with("CAI_CONFIG_") {
                let config_key = key
                    .strip_prefix("CAI_CONFIG_")
                    .unwrap()
                    .to_lowercase()
                    .replace('_', ".");
                overrides.insert(config_key, value);
            }
        }

        overrides
    }

    /// Resolve final configuration by merging all layers
    fn resolve(&mut self) -> Result<()> {
        let mut resolved = ConfigValues::default();
        let mut resolution_order = Vec::new();

        // Merge configurations in priority order
        let layers = [
            &self.global_config,
            &self.user_data_config,
            &self.project_config,
            &self.local_config,
        ];

        for layer in layers.iter() {
            if let Some(layer) = layer {
                Self::merge_config_values(&mut resolved, &layer.config)?;
                resolution_order.push(layer.source_path.display().to_string());
            }
        }

        // Apply environment overrides
        self.apply_env_overrides(&mut resolved)?;
        if !self.env_overrides.is_empty() {
            resolution_order.push("Environment Variables".to_string());
        }

        self.resolved_cache = Some(ResolvedConfig {
            config: resolved,
            resolution_order,
            resolution_timestamp: std::time::SystemTime::now(),
        });

        Ok(())
    }

    /// Merge configuration values
    fn merge_config_values(target: &mut ConfigValues, source: &ConfigValues) -> Result<()> {
        // Core config
        if let Some(source_core) = &source.core {
            let target_core = target.core.get_or_insert_with(CoreConfig::default);
            Self::merge_core_config(target_core, source_core);
        }

        // LLM config
        if let Some(source_llm) = &source.llm {
            let target_llm = target.llm.get_or_insert_with(LLMConfig::default);
            Self::merge_llm_config(target_llm, source_llm);
        }

        // MCP config
        if let Some(source_mcp) = &source.mcp {
            let target_mcp = target.mcp.get_or_insert_with(MCPConfig::default);
            Self::merge_mcp_config(target_mcp, source_mcp);
        }

        // Logging config
        if let Some(source_logging) = &source.logging {
            let target_logging = target.logging.get_or_insert_with(LoggingConfig::default);
            Self::merge_logging_config(target_logging, source_logging);
        }

        // UI config
        if let Some(source_ui) = &source.ui {
            let target_ui = target.ui.get_or_insert_with(UIConfig::default);
            Self::merge_ui_config(target_ui, source_ui);
        }

        // Other configs can be merged similarly...

        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&self, config: &mut ConfigValues) -> Result<()> {
        for (key, value) in &self.env_overrides {
            Self::set_config_value(config, key, value)?;
        }
        Ok(())
    }

    /// Set a configuration value using dot notation
    fn set_config_value(config: &mut ConfigValues, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        
        match parts.as_slice() {
            ["logging", "level"] => {
                config.logging.get_or_insert_with(LoggingConfig::default).level = Some(value.to_string());
            }
            ["core", "prompts_directory"] => {
                config.core.get_or_insert_with(CoreConfig::default).prompts_directory = Some(PathBuf::from(value));
            }
            ["core", "session_timeout"] => {
                let timeout = value.parse::<u64>()
                    .map_err(|_| anyhow!("Invalid session timeout value: {}", value))?;
                config.core.get_or_insert_with(CoreConfig::default).session_timeout = Some(timeout);
            }
            ["ui", "color_scheme"] => {
                config.ui.get_or_insert_with(UIConfig::default).color_scheme = Some(value.to_string());
            }
            // Add more mappings as needed
            _ => {
                // Store in custom section if not recognized
                config.custom.get_or_insert_with(HashMap::new)
                    .insert(key.to_string(), toml::Value::String(value.to_string()));
            }
        }

        Ok(())
    }

    /// Get a configuration value with type inference
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        let resolved = self.resolved_cache.as_ref()?;
        self.get_value_from_config(&resolved.config, key)
    }

    /// Get a configuration value from the resolved config
    fn get_value_from_config<T>(&self, config: &ConfigValues, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        let parts: Vec<&str> = key.split('.').collect();
        
        match parts.as_slice() {
            ["logging", "level"] => {
                config.logging.as_ref()?.level.as_ref().and_then(|v| {
                    toml::Value::String(v.clone()).try_into().ok()
                })
            }
            ["core", "session_timeout"] => {
                config.core.as_ref()?.session_timeout.and_then(|v| {
                    toml::Value::Integer(v as i64).try_into().ok()
                })
            }
            ["ui", "color_scheme"] => {
                config.ui.as_ref()?.color_scheme.as_ref().and_then(|v| {
                    toml::Value::String(v.clone()).try_into().ok()
                })
            }
            // Add more value extractors as needed
            _ => {
                // Check custom section
                config.custom.as_ref()?.get(key).and_then(|v| {
                    v.clone().try_into().ok()
                })
            }
        }
    }

    /// Get the resolved configuration
    pub fn get_resolved(&self) -> Option<&ResolvedConfig> {
        self.resolved_cache.as_ref()
    }

    /// Check if configuration needs reloading
    pub async fn needs_reload(&self) -> Result<bool> {
        let paths = [
            (&self.global_config, dirs::config_dir().map(|d| d.join("cai").join("config.toml"))),
            (&self.user_data_config, dirs::data_local_dir().map(|d| d.join("cai").join("config.toml"))),
            (&self.project_config, Some(PathBuf::from("./cai.toml"))),
            (&self.local_config, Some(PathBuf::from("./.cai.toml"))),
        ];

        for (layer, path) in paths.iter() {
            if let Some(path) = path {
                if path.exists() {
                    let metadata = fs::metadata(path).await?;
                    let current_modified = metadata.modified()?;
                    
                    if let Some(layer) = layer {
                        if current_modified > layer.last_modified {
                            return Ok(true);
                        }
                    } else {
                        // New file appeared
                        return Ok(true);
                    }
                } else if layer.is_some() {
                    // File was deleted
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Reload configuration
    pub async fn reload(&mut self) -> Result<()> {
        *self = Self::load().await?;
        Ok(())
    }

    /// Export current configuration to a file
    pub async fn export_to_file(&self, path: &Path) -> Result<()> {
        if let Some(resolved) = &self.resolved_cache {
            let content = toml::to_string_pretty(&resolved.config)?;
            
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            
            fs::write(path, content).await?;
        }
        Ok(())
    }

    /// Get configuration resolution info
    pub fn get_resolution_info(&self) -> Option<Vec<String>> {
        self.resolved_cache.as_ref().map(|r| r.resolution_order.clone())
    }
}

// Merge helper functions
impl HierarchicalConfig {
    fn merge_core_config(target: &mut CoreConfig, source: &CoreConfig) {
        if source.session_timeout.is_some() {
            target.session_timeout = source.session_timeout;
        }
        if source.max_concurrent_operations.is_some() {
            target.max_concurrent_operations = source.max_concurrent_operations;
        }
        if source.prompts_directory.is_some() {
            target.prompts_directory = source.prompts_directory.clone();
        }
        if source.auto_save_session.is_some() {
            target.auto_save_session = source.auto_save_session;
        }
        if source.session_state_directory.is_some() {
            target.session_state_directory = source.session_state_directory.clone();
        }
    }

    fn merge_llm_config(target: &mut LLMConfig, source: &LLMConfig) {
        if source.default_provider.is_some() {
            target.default_provider = source.default_provider.clone();
        }
        if let Some(source_providers) = &source.providers {
            let target_providers = target.providers.get_or_insert_with(HashMap::new);
            for (key, value) in source_providers {
                target_providers.insert(key.clone(), value.clone());
            }
        }
        // Continue for other fields...
    }

    fn merge_mcp_config(target: &mut MCPConfig, source: &MCPConfig) {
        if let Some(source_servers) = &source.servers {
            let target_servers = target.servers.get_or_insert_with(HashMap::new);
            for (key, value) in source_servers {
                target_servers.insert(key.clone(), value.clone());
            }
        }
        if source.global_timeout.is_some() {
            target.global_timeout = source.global_timeout;
        }
        // Continue for other fields...
    }

    fn merge_logging_config(target: &mut LoggingConfig, source: &LoggingConfig) {
        if source.level.is_some() {
            target.level = source.level.clone();
        }
        if source.format.is_some() {
            target.format = source.format.clone();
        }
        if source.file.is_some() {
            target.file = source.file.clone();
        }
        // Continue for other fields...
    }

    fn merge_ui_config(target: &mut UIConfig, source: &UIConfig) {
        if source.color_scheme.is_some() {
            target.color_scheme = source.color_scheme.clone();
        }
        if source.show_progress.is_some() {
            target.show_progress = source.show_progress;
        }
        // Continue for other fields...
    }
}

// Default implementations
impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            session_timeout: Some(3600), // 1 hour
            max_concurrent_operations: Some(4),
            prompts_directory: Some(PathBuf::from("./prompts")),
            auto_save_session: Some(true),
            session_state_directory: None,
        }
    }
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            default_provider: Some("openrouter".to_string()),
            providers: None,
            default_models: None,
            request_timeout: Some(120),
            max_retries: Some(3),
            context_management: None,
        }
    }
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            servers: Some(HashMap::new()),
            global_timeout: Some(30),
            auto_start: Some(Vec::new()),
            retry_config: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Some("info".to_string()),
            format: Some("compact".to_string()),
            file: None,
            rotate: Some(false),
            max_size: Some(100),
            max_files: Some(5),
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            color_scheme: Some("auto".to_string()),
            show_progress: Some(true),
            animations: Some(true),
            status_line: None,
            prompts: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config_content = r#"
[core]
session_timeout = 7200
prompts_directory = "/test/prompts"

[logging]
level = "debug"
format = "json"

[ui]
color_scheme = "dark"
show_progress = true
"#;

        fs::write(&config_path, config_content).await.unwrap();
        
        let layer = HierarchicalConfig::load_config_file(&config_path, 10).await.unwrap().unwrap();
        assert_eq!(layer.config.core.as_ref().unwrap().session_timeout, Some(7200));
        assert_eq!(layer.config.logging.as_ref().unwrap().level, Some("debug".to_string()));
    }

    #[tokio::test]
    async fn test_config_merging() {
        let mut config = ConfigValues::default();
        
        let source1 = ConfigValues {
            core: Some(CoreConfig {
                session_timeout: Some(3600),
                max_concurrent_operations: Some(2),
                ..Default::default()
            }),
            logging: Some(LoggingConfig {
                level: Some("info".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let source2 = ConfigValues {
            core: Some(CoreConfig {
                session_timeout: Some(7200), // Override
                prompts_directory: Some(PathBuf::from("/new/path")), // New
                ..Default::default()
            }),
            ..Default::default()
        };

        HierarchicalConfig::merge_config_values(&mut config, &source1).unwrap();
        HierarchicalConfig::merge_config_values(&mut config, &source2).unwrap();

        // Check merged values
        let core = config.core.unwrap();
        assert_eq!(core.session_timeout, Some(7200)); // Overridden
        assert_eq!(core.max_concurrent_operations, Some(2)); // Preserved
        assert_eq!(core.prompts_directory, Some(PathBuf::from("/new/path"))); // New

        assert_eq!(config.logging.unwrap().level, Some("info".to_string())); // Preserved
    }
}