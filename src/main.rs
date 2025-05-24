use tokio;
mod chat;
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
mod tree;

use chat_ui::main_ui;
//use editor::run_editor;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    main_ui();

    //chat::chat_loop().await?;
    Ok(())
}
