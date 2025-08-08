# Enhanced Configuration System

The CAI application now supports a comprehensive configuration system through the `~/cai.conf` file using TOML format.

## Configuration Structure

The configuration is organized into four main sections:

### LLM Settings
Controls the language model behavior:
```toml
[llm]
model = "google/gemini-2.0-flash-exp:free"
temperature = 0.7
max_tokens = 4000
top_p = 0.9
system_prompt = "You are a helpful AI assistant." # optional
```

### UI Settings
Controls the user interface:
```toml
[ui]
color_scheme = "default"
show_line_numbers = true
response_format = "markdown"
auto_scroll = true
```

### Memory Settings
Controls memory and export behavior:
```toml
[memory]
auto_save_interval_minutes = 5
memory_limit_mb = 100
default_export_format = "markdown"
auto_export_on_exit = false
```

### Workflow Settings
Controls workflow execution:
```toml
[workflow]
max_iterations = 10
timeout_seconds = 300
verify_steps = true
parallel_execution = false
```

### Model Presets
Save commonly used LLM configurations:
```toml
[model_presets.creative]
model = "anthropic/claude-3.5-sonnet"
temperature = 1.2
max_tokens = 8000
top_p = 0.95

[model_presets.analytical]
model = "google/gemini-2.0-flash-exp:free"
temperature = 0.3
max_tokens = 4000
top_p = 0.8
```

## Commands

### View Configuration
```
!config-get()
```
Displays current configuration settings.

### Set LLM Parameters
```
!config-set-llm(temperature, 0.8)
!config-set-llm(max_tokens, 6000)
!config-set-llm(top_p, 0.95)
!config-set-llm(system_prompt, "You are an expert developer.")
```

### Session Overrides
Temporarily override settings for the current session:
```
!config-session(temperature, 1.2)
!config-session(system_prompt, "Be creative and innovative.")
```

Clear session overrides:
```
!config-session-clear()
```

### Model Presets
Save current LLM settings as a preset:
```
!config-preset-save(my_preset)
```

Load a saved preset:
```
!config-preset-load(creative)
```

List all presets:
```
!config-preset-list()
```

## Backward Compatibility

The system automatically migrates old configuration files that only contain a `model` field to the new format while preserving your model selection.

## Default Values

If no configuration file exists, the system creates one with sensible defaults:
- Model: `google/gemini-2.0-flash-exp:free`
- Temperature: `0.7`
- Max Tokens: `4000`
- Top P: `0.9`
- All other settings use their respective defaults