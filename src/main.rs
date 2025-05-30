use tokio;

// New modular structure
mod app;
mod core;
mod services;
mod ui;
mod utils;

// Legacy modules (keeping for compatibility during transition)
mod commands;
mod commands_registry;
mod autocomplete;
mod commands_selector;
mod files_selector;
mod message_popup;
mod popup_util;
mod syntax_highlighting;

// Re-exports for backward compatibility
mod chat {
    pub use crate::core::memory::*;
    pub use crate::core::command_processor::*;
}
mod configuration {
    pub use crate::app::config::*;
}
mod files {
    pub use crate::services::file_service::*;
}
mod terminal {
    pub use crate::utils::terminal::*;
}
mod tree {
    pub use crate::services::project_service::*;
}
mod openrouter {
    pub use crate::services::llm_client::*;
}
mod popup_manager {
    pub use crate::ui::popups::popup_manager::*;
}
mod yes_no_popup {
    pub use crate::ui::popups::yes_no::*;
}

use crate::ui::app::ChatUIApp;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    commands::register_all_commands();
    
    println!(
        "{}",
        utils::terminal::format_info("Starting CAI application...")
    );

    let terminal = ratatui::init();
    let app_result = ChatUIApp::new()?.run(terminal);
    ratatui::restore();
    
    match app_result {
        Ok(()) => println!("{}", utils::terminal::format_info("Application closed successfully")),
        Err(e) => println!("{}", utils::terminal::format_error(&format!("Application error: {}", e))),
    }
    
    Ok(())
}
