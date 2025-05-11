use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

pub type CommandHandler = fn(&[String]) -> Result<Option<String>, Box<dyn std::error::Error>>;

#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub pattern: Regex,
    pub description: String,
    pub usage_example: String,
    pub handler: CommandHandler,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("pattern", &self.pattern.as_str())
            .field("description", &self.description)
            .field("usage_example", &self.usage_example)
            .finish()
    }
}

pub struct CommandResult {
    pub command_name: String,
    pub parameters: Vec<String>,
}

lazy_static! {
    static ref COMMAND_REGISTRY: Mutex<HashMap<String, Command>> = Mutex::new(HashMap::new());
}

pub fn register_command(command: Command) {
    let mut registry = COMMAND_REGISTRY.lock().unwrap();
    registry.insert(command.name.clone(), command);
}

pub fn get_command(name: &str) -> Option<Command> {
    let registry = COMMAND_REGISTRY.lock().unwrap();
    registry.get(name).cloned()
}

#[allow(dead_code)]
pub fn get_all_commands() -> Vec<Command> {
    let registry = COMMAND_REGISTRY.lock().unwrap();
    registry.values().cloned().collect()
}

pub fn parse_command(input: &str) -> Option<CommandResult> {
    let registry = COMMAND_REGISTRY.lock().unwrap();

    for command in registry.values() {
        if let Some(captures) = command.pattern.captures(input) {
            let mut parameters = Vec::new();

            // Skip the first capture (which is the entire match)
            for i in 1..captures.len() {
                if let Some(capture) = captures.get(i) {
                    parameters.push(capture.as_str().to_string());
                }
            }

            return Some(CommandResult {
                command_name: command.name.clone(),
                parameters,
            });
        }
    }

    None
}

pub fn execute_command(input: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(command_result) = parse_command(input) {
        if let Some(command) = get_command(&command_result.command_name) {
            return (command.handler)(&command_result.parameters);
        }
    }

    Ok(None)
}

pub fn print_help() {
    let registry = COMMAND_REGISTRY.lock().unwrap();
    println!("Available commands:");

    for command in registry.values() {
        println!("  {} - {}", command.usage_example, command.description);
    }
}
