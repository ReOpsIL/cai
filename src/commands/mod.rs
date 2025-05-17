use regex::Regex;
use std::fs;
use std::path::Path;

use crate::autocomplete::{autocomplete_file_path, autocomplete_memory_id, autocomplete_model_id};
use crate::chat::{self, Prompt, PromptType};
use crate::commands_registry::{Command, CommandType, register_command};
use crate::files::files as file_module; // Import autocomplete handlers

pub mod bash_cmd;
pub mod help;
pub mod set_model;

// Initialize and register all commands
pub fn register_all_commands() {
    // List files command
    register_command(Command {
        name: "list-files".to_string(),
        pattern: Regex::new(r"@list-files\(\s*(\S+)\s*\)").unwrap(),
        description: "List files matching a pattern".to_string(),
        usage_example: "@list-files([wildcard])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @list-files [wildcard]");
                return Ok(None);
            }
            let pattern = &params[0];
            let files = file_module::list_files(pattern)?;
            Ok(Some(format!("\n{}\n", files.join("\n"))))
        },
        section: "file".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(autocomplete_file_path),
    });

    // List folders command
    register_command(Command {
        name: "list-folders".to_string(),
        pattern: Regex::new(r"@list-folders\(\s*(\S+)\s*\)").unwrap(),
        description: "List folders matching a pattern".to_string(),
        usage_example: "@list-folders([wildcard])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @list-folders [wildcard]");
                return Ok(None);
            }
            let pattern = &params[0];
            let folders = file_module::list_folders(pattern)?;
            Ok(Some(folders.join("\n")))
        },
        section: "folder".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(autocomplete_file_path),
    });

    // Read files command
    register_command(Command {
        name: "read-files".to_string(),
        pattern: Regex::new(r"@read-files\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Read multiple files using wildcard pattern into memory".to_string(),
        usage_example: "@read-files([wildcard])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @read-files([wildcard])");
                return Ok(None);
            }
            let pattern = &params[0];
            let files_map = file_module::read_files(pattern)?;

            // Concatenate all file contents into one string
            let mut combined_content = String::new();
            for (filename, content) in &files_map {
                combined_content.push_str(&format!("File: {}\n", filename));
                combined_content.push_str(content);
                combined_content.push_str("\n\n");
            }

            Ok(Some(format!("{}", combined_content)))
        },
        section: "file".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: Some(autocomplete_file_path),
    });

    register_command(Command {
        name: "read-folders".to_string(),
        pattern: Regex::new(r"@read-folders\(\s*(\S+)\s*\)").unwrap(),
        description: "Read multiple folders using wildcard pattern into memory".to_string(),
        usage_example: "@read-folders([wildcard])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @read-folders([wildcard])");
                return Ok(None);
            }
            let pattern = &params[0];
            let files_map = file_module::read_folder(pattern)?;

            // Concatenate all file contents into one string
            let mut combined_content = String::new();
            for (filename, content) in &files_map {
                combined_content.push_str(&format!("File: {}\n", filename));
                combined_content.push_str(content);
                combined_content.push_str("\n\n");
            }

            Ok(Some(format!("{}", combined_content)))
        },
        section: "folder".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: Some(autocomplete_file_path),
    });

    // Read file command
    register_command(Command {
        name: "read-file".to_string(),
        pattern: Regex::new(r"@read-file\(\s*(\S+)\s*\)").unwrap(),
        description: "Read a file into prompt".to_string(),
        usage_example: "@read-file([filename])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @read-file([filename])");
                return Ok(None);
            }
            let filename = &params[0];
            let contents = file_module::read_file(filename)?;

            Ok(Some(format!("File: {}\n{}", filename, contents)))
        },
        section: "file".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: Some(autocomplete_file_path),
    });

    register_command(Command {
        name: "get-memory".to_string(),
        pattern: Regex::new(r"@get-memory\(\s*(\S+)\s*\)").unwrap(),
        description: "Load content from memory into chat".to_string(),
        usage_example: "@get-memory([memory-id])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @get-memory([memory-id])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let memory = chat::get_memory().lock().unwrap();

            match memory.get(memory_id) {
                Some(prompt) => Ok(Some(format!("{}:\n{}\n", memory_id, prompt.value))),
                None => Ok(Some(format!("Error: prompt id {} not found.", memory_id))),
            }
        },
        section: "memory".to_string(),
        command_type: CommandType::LLM,
        autocomplete_handler: Some(autocomplete_memory_id),
    });

    register_command(Command {
        name: "set-alias".to_string(),
        pattern: Regex::new(r"@set-alias\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Load content from memory into chat".to_string(),
        usage_example: "@set-alias([alias-id])".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @set-alias([alias-id])");
                return Ok(None);
            }
            let alias_id = &params[0];
            let alias = &params[0];
            let memory = chat::get_memory().lock().unwrap();

            match memory.get(alias_id) {
                Some(prompt) => Ok(Some(format!(
                    "Error: alias id {} all ready found with content {}.",
                    prompt.id, prompt.value
                ))),
                None => {
                    let prompt = Prompt::new(alias.clone(), PromptType::ALIAS);
                    Ok(Some(format!("Alias {} added.", prompt.id)))
                }
            }
        },
        section: "memory".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(autocomplete_memory_id),
    });

    register_command(Command {
        name: "export".to_string(),
        pattern: Regex::new(r"@export\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Export memory content into file.".to_string(),
        usage_example: "@export(45dge64 or ? or _ or @, ./output.md)".to_string(),
        handler: |params| {
            let mut content = String::new();
            let memory = chat::get_memory().lock().unwrap();

            if params.len() < 2 {
                println!("Usage: @export([id or ? or _ or @],[file-name])");
                return Ok(None);
            }
            let id = &params[0];
            let file_name = &params[1];

            let mut prompt_ordered: Vec<&Prompt> = Vec::new();
            for (_key, val) in memory.iter() {
                prompt_ordered.push(val);
            }
            // Sort prompts by date
            prompt_ordered.sort_by(|a, b| a.date.cmp(&b.date));

            for prompt in prompt_ordered.iter() {
                if id == "@"
                    || *id == prompt.id
                    || (prompt.ptype == PromptType::QUESTION && id == "?")
                    || (prompt.ptype == PromptType::ANSWER && id == "_")
                    || (prompt.ptype == PromptType::ALIAS && id == "^")
                {
                    content.push_str(&format!("{}:\n{}\n", prompt.id, prompt.value));
                }
            }

            // Ensure the directory exists
            if let Some(parent) = Path::new(file_name).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::write(file_name, content)?;
            Ok(Some(format!("File saved {}", file_name)))
        },
        section: "Utility".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(autocomplete_memory_id),
    });

    // Reset context command
    register_command(Command {
        name: "reset-memory".to_string(),
        pattern: Regex::new(r"!reset-memory\(\s*\)").unwrap(),
        description: "Reset the memory".to_string(),
        usage_example: "!reset-memory()".to_string(),
        handler: |_| {
            let mut memory = chat::get_memory().lock().unwrap();
            memory.clear();
            Ok(Some("Memory reset done.".to_string()))
        },
        section: "memory".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    register_command(Command {
        name: "remove-memory".to_string(),
        pattern: Regex::new(r"!remove-memory\(\s*(\S+)\s*\)").unwrap(),
        description: "Remove memory item by id".to_string(),
        usage_example: "!remove-memory([memory-id])".to_string(),
        handler: |params| {
            if params.len() < 1 {
                println!("Usage: !remove-memory([memory-id])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let mut memory = chat::get_memory().lock().unwrap();
            memory.remove(memory_id);
            Ok(Some(format!("Removed memory item {}", memory_id)))
        },
        section: "memory".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(autocomplete_memory_id),
    });

    // set model command
    register_command(Command {
        name: "set-model".to_string(),
        pattern: Regex::new(r"!set-model\(\s*(\S+)\s*\)").unwrap(),
        description: "Set LLM model".to_string(),
        usage_example: "!set-model([model-id])".to_string(),
        handler: |params| {
            if params.len() < 1 {
                println!("Usage: !set-model([model-id])");
                return Ok(None);
            }
            let model_id = &params[0];

            let command = format!("!set-model({})", model_id);
            println!("Starting model change process for {}", model_id);
            match crate::commands::set_model::handle_set_model(&command) {
                Ok(_) => Ok(Some("Model selection complete.".to_string())),
                Err(e) => {
                    println!("Error selecting model: {}", e);
                    Ok(Some(
                        "Failed to set model. See terminal for details.".to_string(),
                    ))
                }
            }
        },
        section: "utility".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(autocomplete_model_id),
    });

    // Register help command and set model command from existing modules
    help::register_help_command();
    bash_cmd::register_bash_command();
}
