use tokio;

mod history;
mod configuration;
mod command_handler;
mod commands {
    pub mod set_model;
    pub mod help;
}
mod chat;
mod openrouter;
mod files;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    chat::chat_loop().await?;
    Ok(())
}
