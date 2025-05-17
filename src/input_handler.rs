use crate::{autocomplete};
use rustyline::error::ReadlineError;


pub async fn get_input() -> Result<String, Box<dyn std::error::Error>> {
    let mut editor_guard = autocomplete::RL_EDITOR.lock().unwrap(); // unwrap() panics if Mutex is poisoned

    match editor_guard.readline("You: ") {
        Ok(line) => Ok(line),
        Err(ReadlineError::Interrupted) => Err("Input interrupted".to_string().into()),
        Err(ReadlineError::Eof) => Err("Input terminated".to_string().into()),
        Err(err) => Err(Box::new(err)),
    }
}

