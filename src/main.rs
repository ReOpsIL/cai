use tokio;
mod chat;
mod command_handler;
mod commands;
mod commands_registry;
mod configuration;
mod files;
mod input_handler;
mod openrouter;
mod terminal;
mod autocomplete;
mod chat_ui;
mod commands_selector;
mod files_selector;
mod workflow;
mod workflow_test;

use chat_ui::main_ui;
//use editor::run_editor;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize models at startup
    println!("Initializing models...");
    if let Err(e) = commands::set_model::initialize_models().await {
        println!("Warning: Failed to initialize models: {}", e);
        println!("Some commands may not work correctly");
    }

    main_ui();

    //chat::chat_loop().await?;
    Ok(())
}
