use regex::Regex;
use crate::commands_registry::{Command, CommandType, register_command};
use crate::configuration::{self, Config, LlmSettings};
use std::collections::HashMap;

pub fn register_config_commands() {
    // Get current configuration
    register_command(Command {
        name: "config-get".to_string(),
        pattern: Regex::new(r"!config-get\(\s*\)").unwrap(),
        description: "Display current configuration".to_string(),
        usage_example: "!config-get()".to_string(),
        handler: |_| {
            match configuration::get_effective_config() {
                Ok(config) => {
                    let config_str = format!(
                        "Current Configuration:\n\
                         \nLLM Settings:\n\
                         - Model: {}\n\
                         - Temperature: {}\n\
                         - Max Tokens: {}\n\
                         - Top P: {}\n\
                         - System Prompt: {}\n\
                         \nUI Settings:\n\
                         - Color Scheme: {}\n\
                         - Show Line Numbers: {}\n\
                         - Response Format: {}\n\
                         - Auto Scroll: {}\n\
                         \nMemory Settings:\n\
                         - Auto Save Interval (min): {}\n\
                         - Memory Limit (MB): {}\n\
                         - Default Export Format: {}\n\
                         - Auto Export on Exit: {}\n\
                         \nWorkflow Settings:\n\
                         - Max Iterations: {}\n\
                         - Timeout (sec): {}\n\
                         - Verify Steps: {}\n\
                         - Parallel Execution: {}",
                        config.llm.model,
                        config.llm.temperature,
                        config.llm.max_tokens,
                        config.llm.top_p,
                        config.llm.system_prompt.as_ref().unwrap_or(&"None".to_string()),
                        config.ui.color_scheme,
                        config.ui.show_line_numbers,
                        config.ui.response_format,
                        config.ui.auto_scroll,
                        config.memory.auto_save_interval_minutes,
                        config.memory.memory_limit_mb,
                        config.memory.default_export_format,
                        config.memory.auto_export_on_exit,
                        config.workflow.max_iterations,
                        config.workflow.timeout_seconds,
                        config.workflow.verify_steps,
                        config.workflow.parallel_execution
                    );
                    Ok(Some(config_str))
                }
                Err(e) => Ok(Some(format!("Error loading configuration: {}", e)))
            }
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Set LLM parameter
    register_command(Command {
        name: "config-set-llm".to_string(),
        pattern: Regex::new(r"!config-set-llm\(\s*(\S+)\s*,\s*(.+)\s*\)").unwrap(),
        description: "Set LLM parameter (temperature, max_tokens, top_p, system_prompt)".to_string(),
        usage_example: "!config-set-llm(temperature, 0.8)".to_string(),
        handler: |params| {
            if params.len() < 2 {
                return Ok(Some("Usage: !config-set-llm(parameter, value)".to_string()));
            }
            
            let param = &params[0];
            let value = &params[1];
            
            match configuration::load_configuration() {
                Ok(mut config) => {
                    match param.as_str() {
                        "temperature" => {
                            match value.parse::<f32>() {
                                Ok(temp) if temp >= 0.0 && temp <= 2.0 => {
                                    config.llm.temperature = temp;
                                }
                                Ok(_) => return Ok(Some("Temperature must be between 0.0 and 2.0".to_string())),
                                Err(_) => return Ok(Some("Invalid temperature value".to_string())),
                            }
                        }
                        "max_tokens" => {
                            match value.parse::<u32>() {
                                Ok(tokens) if tokens > 0 && tokens <= 32000 => {
                                    config.llm.max_tokens = tokens;
                                }
                                Ok(_) => return Ok(Some("Max tokens must be between 1 and 32000".to_string())),
                                Err(_) => return Ok(Some("Invalid max_tokens value".to_string())),
                            }
                        }
                        "top_p" => {
                            match value.parse::<f32>() {
                                Ok(top_p) if top_p > 0.0 && top_p <= 1.0 => {
                                    config.llm.top_p = top_p;
                                }
                                Ok(_) => return Ok(Some("Top P must be between 0.0 and 1.0".to_string())),
                                Err(_) => return Ok(Some("Invalid top_p value".to_string())),
                            }
                        }
                        "system_prompt" => {
                            if value == "null" || value == "none" {
                                config.llm.system_prompt = None;
                            } else {
                                config.llm.system_prompt = Some(value.to_string());
                            }
                        }
                        _ => return Ok(Some("Invalid parameter. Use: temperature, max_tokens, top_p, or system_prompt".to_string())),
                    }
                    
                    match configuration::save_configuration(&config) {
                        Ok(_) => Ok(Some(format!("LLM parameter '{}' set to '{}'", param, value))),
                        Err(e) => Ok(Some(format!("Error saving configuration: {}", e))),
                    }
                }
                Err(e) => Ok(Some(format!("Error loading configuration: {}", e)))
            }
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Set session override
    register_command(Command {
        name: "config-session".to_string(),
        pattern: Regex::new(r"!config-session\(\s*(\S+)\s*,\s*(.+)\s*\)").unwrap(),
        description: "Set temporary session configuration override".to_string(),
        usage_example: "!config-session(temperature, 1.2)".to_string(),
        handler: |params| {
            if params.len() < 2 {
                return Ok(Some("Usage: !config-session(parameter, value)".to_string()));
            }
            
            let param = &params[0];
            let value = &params[1];
            
            // Get current session override or create new one
            let mut override_config = configuration::get_session_config_override()
                .cloned()
                .unwrap_or_else(|| Config::default());
            
            match param.as_str() {
                "temperature" => {
                    match value.parse::<f32>() {
                        Ok(temp) if temp >= 0.0 && temp <= 2.0 => {
                            override_config.llm.temperature = temp;
                        }
                        Ok(_) => return Ok(Some("Temperature must be between 0.0 and 2.0".to_string())),
                        Err(_) => return Ok(Some("Invalid temperature value".to_string())),
                    }
                }
                "max_tokens" => {
                    match value.parse::<u32>() {
                        Ok(tokens) if tokens > 0 && tokens <= 32000 => {
                            override_config.llm.max_tokens = tokens;
                        }
                        Ok(_) => return Ok(Some("Max tokens must be between 1 and 32000".to_string())),
                        Err(_) => return Ok(Some("Invalid max_tokens value".to_string())),
                    }
                }
                "top_p" => {
                    match value.parse::<f32>() {
                        Ok(top_p) if top_p > 0.0 && top_p <= 1.0 => {
                            override_config.llm.top_p = top_p;
                        }
                        Ok(_) => return Ok(Some("Top P must be between 0.0 and 1.0".to_string())),
                        Err(_) => return Ok(Some("Invalid top_p value".to_string())),
                    }
                }
                "system_prompt" => {
                    if value == "null" || value == "none" {
                        override_config.llm.system_prompt = None;
                    } else {
                        override_config.llm.system_prompt = Some(value.to_string());
                    }
                }
                _ => return Ok(Some("Invalid parameter. Use: temperature, max_tokens, top_p, or system_prompt".to_string())),
            }
            
            configuration::set_session_config_override(override_config);
            Ok(Some(format!("Session override for '{}' set to '{}' (temporary)", param, value)))
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Clear session overrides
    register_command(Command {
        name: "config-session-clear".to_string(),
        pattern: Regex::new(r"!config-session-clear\(\s*\)").unwrap(),
        description: "Clear all session configuration overrides".to_string(),
        usage_example: "!config-session-clear()".to_string(),
        handler: |_| {
            configuration::clear_session_config_override();
            Ok(Some("Session configuration overrides cleared".to_string()))
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Create and save model preset
    register_command(Command {
        name: "config-preset-save".to_string(),
        pattern: Regex::new(r"!config-preset-save\(\s*(\S+)\s*\)").unwrap(),
        description: "Save current LLM settings as a preset".to_string(),
        usage_example: "!config-preset-save(creative)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: !config-preset-save(preset_name)".to_string()));
            }
            
            let preset_name = &params[0];
            
            match configuration::get_effective_config() {
                Ok(mut config) => {
                    let preset = config.llm.clone();
                    config.model_presets.insert(preset_name.to_string(), preset);
                    
                    match configuration::save_configuration(&config) {
                        Ok(_) => Ok(Some(format!("Preset '{}' saved successfully", preset_name))),
                        Err(e) => Ok(Some(format!("Error saving preset: {}", e))),
                    }
                }
                Err(e) => Ok(Some(format!("Error loading configuration: {}", e)))
            }
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Load model preset
    register_command(Command {
        name: "config-preset-load".to_string(),
        pattern: Regex::new(r"!config-preset-load\(\s*(\S+)\s*\)").unwrap(),
        description: "Load a saved model preset".to_string(),
        usage_example: "!config-preset-load(creative)".to_string(),
        handler: |params| {
            if params.is_empty() {
                return Ok(Some("Usage: !config-preset-load(preset_name)".to_string()));
            }
            
            let preset_name = &params[0];
            
            match configuration::load_configuration() {
                Ok(mut config) => {
                    match config.model_presets.get(preset_name) {
                        Some(preset) => {
                            config.llm = preset.clone();
                            match configuration::save_configuration(&config) {
                                Ok(_) => Ok(Some(format!("Preset '{}' loaded successfully", preset_name))),
                                Err(e) => Ok(Some(format!("Error saving configuration: {}", e))),
                            }
                        }
                        None => Ok(Some(format!("Preset '{}' not found", preset_name)))
                    }
                }
                Err(e) => Ok(Some(format!("Error loading configuration: {}", e)))
            }
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // List presets
    register_command(Command {
        name: "config-preset-list".to_string(),
        pattern: Regex::new(r"!config-preset-list\(\s*\)").unwrap(),
        description: "List all saved model presets".to_string(),
        usage_example: "!config-preset-list()".to_string(),
        handler: |_| {
            match configuration::load_configuration() {
                Ok(config) => {
                    if config.model_presets.is_empty() {
                        Ok(Some("No presets saved".to_string()))
                    } else {
                        let mut preset_list = String::from("Saved Presets:\n");
                        for (name, preset) in &config.model_presets {
                            preset_list.push_str(&format!(
                                "- {}: {} (temp: {}, max_tokens: {}, top_p: {})\n",
                                name, preset.model, preset.temperature, preset.max_tokens, preset.top_p
                            ));
                        }
                        Ok(Some(preset_list))
                    }
                }
                Err(e) => Ok(Some(format!("Error loading configuration: {}", e)))
            }
        },
        section: "configuration".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });
}