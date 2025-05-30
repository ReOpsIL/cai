use crate::chat::get_memory;
use crate::services::llm_client::LLMClient;
use rustyline::completion::Pair as Completion;
use rustyline::error::ReadlineError;
use std::fs;
use std::path::{Path, PathBuf};
use crate::{commands_registry};
use lazy_static::lazy_static;
use regex::Regex;
use rustyline::highlight::CmdKind;
use std::sync::OnceLock;
use std::sync::Mutex;
use rustyline::config::Configurer;

// Custom implementation of a colored prompt with autocompletion
#[derive(Default)]
pub struct ColoredPrompt {}

impl rustyline::validate::Validator for ColoredPrompt {}
impl rustyline::Helper for ColoredPrompt {}

lazy_static! {
    pub static ref RL_EDITOR: Mutex<rustyline::Editor::<ColoredPrompt, rustyline::history::DefaultHistory>> = {
        let mut rl = rustyline::Editor::<ColoredPrompt, rustyline::history::DefaultHistory>::new()
            .expect("Failed to initialize Rustyline editor"); // Handle error, panic if critical

        rl.set_helper(Some(ColoredPrompt::default()));
        rl.set_auto_add_history(true);
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }
        Mutex::new(rl)
    };
}
static COMMAND_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_command_regex() -> &'static Regex {
    COMMAND_REGEX.get_or_init(|| {
        Regex::new(r"([@!]\w+(?:\([^)]*\))?)\s*(\S*)").expect("Failed to compile command regex")
    })
}

// Colorize commands for display
fn colorize_commands(input: &str) -> String {
    let regex = get_command_regex();
    let mut result = input.to_string();
    let mut offset = 0;

    for cap in regex.find_iter(input) {
        let start = cap.start() + offset;
        let end = cap.end() + offset;

        // Insert ANSI color codes
        result.insert_str(end, "\x1b[0m"); // Reset color
        result.insert_str(start, "\x1b[36m"); // Cyan color

        // Update offset for subsequent replacements
        offset += 9; // Length of color codes (5 for cyan, 4 for reset)
    }

    result
}

// Simple fuzzy matching algorithm
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();

    // Exact match gets highest score
    if haystack_lower == needle_lower {
        return Some(100);
    }

    // Starting with the needle is next best
    if haystack_lower.starts_with(&needle_lower) {
        return Some(90);
    }

    // Check if all characters in needle exist in haystack in sequence
    let haystack_chars: Vec<char> = haystack_lower.chars().collect();
    let needle_chars: Vec<char> = needle_lower.chars().collect();

    let mut h_idx = 0;
    let mut n_idx = 0;
    let mut last_match_pos = 0;
    let mut consecutive = 0;
    let mut max_consecutive = 0;

    while h_idx < haystack_chars.len() && n_idx < needle_chars.len() {
        if haystack_chars[h_idx] == needle_chars[n_idx] {
            if last_match_pos + 1 == h_idx {
                consecutive += 1;
            } else {
                consecutive = 1;
            }

            max_consecutive = max_consecutive.max(consecutive);
            last_match_pos = h_idx;
            n_idx += 1;
        }
        h_idx += 1;
    }

    if n_idx == needle_chars.len() {
        // All characters matched - calculate score based on:
        // 1. Position of first match (earlier is better)
        // 2. Consecutive matches (more is better)
        // 3. Total matched vs. haystack length (higher ratio is better)
        let position_score = 40 - ((last_match_pos * 40) / haystack_chars.len()).min(40);
        let consecutive_score = max_consecutive * 40 / needle_chars.len().max(1);
        let coverage_score = (needle_chars.len() * 20) / haystack_chars.len().max(1);

        Some(position_score + consecutive_score + coverage_score)
    } else {
        None
    }
}


impl rustyline::hint::Hinter for ColoredPrompt {
    type Hint = String;
}

