use std::io::Write;
use tokio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use signal_hook::{consts::SIGINT};
use std::collections::HashMap;
use serde_json;
use crate::files::files as file_module;
use lazy_static::lazy_static;
use std::sync::Mutex;

use crate::history;
use crate::configuration;
use crate::command_handler::{self, handle_command};
use crate::openrouter;
mod input_handler;

// In-memory context
lazy_static! {
    static ref MEMORY: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

fn get_memory() -> &'static Mutex<HashMap<String, String>> {
    &MEMORY
}

pub async fn reset_context() -> Result<(), Box<dyn std::error::Error>> {
    let memory = get_memory();
    let mut memory = memory.lock().unwrap();
    memory.clear();
    println!("Context reset.");
    Ok(())
}

pub async fn save_context(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let memory = get_memory();
    let memory = memory.lock().unwrap();
    let json = serde_json::to_string(&*memory)?;
    std::fs::write(file_name, json)?;
    println!("Context saved to {}.", file_name);
    Ok(())
}

async fn execute_command(command: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    match parts[0] {
        "@list-files" => {
            if parts.len() < 2 {
                println!("Usage: @list-files [wildcard]");
                return Ok(None);
            }
            let pattern = parts[1];
            let files = file_module::list_files(pattern)?;
            Ok(Some(files.join("\n")))
        }
        "@list-folders" => {
            if parts.len() < 2 {
                println!("Usage: @list-folders [wildcard]");
                return Ok(None);
            }
            let pattern = parts[1];
            let folders = file_module::list_folders(pattern)?;
            Ok(Some(folders.join("\n")))
        }
        "@read-file" => {
            if parts.len() < 3 {
                println!("Usage: @read-file [memory-id] [filename]");
                return Ok(None);
            }
            let memory_id = parts[1];
            let filename = parts[2];
            let contents = file_module::read_file(filename)?;
            let memory = get_memory();
            let mut memory = memory.lock().unwrap();
            memory.insert(memory_id.to_string(), contents.clone());
            Ok(Some(format!("File {} read into memory id {}", filename, memory_id)))
        }
        "@read-files" => {
            if parts.len() < 3 {
                println!("Usage: @read-files [memory-id] [wildcard]");
                return Ok(None);
            }
            let memory_id = parts[1];
            let pattern = parts[2];
            let files = file_module::read_files(pattern)?;
            let memory = get_memory();
            let mut memory = memory.lock().unwrap();
            memory.insert(memory_id.to_string(), serde_json::to_string(&files)?);
            Ok(Some(format!("Files matching {} read into memory id {}", pattern, memory_id)))
        }
        "@read-folder" => {
            if parts.len() < 3 {
                println!("Usage: @read-folder [memory-id] [wildcard]");
                return Ok(None);
            }
            let memory_id = parts[1];
            let pattern = parts[2];
            let folder_contents = file_module::read_folder(pattern)?;
            let memory = get_memory();
            let mut memory = memory.lock().unwrap();
            memory.insert(memory_id.to_string(), serde_json::to_string(&folder_contents)?);
            Ok(Some(format!("Folder contents matching {} read into memory id {}", pattern, memory_id)))
        }
        "@reset-context" => {
            reset_context().await?;
            Ok(None)
        }
        "@save-context" => {
            if parts.len() < 2 {
                println!("Usage: @save-context [file-name]");
                return Ok(None);
            }
            let file_name = parts[1];
            save_context(file_name).await?;
            Ok(None)
        }
        _ => {
            println!("Unknown command: {}", command);
            Ok(None)
        }
    }
}

pub async fn chat_loop() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    let config = configuration::load_configuration()?;

    println!("Loaded config: {:?}", config);

    let mut prompt_history: Vec<String> = Vec::new();
    let max_prompts = 10; // Maximum number of prompts to store

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&term))?;

    loop {
        let mut input = match input_handler::get_input().await {
            Ok(input) => input,
            Err(_) => break, // Handle Ctrl-d
        };

        input = input.trim().to_owned();

        // Store the prompt in history
        prompt_history.push(input.to_string());
        if prompt_history.len() > max_prompts {
            prompt_history.remove(0); // Remove the oldest prompt
        }

        if term.load(Ordering::Relaxed) || input.eq_ignore_ascii_case("exit") {
            break;
        }

        if input.eq("?") {
            command_handler::handle_command("@help").await?;
        }
        else if input.starts_with("@") {
            match execute_command(&input).await {
                Ok(Some(output)) => {
                    println!("{}", output);
                }
                Ok(None) => {}
                Err(e) => {
                    println!("Error executing command: {}", e);
                }
            }
        } else {
            // Check for embedded commands
            let mut enriched_input = input.to_string();
            let mut pos = 0;
            while pos < enriched_input.len() {
                if enriched_input[pos..].starts_with("@") {
                    let end = enriched_input[pos..].find(|c: char| c == ' ' || c == '\n').map(|x| x + pos).unwrap_or(enriched_input.len());
                    let command = &enriched_input[pos..end];
                    match execute_command(command).await {
                        Ok(Some(output)) => {
                            // Inject the output into the prompt
                            enriched_input.replace_range(pos..end, &format!("\n```\n{}\n```\n", output));
                            pos += format!("\n```\n{}\n```\n", output).len();
                        }
                        Ok(None) => {
                            pos = end;
                        }
                        Err(e) => {
                            println!("Error executing command: {}", e);
                            pos = end;
                        }
                    }
                } else {
                    pos += 1;
                }
            }

            let memory = get_memory();
            let memory = memory.lock().unwrap();

            // Enrich prompt with memory contents
            for (key, value) in memory.iter() {
                enriched_input.push_str(&format!("\n```\n{}:\n{}\n```\n", key, value));
            }
            input = enriched_input
        }

        let response = openrouter::call_openrouter_api(&api_key, &input).await?;
        println!("OpenRouter: {}", response);
    }

    history::save_prompt_history(&prompt_history).await?;

    Ok(())
}
