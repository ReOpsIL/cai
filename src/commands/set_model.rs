use crate::configuration;
use crate::openrouter;
use lazy_static::lazy_static;
use regex::Regex;
use std::io::{self, Write};
use std::sync::Mutex;

// Use OpenRouter's Model directly
use crate::openrouter::Model;

lazy_static! {
    static ref MODELS: Mutex<Vec<Model>> = Mutex::new(Vec::new());
}

pub async fn initialize_models() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");
    
    let models = openrouter::list_openrouter_models(&api_key).await?;
    
    let mut models_store = MODELS.lock().unwrap();
    *models_store = models;
    
    println!("Models initialized: {} models available", models_store.len());
    Ok(())
}

pub fn handle_set_model(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Extract filter from parentheses format
    let filter_match = Regex::new(r"@set-model\(\s*(?:(.+))?\s*\)")
        .unwrap()
        .captures(command);
    let model_filter = filter_match
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim());

    // Get models from the initialized static store
    let models_guard = MODELS.lock().unwrap();
    let models = models_guard.clone();
    drop(models_guard); // Release lock early

    let filtered_models: Vec<_> = models
        .iter()
        .filter(|model| model_filter.is_none() || model.id.contains(model_filter.unwrap()))
        .collect();

    if filtered_models.is_empty() {
        println!("No models found matching filter: {:?}", model_filter);
    } else {
        println!("Available models:");
        for (i, model) in filtered_models.iter().enumerate() {
            println!("{}: {}", i + 1, model.name);
        }

        println!("Enter the number of the model to select (or press Enter to cancel):");
        io::stdout().flush()?;

        let mut model_index_input = String::new();
        io::stdin().read_line(&mut model_index_input)?;

        if let Ok(model_index) = model_index_input.trim().parse::<usize>() {
            if model_index > 0 && model_index <= filtered_models.len() {
                let selected_model = &filtered_models[model_index - 1];
                println!("Selected model: {}", selected_model.name);

                // Update config
                let mut config = configuration::load_configuration()?;
                config.model = selected_model.id.clone();
                configuration::save_configuration(&config)?;

                println!("Model saved to config file.");
            } else {
                println!("Invalid model number.");
            }
        } else {
            println!("No model selected.");
        }
    }

    Ok(())
}
