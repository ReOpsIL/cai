use tokio;
mod chat;
mod command_handler;
mod commands;
mod commands_registry;
mod configuration;
mod files;
mod history;
mod openrouter;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    chat::chat_loop().await?;
    Ok(())
}
