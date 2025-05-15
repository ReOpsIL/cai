use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use rustyline::config::Configurer;
use rustyline::highlight::CmdKind;
use std::sync::Mutex;
use std::sync::OnceLock;
use regex::Regex;

// No longer using a static editor since we create a new one for each prompt
// This allows us to use the highlighting feature
static COMMAND_REGEX: OnceLock<Regex> = OnceLock::new();

// Function removed as we now create the editor inline with highlighting support

fn get_command_regex() -> &'static Regex {
    COMMAND_REGEX.get_or_init(|| {
        Regex::new(r"(@\w+(?:\([^)]*\))?)").expect("Failed to compile command regex")
    })
}

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
    
    match rl.readline("You: ") {
        Ok(line) => Ok(line),
        Err(ReadlineError::Interrupted) => {
            // Ctrl-C
            Err("Input interrupted".to_string().into())
        },
        Err(ReadlineError::Eof) => {
            // Ctrl-D
            Err("Input terminated".to_string().into())
        },
        Err(err) => Err(Box::new(err)),
    }
}