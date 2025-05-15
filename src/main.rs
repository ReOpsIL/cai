use tokio;
mod chat;
mod command_handler;
mod commands;
mod commands_registry;
mod configuration;
mod files;
mod openrouter;
mod terminal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize models at startup
    println!("Initializing models...");
    if let Err(e) = commands::set_model::initialize_models().await {
        println!("Warning: Failed to initialize models: {}", e);
        println!("Some commands may not work correctly");
    }

    chat::chat_loop().await?;
    Ok(())
}
