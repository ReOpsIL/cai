use crate::terminal;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

pub type CommandHandlerOutputType = Result<Option<String>, Box<dyn std::error::Error>>;

pub type CommandHandler = fn(&[String]) -> CommandHandlerOutputType;
use rustyline::completion::Pair as Completion;
use rustyline::error::ReadlineError;

pub type AutocompleteHandler = fn(&str, usize) -> Result<(usize, Vec<Completion>), ReadlineError>;

#[derive(Clone, PartialEq, Debug)]
pub enum CommandType {
    NotLLM,
    LLM,
    Terminal,
}

#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub pattern: Regex,
    pub description: String,
    pub usage_example: String,
    pub handler: CommandHandler,
    pub section: String,
    pub command_type: CommandType,
    pub autocomplete_handler: Option<AutocompleteHandler>, // Add autocomplete handler field
}

impl Command {
    pub fn new(
        name: String,
        pattern: Regex,
        description: String,
        usage_example: String,
        handler: CommandHandler,
        section: String,
    ) -> Self {
        Self {
            name,
            pattern,
            description,
            usage_example,
            handler,
            section,
            command_type: CommandType::NotLLM,
            autocomplete_handler: None,
        }
    }
}

pub struct CommandHandlerResult {
    pub command_output: CommandHandlerOutputType,
    pub command: Command,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("pattern", &self.pattern.as_str())
            .field("description", &self.description)
            .field("usage_example", &self.usage_example)
            .field("section", &self.section)
            .field("autocomplete_handler", &self.autocomplete_handler.is_some())
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

pub fn execute_command(
    input: &str,
) -> Result<Option<CommandHandlerResult>, Box<dyn std::error::Error>> {
    if let Some(command_result) = parse_command(input) {
        if let Some(command) = get_command(&command_result.command_name) {
            let command_handler_output = (command.handler)(&command_result.parameters);

            return Ok(Some(CommandHandlerResult {
                command_output: command_handler_output,
                command: command,
            }));
        }
    }

    Err("Command not found".into())
}

// Modified print_help function to group commands by section
pub fn print_help() {
    let registry = COMMAND_REGISTRY.lock().unwrap();
    println!("{}", terminal::format_info("Available commands:"));

    use std::collections::BTreeMap;
    let mut sections: BTreeMap<String, Vec<&Command>> = BTreeMap::new();

    // Group commands by section
    for command in registry.values() {
        sections
            .entry(command.section.clone())
            .or_default()
            .push(command);
    }

    // Sort commands within each section by name
    for commands in sections.values_mut() {
        commands.sort_by_key(|c| &c.name);
    }

    // Print sections and commands
    for (section_name, commands) in sections {
        // Capitalize the first letter of the section name for the headline
        let formatted_section_name = if let Some(first_char) = section_name.chars().next() {
            format!(
                "{}{}",
                first_char.to_uppercase(),
                section_name.chars().skip(1).collect::<String>()
            )
        } else {
            section_name.clone()
        };

        println!(
            "\n{}",
            terminal::yellow(&format!("--- {} Commands ---", formatted_section_name))
        );
        for command in commands {
            println!(
                "  {} - {}",
                terminal::cyan(&command.usage_example),
                terminal::white(&command.description)
            );
        }
    }
}
