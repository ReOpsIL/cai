use regex::Regex;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::highlight::CmdKind;
use std::sync::OnceLock;
use termion::color::{Fg, Reset};
use crate::terminal;

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
