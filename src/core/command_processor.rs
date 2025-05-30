use crate::commands_registry::{CommandHandlerResult, CommandType};
use crate::{commands_registry, utils::terminal};

#[derive(Debug, PartialEq)]
pub struct Command {
    pub name: String,
    pub parameter: String,
}

#[derive(Clone)]
pub struct CommandProcessor;

impl CommandProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_command(
        &self,
        command: &str,
    ) -> Result<Option<CommandHandlerResult>, Box<dyn std::error::Error>> {
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
            terminal::format_error(&format!("Unknown command: {} ", command))
        );
        Ok(None)
    }

    pub fn check_embedded_commands(&self, input: &str) -> (String, bool) {
        let mut enriched_input = input.to_string();
        let mut current_pos = 0;
        let mut offline = false;

        while current_pos < enriched_input.len() {
            if let Some((command, command_len_in_original_segment)) =
                self.parse_command_from_input(&enriched_input[current_pos..])
            {
                // The `command_len_in_original_segment` is the length of the command string itself,
                // which is also the length to replace in the `enriched_input` starting from `current_pos`.
                let command_end_in_enriched_input = current_pos + command_len_in_original_segment;

                match self.execute_command(&command) {
                    Ok(Some(output)) => {
                        if output.command.command_type == CommandType::Terminal
                            || output.command.command_type == CommandType::NotLLM
                        {
                            offline = true;
                        }
                        match output.command_output {
                            Ok(Some(s)) => {
                                let formatted_output = format!("{}", s);
                                enriched_input.replace_range(
                                    current_pos..command_end_in_enriched_input,
                                    &formatted_output,
                                );
                                current_pos += formatted_output.len();
                            }
                            Ok(None) => {
                                // Command succeeded but no output, advance past the command
                                println!(
                                    "Calling command {} succeeded, but no returned value was present.",
                                    command
                                );
                                current_pos = command_end_in_enriched_input;
                            }
                            Err(e) => {
                                // Command execution resulted in an error
                                println!("An error occurred while calling command {}: {}", command, e);
                                current_pos = command_end_in_enriched_input;
                            }
                        }
                    }
                    Ok(None) => {
                        // Command not found or no action taken by command
                        // It's important to advance past the parsed command to avoid infinite loops
                        current_pos = command_end_in_enriched_input;
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            terminal::format_error(&format!(
                                "Error executing command {}: {}",
                                command, e
                            ))
                        );
                        current_pos = command_end_in_enriched_input;
                    }
                }
            } else {
                current_pos += 1;
            }
        }
        (enriched_input, offline)
    }

    // Helper function to parse command and its length from input segment
    fn parse_command_from_input(&self, input_segment: &str) -> Option<(String, usize)> {
        if !input_segment.starts_with("@") {
            return None;
        }

        // Determine the initial end of the command name part (before parameters)
        let command_name_end = input_segment
            .find(|c: char| c == ' ' || c == '\n' || c == '(')
            .unwrap_or(input_segment.len());

        let _potential_command_text = &input_segment[..command_name_end];

        // Now, determine the actual end of the full command including parameters
        let command_actual_end = if input_segment[command_name_end..].starts_with('(') {
            // Command has parameters in parentheses
            let mut paren_level = 0;
            let mut in_string = false;
            let mut last_char_was_escape = false;
            let mut end_idx = command_name_end; // Start searching from after the command name

            for (i, char_code) in input_segment[command_name_end..].char_indices() {
                end_idx = command_name_end + i + char_code.len_utf8();
                if last_char_was_escape {
                    last_char_was_escape = false;
                    continue;
                }
                match char_code {
                    '\\' => last_char_was_escape = true,
                    '"' if !last_char_was_escape => in_string = !in_string,
                    '(' if !in_string => paren_level += 1,
                    ')' if !in_string => {
                        paren_level -= 1;
                        if paren_level == 0 {
                            break; // Found matching closing parenthesis
                        }
                    }
                    _ => {}
                }
            }
            if paren_level == 0 {
                end_idx // End is after the closing parenthesis
            } else {
                // Mismatched parentheses, fallback to newline or end of string
                input_segment
                    .find('\n')
                    .unwrap_or(input_segment.len())
            }
        } else {
            // Command does not have parameters in parentheses, ends at space or newline
            command_name_end
        };
        
        let command_str = input_segment[..command_actual_end].to_string();
        Some((command_str, command_actual_end))
    }
}