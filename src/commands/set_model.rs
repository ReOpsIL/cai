use crate::commands_registry::{Command, register_command};
use crate::configuration;
use crate::openrouter;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    id: String,
    name: String,
}

pub fn register_set_model_command() {
    register_command(Command {
        name: "set-model".to_string(),
        pattern: Regex::new(r"@set-model\(\s*(?:(.+))?\s*\)").unwrap(),
        description: "Set the model to use for chat".to_string(),
        usage_example: "@set-model([optional_filter])".to_string(),
        handler: |params| {
            // We can't use async code directly in the handler, so return instructions
            println!("Please use the async set-model command for now");
            println!("Command will be fully integrated in a future update");

            if let Some(filter) = params.get(0) {
                println!("Filter provided: {}", filter);
            }

            Ok(None)
        },
    });
}

pub async fn handle_set_model(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    // Extract filter from parentheses format
    let filter_match = Regex::new(r"@set-model\(\s*(?:(.+))?\s*\)")
        .unwrap()
        .captures(command);
    let model_filter = filter_match
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim());
    let models = openrouter::list_openrouter_models(api_key.as_str()).await?;

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
