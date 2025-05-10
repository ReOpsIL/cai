use std::io::{self, Write};
use crate::configuration;
use crate::openrouter;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    id: String,
    name: String,
}

pub async fn handle_set_model(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    let parts: Vec<&str> = command.splitn(2, ' ').collect();
    let model_filter = parts.get(1).map(|s| s.trim());
    let models = openrouter::list_openrouter_models(api_key.as_str()).await?;

    let filtered_models: Vec<_> = models
        .iter()
        .filter(|model| {
            model_filter.is_none() || model.id.contains(model_filter.unwrap())
        })
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