use std::fs;
use std::path::Path;
use chrono::Local;

pub async fn save_prompt_history(prompt_history: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "prompt_history.txt";
    let path = Path::new(file_path);

    let now = Local::now();
    let time_stamp = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut file_content = String::new();

    file_content.push_str(&format!("@--- [{}] ---\n\n", time_stamp));

    for prompt in prompt_history {
        file_content.push_str(&format!("{}\n", prompt));
    }

    fs::write(path, file_content)?;
    println!("Prompt history saved to prompt_history.txt");

    Ok(())
}