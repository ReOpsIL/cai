use std::io::{self, Write};

pub async fn get_input() -> Result<String, Box<dyn std::error::Error>> {
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}