impl rustyline::highlight::Highlighter for ColoredPrompt {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        std::borrow::Cow::Owned(colorize_commands(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _cmd_kind: CmdKind) -> bool {
        true
    }
}

// Implement tab completion for commands and parameters
impl rustyline::completion::Completer for ColoredPrompt {
    type Candidate = Completion;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Completion>), ReadlineError> {
        let text_before_cursor = &line[..pos];

        // Determine the command prefix and the start position of the command name
        let (command_prefix_char, command_name_start_pos) = if text_before_cursor.starts_with('!') {
            ('!', 1)
        } else if let Some(at_pos) = text_before_cursor.rfind('@') {
            ('@', at_pos + 1)
        } else {
            return Ok((pos, vec![]));
        };

        // Get all registered commands
        let commands = commands_registry::get_all_commands();

        // Check if we're completing a command name or its parameters
        if let Some(paren_pos) = text_before_cursor.rfind('(') {
            // We're inside parameters - find the command name
            let cmd_text = &text_before_cursor[..paren_pos];
            let cmd_name = cmd_text.trim_start_matches(command_prefix_char).trim();

            // Find the matching command
            if let Some(command) = commands.iter().find(|cmd| cmd.name == cmd_name) {
                // If command has a specific autocomplete handler, use it
                if let Some(handler) = command.autocomplete_handler {
                    return handler(line, pos);
                }
            }
            return Ok((pos, vec![]));
        } else {
            // We're completing a command name
            let typed_prefix = &text_before_cursor[command_name_start_pos..];

            // Find matching commands using fuzzy matching
            let mut scored_commands = Vec::new();
            for cmd in &commands {
                if let Some(score) = fuzzy_match(&cmd.name, typed_prefix) {
                    scored_commands.push((score, cmd));
                }
            }

            // Sort by score (higher is better)
            scored_commands.sort_by(|a, b| b.0.cmp(&a.0));

            // Convert to completion entries
            let matching_commands = scored_commands
                .into_iter()
                .map(|(_, cmd)| Completion {
                    display: format!("{}{}", command_prefix_char, cmd.name),
                    replacement: cmd.usage_example.clone()[1..].to_string(),
                })
                .collect();

            return Ok((command_name_start_pos, matching_commands));
        }
    }
}



// Helper function to extract parameter information from command line
fn extract_parameter_info(text: &str) -> Option<(usize, usize, &str)> {
    // Find opening parenthesis
    let paren_pos = text.rfind('(')?;

    // Extract parameters text
    let params_text = &text[(paren_pos + 1)..];

    // Split by commas to get individual parameters
    let params: Vec<&str> = params_text.split(',').collect();

    // Find current parameter (the last one)
    let param_index = params.len() - 1;
    let current_param = params[param_index].trim();

    // Calculate start position of current parameter
    let param_start_pos = paren_pos + 1 + params_text.len() - params[param_index].len();

    Some((param_index, param_start_pos, current_param))
}

// Autocomplete handler for model IDs
pub fn autocomplete_empty(
    _line: &str,
    _pos: usize,
) -> Result<(usize, Vec<Completion>), ReadlineError> {
    Ok((0, vec![]))
}

