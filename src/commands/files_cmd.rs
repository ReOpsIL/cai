use regex::Regex;
use crate::commands_registry::{register_command, Command, CommandType};
use crate::files::files as file_module; // Import autocomplete handlers
use crate::autocomplete::{autocomplete_file_path, autocomplete_memory_id};
use crate::chat;

pub fn register_files_cmd() {
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
}
