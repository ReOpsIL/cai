use crate::commands_registry::{Command, CommandType, register_command};
use regex::Regex;
use rustyline::completion::Pair as Completion;
use rustyline::error::ReadlineError;

pub fn register_help_command() {
    register_command(Command {
        name: "help".to_string(),
        pattern: Regex::new(r"@help\(\s*(\S*)\s*\)").unwrap(),
        description: "Display help information for available commands".to_string(),
        usage_example: "@help([topic])".to_string(),
        handler: |params| {
            if params.is_empty() || params[0].is_empty() {
                crate::commands_registry::print_help();
            } else if params[0] == "autocomplete" {
                println!("\n--- Autocomplete Functionality ---\n");
                println!(
                    "This application supports intelligent autocomplete for commands and their parameters:"
                );
                println!("- Press TAB while typing a command to autocomplete the command name");
                println!(
                    "- Press TAB after typing a command's opening parenthesis to see available parameter options"
                );
                println!("- Command parameters use context-aware completion:");
                println!("  • File commands (@read-file, @list-files, etc.) - Fuzzy path matching");
                println!("  • Memory commands (@get-memory, @export, etc.) - Memory ID matching");
                println!("  • Model commands (!set-model) - Model ID matching");
                println!("  • Option commands (@select-option, etc.) - Predefined option matching");
                println!(
                    "\nTip: You can type partial text and press TAB to see fuzzy matched suggestions"
                );
            }
            Ok(None)
        },
        command_type: CommandType::NotLLM,
        autocomplete_handler: Some(
            |line, pos| -> Result<(usize, Vec<Completion>), ReadlineError> {
                // Extract parameter information
                let text_before_cursor = &line[..pos];
                if let Some(paren_pos) = text_before_cursor.rfind('(') {
                    let params_text = &text_before_cursor[(paren_pos + 1)..];
                    let param_start = params_text.trim();
                    let arg_start_pos = paren_pos + 1 + (params_text.len() - param_start.len());

                    // Define available help topics
                    let options = &["", "autocomplete"];

                    // Score options based on fuzzy matching
                    let mut scored_options: Vec<_> = Vec::new();
                    for option in options {
                        if let Some(score) = crate::input_handler::fuzzy_match(option, param_start)
                        {
                            scored_options.push((score, option.to_string()));
                        }
                    }

                    // Sort by score (higher is better)
                    scored_options.sort_by(|a, b| b.0.cmp(&a.0));

                    // Convert to completion entries
                    let matching_options: Vec<Completion> = scored_options
                        .into_iter()
                        .map(|(_, option)| Completion {
                            display: option.clone(),
                            replacement: option,
                        })
                        .collect();

                    return Ok((arg_start_pos, matching_options));
                }

                Ok((pos, vec![]))
            },
        ),
        section: "Help".to_string(),
    });
}