// Autocomplete handler for file paths
pub fn autocomplete_file_path(
    line: &str,
    pos: usize,
) -> Result<(usize, Vec<Completion>), ReadlineError> {
    if let Some((_, param_start_pos, param_text)) = extract_parameter_info(&line[..pos]) {
        let current_input =
            param_text.trim_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace());

        // Parse the path to get directory and file prefix
        let base_path = PathBuf::from(current_input);
        let (dir, file_prefix) = if current_input.ends_with('/') {
            (base_path, String::new())
        } else {
            (
                base_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
                base_path
                    .file_name()
                    .map_or(String::new(), |name| name.to_string_lossy().into_owned()),
            )
        };

        // Get directory entries
        let mut completions = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let entry_path = entry.path();
                    if let Some(file_name) = entry_path.file_name().and_then(|name| name.to_str()) {
                        // Create completion info
                        let completion_text = if entry_path.is_dir() {
                            format!("{}/", file_name)
                        } else {
                            file_name.to_string()
                        };

                        let display_text = if dir == Path::new(".") {
                            completion_text.clone()
                        } else {
                            format!("{}/{}", dir.display(), completion_text)
                        };

                        // Add if it matches using fuzzy search
                        if let Some(_) = fuzzy_match(&file_name, &file_prefix) {
                            completions.push(Completion {
                                display: display_text,
                                replacement: completion_text,
                            });
                        }
                    }
                }
            }
        }

        // Sort completions by score
        completions.sort_by(|a, b| {
            let score_a = fuzzy_match(&a.replacement, &file_prefix).unwrap_or(0);
            let score_b = fuzzy_match(&b.replacement, &file_prefix).unwrap_or(0);
            score_b.cmp(&score_a)
        });

        return Ok((param_start_pos, completions));
    }

    Ok((pos, vec![]))
}

// Autocomplete handler for memory IDs
pub fn autocomplete_memory_id(
    line: &str,
    pos: usize,
) -> Result<(usize, Vec<Completion>), ReadlineError> {
    if let Some((_, param_start_pos, param_text)) = extract_parameter_info(&line[..pos]) {
        let typed_prefix =
            param_text.trim_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace());

        // Get memory IDs
        let memory = get_memory().lock().unwrap();
        let ids: Vec<String> = memory.keys().cloned().collect();

        // Match IDs using fuzzy search
        let mut completions = Vec::new();
        for id in ids {
            if let Some(score) = fuzzy_match(&id, typed_prefix) {
                completions.push((
                    score,
                    Completion {
                        display: id.clone(),
                        replacement: id,
                    },
                ));
            }
        }

        // Sort by score
        completions.sort_by(|a, b| b.0.cmp(&a.0));

        // Extract just the completions without scores
        let result = completions.into_iter().map(|(_, c)| c).collect();

        return Ok((param_start_pos, result));
    }

    Ok((pos, vec![]))
}

// Autocomplete handler for model IDs
pub fn autocomplete_model_id(
    line: &str,
    pos: usize,
) -> Result<(usize, Vec<Completion>), ReadlineError> {
    if let Some((_, param_start_pos, param_text)) = extract_parameter_info(&line[..pos]) {
        let typed_prefix =
            param_text.trim_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace());

        // Get models from OpenRouter
        let models = match LLMClient::new() {
            Ok(client) => {
                match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt.block_on(async {
                        match client.list_models().await {
                            Ok(models) => models,
                            Err(_) => Vec::new(),
                        }
                    }),
                    Err(_) => Vec::new(),
                }
            },
            Err(_) => Vec::new(),
        };

        // Match models using fuzzy search
        let mut completions = Vec::new();
        for model in &models {
            // Try to match against ID first (higher priority)
            if let Some(score) = fuzzy_match(&model.id, typed_prefix) {
                completions.push((
                    score * 2,
                    Completion {
                        // Double score for ID matches
                        display: format!("{} ({})", model.id, model.name),
                        replacement: model.id.clone(),
                    },
                ));
                continue;
            }

            // Also try to match against name
            if let Some(score) = fuzzy_match(&model.name, typed_prefix) {
                completions.push((
                    score,
                    Completion {
                        display: format!("{} ({})", model.id, model.name),
                        replacement: model.id.clone(),
                    },
                ));
            }
        }

        // Sort by score
        completions.sort_by(|a, b| b.0.cmp(&a.0));

        // Extract just the completions without scores
        let result = completions.into_iter().map(|(_, c)| c).collect();

        return Ok((param_start_pos, result));
    }

    Ok((pos, vec![]))
}

pub fn save_history() {
    let mut editor_guard = RL_EDITOR.lock().unwrap(); // unwrap() panics if Mutex is poisoned

    editor_guard
        .save_history("history.txt")
        .unwrap_or_else(|err| {
            eprintln!("Failed to save history: {}", err);
        });
}
