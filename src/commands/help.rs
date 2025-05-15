use crate::commands_registry::{Command, register_command};
use regex::Regex;

pub fn register_help_command() {
    register_command(Command {
        name: "help".to_string(),
        pattern: Regex::new(r"@help\(\s*\)").unwrap(),
        description: "Display help information for available commands".to_string(),
        usage_example: "@help()".to_string(),
        handler: |_| {
            crate::commands_registry::print_help();
            Ok(None)
        },
        section: "Help".to_string(),
    });
}
