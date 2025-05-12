use crate::{commands, commands_registry, configuration, history};
use lazy_static::lazy_static;
use signal_hook::consts::SIGINT;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

mod input_handler;

// In-memory context
lazy_static! {
    static ref MEMORY: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub fn get_memory() -> &'static Mutex<HashMap<String, String>> {
    &MEMORY
}

#[derive(Debug, PartialEq)]
pub struct Command {
    pub name: String,
    pub parameter: String,
}

async fn execute_command(command: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // First try to execute with the command registry
    let registry_result = commands_registry::execute_command(command);
    if registry_result.is_ok()
        || registry_result
            .as_ref()
            .err()
            .map_or(false, |e| e.to_string() != "Command not found")
    {
        return registry_result;
    }

    println!("Unknown command: {}", command);
    Ok(None)
}

pub async fn check_embedded_commands(input: &str) -> String {
    // Check for embedded commands
    let mut enriched_input = input.to_string();
    let mut pos = 0;
    while pos < enriched_input.len() {
        if enriched_input[pos..].starts_with("@") {
            // Find the end of the command (space, newline, or end of string)
            let end = enriched_input[pos..]
                .find(|c: char| c == ' ' || c == '\n')
                .map(|x| x + pos)
                .unwrap_or(enriched_input.len());

            // Extract the full command including parameters
            // Need to find the closing parenthesis for commands with the new format
            let command_start = pos;
            let command_text = &enriched_input[pos..];

            // Find the end of the command - either at the next newline or after the closing parenthesis
            let command_end = if command_text.contains('(') {
                let paren_pos = command_text.find('(').unwrap_or(0) + pos;
                let remaining = &enriched_input[paren_pos..];
                let closing_paren = remaining.find(')').map(|x| x + paren_pos + 1);

                match closing_paren {
                    Some(end) => end,
                    None => enriched_input[pos..]
                        .find('\n')
                        .map(|x| x + pos)
                        .unwrap_or(enriched_input.len()),
                }
            } else {
                enriched_input[pos..]
                    .find('\n')
                    .map(|x| x + pos)
                    .unwrap_or(enriched_input.len())
            };

            let command = &enriched_input[command_start..command_end];

            println!("Executing command: {}", command);

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
    enriched_input
}
pub async fn chat_loop() -> Result<(), Box<dyn std::error::Error>> {
    // Register all commands
    commands::register_all_commands();

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
            commands_registry::print_help();
        } else if input.starts_with("@") {
            match execute_command(&input).await {
                Ok(Some(output)) => {
                    println!("{}", output);
                }
                Ok(None) => {
                    println!("Sorry - Unrecognized command...");
                    commands_registry::print_help();
                }
                Err(e) => {
                    println!("Error executing command: {}", e);
                }
            }
        } else {
            let enriched_input = check_embedded_commands(&input).await;

            println!("OpenRouter input: {}", enriched_input.to_string());
            let response = "dummy"; // openrouter::call_openrouter_api(&api_key, &input).await?;
            println!("OpenRouter: {}", response);
        }
    }

    history::save_prompt_history(&prompt_history).await?;

    Ok(())
}
