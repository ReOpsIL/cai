use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::collections::HashMap;
use toml_edit;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LlmSettings {
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiSettings {
    #[serde(default = "default_color_scheme")]
    pub color_scheme: String,
    #[serde(default = "default_show_line_numbers")]
    pub show_line_numbers: bool,
    #[serde(default = "default_response_format")]
    pub response_format: String,
    #[serde(default = "default_auto_scroll")]
    pub auto_scroll: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemorySettings {
    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval_minutes: u32,
    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: u32,
    #[serde(default = "default_export_format")]
    pub default_export_format: String,
    #[serde(default = "default_auto_export")]
    pub auto_export_on_exit: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkflowSettings {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u32,
    #[serde(default = "default_verify_steps")]
    pub verify_steps: bool,
    #[serde(default = "default_parallel_execution")]
    pub parallel_execution: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_mcp_timeout")]
    pub timeout_seconds: u32,
    #[serde(default = "default_mcp_enabled")]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpSettings {
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
    #[serde(default = "default_mcp_auto_connect")]
    pub auto_connect: bool,
    #[serde(default = "default_mcp_global_timeout")]
    pub global_timeout_seconds: u32,
    #[serde(default = "default_mcp_enabled")]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub llm: LlmSettings,
    #[serde(default)]
    pub ui: UiSettings,
    #[serde(default)]
    pub memory: MemorySettings,
    #[serde(default)]
    pub workflow: WorkflowSettings,
    #[serde(default)]
    pub mcp: McpSettings,
    #[serde(default)]
    pub model_presets: HashMap<String, LlmSettings>,
}

// Default value functions
fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 4000 }
fn default_top_p() -> f32 { 0.9 }
fn default_color_scheme() -> String { "default".to_string() }
fn default_show_line_numbers() -> bool { true }
fn default_response_format() -> String { "markdown".to_string() }
fn default_auto_scroll() -> bool { true }
fn default_auto_save_interval() -> u32 { 5 }
fn default_memory_limit() -> u32 { 100 }
fn default_export_format() -> String { "markdown".to_string() }
fn default_auto_export() -> bool { false }
fn default_max_iterations() -> u32 { 10 }
fn default_timeout_seconds() -> u32 { 300 }
fn default_verify_steps() -> bool { true }
fn default_parallel_execution() -> bool { false }
fn default_mcp_timeout() -> u32 { 60 }
fn default_mcp_enabled() -> bool { true }
fn default_mcp_auto_connect() -> bool { true }
fn default_mcp_global_timeout() -> u32 { 120 }

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            model: "google/gemini-2.0-flash-exp:free".to_string(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            top_p: default_top_p(),
            system_prompt: None,
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            color_scheme: default_color_scheme(),
            show_line_numbers: default_show_line_numbers(),
            response_format: default_response_format(),
            auto_scroll: default_auto_scroll(),
        }
    }
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            auto_save_interval_minutes: default_auto_save_interval(),
            memory_limit_mb: default_memory_limit(),
            default_export_format: default_export_format(),
            auto_export_on_exit: default_auto_export(),
        }
    }
}

impl Default for WorkflowSettings {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            timeout_seconds: default_timeout_seconds(),
            verify_steps: default_verify_steps(),
            parallel_execution: default_parallel_execution(),
        }
    }
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            servers: HashMap::new(),
            auto_connect: default_mcp_auto_connect(),
            global_timeout_seconds: default_mcp_global_timeout(),
            enabled: default_mcp_enabled(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmSettings::default(),
            ui: UiSettings::default(),
            memory: MemorySettings::default(),
            workflow: WorkflowSettings::default(),
            mcp: McpSettings::default(),
            model_presets: HashMap::new(),
        }
    }
}

pub fn load_configuration() -> Result<Config, Box<dyn std::error::Error>> {
    let user_dirs = UserDirs::new().expect("Could not find user directories");
    let config_path = user_dirs.home_dir().join("cai.conf");

    let config: Config = if config_path.exists() {
        let mut config_file = fs::File::open(&config_path)?;
        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string)?;
        
        // Try parsing as new format first
        match toml_edit::de::from_str::<Config>(&config_string) {
            Ok(config) => config,
            Err(_) => {
                // Try parsing as legacy format (just model field)
                #[derive(Deserialize)]
                struct LegacyConfig {
                    model: String,
                }
                
                match toml_edit::de::from_str::<LegacyConfig>(&config_string) {
                    Ok(legacy) => {
                        // Convert legacy config to new format
                        let mut config = Config::default();
                        config.llm.model = legacy.model;
                        
                        // Save the migrated config
                        let _ = save_configuration(&config);
                        config
                    },
                    Err(e) => {
                        eprintln!("Warning: Could not parse config file: {}. Using defaults.", e);
                        Config::default()
                    }
                }
            }
        }
    } else {
        // Create default config file
        let default_config = Config::default();
        let toml = toml::to_string_pretty(&default_config)?;
        fs::write(&config_path, toml)?;
        default_config
    };

    Ok(config)
}

pub fn save_configuration(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let user_dirs = UserDirs::new().expect("Could not find user directories");
    let config_path = user_dirs.home_dir().join("cai.conf");
    let toml = toml::to_string_pretty(config)?;
    fs::write(&config_path, toml)?;
    Ok(())
}

// Session-based configuration override support
static mut SESSION_CONFIG_OVERRIDE: Option<Config> = None;

pub fn set_session_config_override(config: Config) {
    unsafe {
        SESSION_CONFIG_OVERRIDE = Some(config);
    }
}

pub fn get_session_config_override() -> Option<&'static Config> {
    unsafe {
        SESSION_CONFIG_OVERRIDE.as_ref()
    }
}

pub fn clear_session_config_override() {
    unsafe {
        SESSION_CONFIG_OVERRIDE = None;
    }
}

pub fn get_effective_config() -> Result<Config, Box<dyn std::error::Error>> {
    let base_config = load_configuration()?;
    
    if let Some(override_config) = get_session_config_override() {
        // Merge session overrides with base config
        let mut effective_config = base_config;
        
        // Override LLM settings if present in session config
        if override_config.llm.model != LlmSettings::default().model {
            effective_config.llm.model = override_config.llm.model.clone();
        }
        if override_config.llm.temperature != LlmSettings::default().temperature {
            effective_config.llm.temperature = override_config.llm.temperature;
        }
        if override_config.llm.max_tokens != LlmSettings::default().max_tokens {
            effective_config.llm.max_tokens = override_config.llm.max_tokens;
        }
        if override_config.llm.top_p != LlmSettings::default().top_p {
            effective_config.llm.top_p = override_config.llm.top_p;
        }
        if override_config.llm.system_prompt.is_some() {
            effective_config.llm.system_prompt = override_config.llm.system_prompt.clone();
        }
        
        Ok(effective_config)
    } else {
        Ok(base_config)
    }
}
