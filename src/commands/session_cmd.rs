use regex::Regex;
use crate::commands_registry::{Command, CommandType, register_command};
use crate::session;

pub fn register_session_commands() {
    // Create session command
    register_command(Command {
        name: "session-create".to_string(),
        pattern: Regex::new(r"@session-create\(\s*(\S+)\s*\)").unwrap(),
        description: "Create a new conversation session".to_string(),
        usage_example: "@session-create(project-work)".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @session-create(session-name)");
                return Ok(None);
            }
            let session_name = &params[0];
            match session::create_session(session_name) {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error creating session: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Switch session command
    register_command(Command {
        name: "session-switch".to_string(),
        pattern: Regex::new(r"@session-switch\(\s*(\S+)\s*\)").unwrap(),
        description: "Switch to an existing conversation session".to_string(),
        usage_example: "@session-switch(project-work)".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @session-switch(session-name)");
                return Ok(None);
            }
            let session_name = &params[0];
            match session::switch_session(session_name) {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error switching session: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // List sessions command
    register_command(Command {
        name: "session-list".to_string(),
        pattern: Regex::new(r"@session-list\(\s*\)").unwrap(),
        description: "List all available conversation sessions".to_string(),
        usage_example: "@session-list()".to_string(),
        handler: |_| {
            match session::list_sessions() {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error listing sessions: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Delete session command
    register_command(Command {
        name: "session-delete".to_string(),
        pattern: Regex::new(r"@session-delete\(\s*(\S+)\s*\)").unwrap(),
        description: "Delete a conversation session".to_string(),
        usage_example: "@session-delete(old-session)".to_string(),
        handler: |params| {
            if params.is_empty() {
                println!("Usage: @session-delete(session-name)");
                return Ok(None);
            }
            let session_name = &params[0];
            match session::delete_session(session_name) {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error deleting session: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Export session command
    register_command(Command {
        name: "session-export".to_string(),
        pattern: Regex::new(r"@session-export\(\s*(\S+)\s*,\s*(\S+)\s*\)").unwrap(),
        description: "Export a conversation session to a file".to_string(),
        usage_example: "@session-export(project-work, ./export.md)".to_string(),
        handler: |params| {
            if params.len() < 2 {
                println!("Usage: @session-export(session-name, file-path)");
                return Ok(None);
            }
            let session_name = &params[0];
            let export_path = &params[1];
            match session::export_session(session_name, export_path) {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error exporting session: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Get current session info command
    register_command(Command {
        name: "session-current".to_string(),
        pattern: Regex::new(r"@session-current\(\s*\)").unwrap(),
        description: "Show information about the current session".to_string(),
        usage_example: "@session-current()".to_string(),
        handler: |_| {
            match session::get_current_session_info() {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error getting current session info: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });

    // Save current session command
    register_command(Command {
        name: "session-save".to_string(),
        pattern: Regex::new(r"@session-save\(\s*\)").unwrap(),
        description: "Manually save the current session".to_string(),
        usage_example: "@session-save()".to_string(),
        handler: |_| {
            match session::save_current_session() {
                Ok(message) => Ok(Some(message)),
                Err(e) => Ok(Some(format!("Error saving session: {}", e))),
            }
        },
        section: "session".to_string(),
        command_type: CommandType::NotLLM,
        autocomplete_handler: None,
    });
}