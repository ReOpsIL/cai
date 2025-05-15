use regex::Regex;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::highlight::CmdKind;
use std::sync::OnceLock;
use termion::color::{Fg, Reset};
use crate::terminal;
use std::path::PathBuf; // Add this for path manipulation
use std::fs; // Add this for filesystem operations

static COMMAND_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_command_regex() -> &'static Regex {
    COMMAND_REGEX.get_or_init(|| {
        // This regex matches all commands found in mod.rs
        // It handles both @ and ! command prefixes and properly captures the parameters
        Regex::new(r"([@!][a-zA-Z\-]*)") //\(\s*(?:\S+\s*(?:,\s*\S+\s*)*)??\)
            .expect("Failed to compile command regex")
    })
}

fn colorize_commands(input: &str) -> String {
    let regex = get_command_regex();
    let mut result = input.to_string();
    let mut offset = 0;

    for cap in regex.find_iter(input) {
        let start = cap.start() + offset;
        let end = cap.end() + offset;

        // Insert termion color codes
        let reset = format!("{}", Fg(Reset));
        let cyan = format!("{}", Fg(terminal::CYAN));
        
        result.insert_str(end, &reset);
        result.insert_str(start, &cyan);

        // Update offset for subsequent replacements
        offset += cyan.len() + reset.len();
    }

    result
}

// Custom implementation of a colored prompt
#[derive(Default)]
struct ColoredPrompt {}

impl rustyline::Helper for ColoredPrompt {}
impl rustyline::highlight::Highlighter for ColoredPrompt {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        std::borrow::Cow::Owned(colorize_commands(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _cmd_kind: CmdKind) -> bool {
        true
    }
}
impl rustyline::completion::Completer for ColoredPrompt {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Result<(usize, Vec<Self::Candidate>), rustyline::error::ReadlineError> {
        let text_before_cursor = &line[..pos];

        // Find the last opening parenthesis before the cursor
        if let Some(open_paren_idx) = text_before_cursor.rfind('(') {
            // Basic check if the text before the parenthesis looks like a command call prefix
            // A more robust implementation might check against a list of known commands
             let potential_command_prefix = &text_before_cursor[..open_paren_idx];
             if potential_command_prefix.contains('@') || potential_command_prefix.contains('!') {

                // Extract the text within the potential command argument
                let partial_path_segment = &text_before_cursor[open_paren_idx + 1..pos];

                // Determine the directory to list and the prefix to match
                let (base_dir, file_prefix) = if let Some(last_slash_idx) = partial_path_segment.rfind('/') {
                    // Path includes a directory part
                    let dir_part = &partial_path_segment[..=last_slash_idx];
                    let prefix = &partial_path_segment[last_slash_idx + 1..];
                    (PathBuf::from(dir_part), prefix)
                } else {
                    // Only a file prefix, list current directory
                    (PathBuf::from("."), partial_path_segment)
                };

                let mut completions = Vec::new();
                // Attempt to list directory contents
                // Use `read_dir` which returns an iterator over results
                if let Ok(entries) = fs::read_dir(&base_dir) {
                    for entry in entries.flatten() { // Flatten to skip entries that failed to read
                        if let Ok(file_name) = entry.file_name().into_string() {
                            // Check if the file/directory name starts with the prefix
                            if file_name.starts_with(file_prefix) {
                                let mut candidate = file_name;
                                // Add a slash for directories to make it easier to navigate
                                if entry.path().is_dir() {
                                    candidate.push('/');
                                }
                                completions.push(candidate);
                            }
                        }
                    }
                }

                // Calculate the start position for replacement
                // This is the index in the original line where the file_prefix begins.
                let replacement_start = open_paren_idx + 1 + partial_path_segment.rfind('/').map_or(0, |i| i + 1);

                return Ok((replacement_start, completions));
            }
        }

        // Default behavior: no completions
        Ok((pos, Vec::new()))
    }
}
impl rustyline::hint::Hinter for ColoredPrompt {
    type Hint = String;
}
impl rustyline::validate::Validator for ColoredPrompt {}

pub async fn get_input() -> Result<String, Box<dyn std::error::Error>> {
    let mut rl = rustyline::Editor::<ColoredPrompt, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(ColoredPrompt::default()));
    rl.set_auto_add_history(true);

    match rl.readline("> ") {
        Ok(line) => Ok(line),
        Err(ReadlineError::Interrupted) => {
            // Ctrl-C
            Err("Input interrupted".to_string().into())
        }
        Err(ReadlineError::Eof) => {
            // Ctrl-D
            Err("Input terminated".to_string().into())
        }
        Err(err) => Err(Box::new(err)),
    }
}
