use regex::Regex;
use std::fs;
use std::path::Path;

use crate::chat;
use crate::commands_registry::{Command, register_command};
use crate::files::files as file_module;

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
            Ok(Some(files.join("\n")))
        },
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
    });

    // Read files command
    register_command(Command {
        name: "read-files".to_string(),
        pattern: Regex::new(r"@read-files\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Read multiple files using wildcard pattern into memory".to_string(),
        usage_example: "@read-files([memory-id], [wildcard])".to_string(),
        handler: |params| {
            if params.len() < 2 {
                println!("Usage: @read-files([memory-id], [wildcard])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let pattern = &params[1];
            let files_map = file_module::read_files(pattern)?;

            // Concatenate all file contents into one string
            let mut combined_content = String::new();
            for (filename, content) in &files_map {
                combined_content.push_str(&format!("File: {}\n", filename));
                combined_content.push_str(content);
                combined_content.push_str("\n\n");
            }

            // Save to memory
            let mut memory = chat::get_memory().lock().unwrap();
            memory.insert(memory_id.to_string(), combined_content.clone());

            Ok(Some(format!(
                "Files matching pattern '{}' read into memory id '{}'",
                pattern, memory_id
            )))
        },
    });

    register_command(Command {
        name: "read-folders".to_string(),
        pattern: Regex::new(r"@read-folders\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Read multiple folders using wildcard pattern into memory".to_string(),
        usage_example: "@read-folders([memory-id], [wildcard])".to_string(),
        handler: |params| {
            if params.len() < 2 {
                println!("Usage: @read-folders([memory-id], [wildcard])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let pattern = &params[1];
            let files_map = file_module::read_folder(pattern)?;

            // Concatenate all file contents into one string
            let mut combined_content = String::new();
            for (filename, content) in &files_map {
                combined_content.push_str(&format!("File: {}\n", filename));
                combined_content.push_str(content);
                combined_content.push_str("\n\n");
            }

            // Save to memory
            let mut memory = chat::get_memory().lock().unwrap();
            memory.insert(memory_id.to_string(), combined_content.clone());

            Ok(Some(format!(
                "Files from folders matching pattern '{}' read into memory id '{}'",
                pattern, memory_id
            )))
        },
    });

    // Read file command
    register_command(Command {
        name: "read-file".to_string(),
        pattern: Regex::new(r"@read-file\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Read a file into memory".to_string(),
        usage_example: "@read-file([memory-id], [filename])".to_string(),
        handler: |params| {
            if params.len() < 2 {
                println!("Usage: @read-file([memory-id], [filename])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let filename = &params[1];
            let contents = file_module::read_file(filename)?;

            // We're using the existing chat module to manage memory
            // This is just a placeholder - you'll need to adjust this based on how memory is actually handled
            let mut memory = chat::get_memory().lock().unwrap();
            memory.insert(memory_id.to_string(), contents.clone());

            Ok(Some(format!(
                "File {} read into memory id {}",
                filename, memory_id
            )))
        },
    });

    // Save file command
    register_command(Command {
        name: "save-file".to_string(),
        pattern: Regex::new(r"@save-file\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Save content to a file".to_string(),
        usage_example: "@save-file([memory-id], [filename])".to_string(),
        handler: |params| {
            if params.len() < 2 {
                println!("Usage: @save-file([memory-id], [filename])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let memory = chat::get_memory().lock().unwrap();
            let content = memory
                .get(&memory_id.to_string())
                .expect("Memory ID not found");

            let filename = &params[1];

            // Ensure the directory exists
            if let Some(parent) = Path::new(filename).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            fs::write(filename, content)?;

            Ok(Some(format!("Content saved to file {}", filename)))
        },
    });

    // Save file command
    register_command(Command {
        name: "save-all".to_string(),
        pattern: Regex::new(r"@save-all\(\s*(\S+)\s*\)").unwrap(),
        description: "Save content to a file".to_string(),
        usage_example: "@save-all([filename])".to_string(),
        handler: |params| {
            if params.len() < 1 {
                println!("Usage: @save-all([filename])");
                return Ok(None);
            }
            let filename = &params[0];
            let memory = chat::get_memory().lock().unwrap();
            let json = serde_json::to_string(&*memory)?;
            std::fs::write(filename, json)?;

            Ok(Some(format!("Content saved to file {}", filename)))
        },
    });

    register_command(Command {
        name: "get-memory".to_string(),
        pattern: Regex::new(r"@get-memory\(\s*(\S+)\s*\)").unwrap(),
        description: "Load content from memory into chat".to_string(),
        usage_example: "@get-memory([memory-id])".to_string(),
        handler: |params| {
            if params.len() < 1 {
                println!("Usage: @get-memory([memory-id])");
                return Ok(None);
            }
            let memory_id = &params[0];
            let memory = chat::get_memory().lock().unwrap();
            let content = memory.get(memory_id);

            let not_found = "Memory id not found".to_string();
            Ok(Some(format!(
                "```{}:\n\n{}\n```",
                memory_id,
                content.unwrap_or_else(|| &not_found)
            )))
        },
    });

    register_command(Command {
        name: "dump-memory".to_string(),
        pattern: Regex::new(r"@dump-memory\(\s*\)").unwrap(),
        description: "Dump all memory content into chat".to_string(),
        usage_example: "@dump-memory()".to_string(),
        handler: |_| {
            let mut content = String::new();
            let memory = chat::get_memory().lock().unwrap();

            for (key, value) in memory.iter() {
                content.push_str(&format!("\n```\n{}:\n{}\n```\n", key, value));
            }

            Ok(Some(content))
        },
    });

    // Reset context command
    register_command(Command {
        name: "reset-memory".to_string(),
        pattern: Regex::new(r"@reset-memory\(\s*\)").unwrap(),
        description: "Reset the memory".to_string(),
        usage_example: "@reset-memory()".to_string(),
        handler: |_| {
            let mut memory = chat::get_memory().lock().unwrap();
            memory.clear();
            Ok(Some("Memory reset done.".to_string()))
        },
    });

    // set model command
    register_command(Command {
        name: "set-model".to_string(),
        pattern: Regex::new(r"@set-model\(\s*(\S+)\s*\)").unwrap(),
        description: "Set LLM model".to_string(),
        usage_example: "@set-model([model-id])".to_string(),
        handler: |params| {
            if params.len() < 1 {
                println!("Usage: @set-model([model-id])");
                return Ok(None);
            }
            let model_id = &params[0];

            let command = format!("@set-model({})", model_id);
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
    });

    // Register help command and set model command from existing modules
    help::register_help_command();
}
