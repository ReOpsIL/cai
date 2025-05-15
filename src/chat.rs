use crate::{commands, commands_registry, configuration, openrouter, terminal};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use signal_hook::consts::SIGINT;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

mod input_handler;

// In-memory context
lazy_static! {
    static ref MEMORY: Mutex<HashMap<String, Prompt>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptType {
    QUESTION,
    ANSWER,
    ALIAS,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub id: String,
    pub date: DateTime<Utc>,
    pub value: String,
    pub ptype: PromptType,
}

impl Prompt {
    pub fn new(value: String, ptype: PromptType) -> Self {
        let prompt = Prompt {
            id: uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("")
                .to_string(),
            date: Utc::now(),
            value,
            ptype,
        };
        let mut memory = get_memory().lock().unwrap();
        memory.insert(prompt.id.clone(), prompt.clone());

        println!("ID: {}\n", terminal::magenta(&prompt.id));

        prompt
    }
}

pub fn get_memory() -> &'static Mutex<HashMap<String, Prompt>> {
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

    println!(
        "{}",
        terminal::format_error(&format!("Unknown command: {}", command))
    );
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

            //println!("Executing command: {}", command);

            match execute_command(command).await {
                Ok(Some(output)) => {
                    // Inject the output into the prompt
                    let formated_output = &format!("{}", output);
                    enriched_input.replace_range(pos..end, formated_output);
                    pos += formated_output.len();
                }
                Ok(None) => {
                    pos = end;
                }
                Err(e) => {
                    println!(
                        "{}",
                        terminal::format_error(&format!("Error executing command: {}", e))
                    );
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

    println!(
        "{}",
        terminal::format_info(&format!("Loaded config: {:?}", config))
    );

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&term))?;

    loop {
        let mut input = match input_handler::get_input().await {
            Ok(input) => input,
            Err(e) => {
                if e.to_string().contains("Input interrupted")
                    || e.to_string().contains("Input terminated")
                {
                    println!("\n{}", terminal::format_success("Goodbye!"));
                    break;
                } else {
                    println!(
                        "{}",
                        terminal::format_error(&format!("Error reading input: {}", e))
                    );
                    continue;
                }
            }
        };

        input = input.trim().to_owned();

        if term.load(Ordering::Relaxed) {
            println!("\n{}", terminal::format_warning("Interrupted, exiting..."));
            break;
        }

        if input.eq_ignore_ascii_case("exit") {
            println!("{}", terminal::format_success("Goodbye!"));
            break;
        }

        if input.eq("?") {
            commands_registry::print_help();
        } else if input.starts_with("!") {
            match execute_command(&input).await {
                Ok(Some(output)) => {
                    println!("{}", output);
                }
                Ok(None) => {
                    println!(
                        "{}",
                        terminal::format_error("Sorry - Unrecognized command...")
                    );
                    commands_registry::print_help();
                }
                Err(e) => {
                    println!("Error executing command: {}", e);
                }
            }
        } else {
            let enriched_input = check_embedded_commands(&input).await;

            Prompt::new(enriched_input.clone(), PromptType::QUESTION);

            // println!("{}\n+++++++++++++++++++\n{}", terminal::cyan("You:"), enriched_input.to_string());
            //let response = String::from(":-) Ok");
            let response = openrouter::call_openrouter_api(&enriched_input).await?;

            println!("{}", response);
            Prompt::new(response.clone(), PromptType::ANSWER);
        }
    }

    Ok(())
}
