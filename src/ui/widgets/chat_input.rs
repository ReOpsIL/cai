use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders},
    Frame,
};
use ratatui::crossterm::event::{KeyEvent, KeyCode};
use tui_textarea::{CursorMove, TextArea};
use crate::app::state::FocusedInputArea;
use crate::services::file_service::FileService;
use crate::services::ollama_client::OllamaClient;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

pub struct ChatInputWidget<'a> {
    text_area: TextArea<'a>,
    file_service: FileService,
    last_keystroke: Option<Instant>,
    typing_pause_duration: Duration,
    last_text: String,
    runtime: Runtime,
}

impl<'a> ChatInputWidget<'a> {
    pub fn new() -> Self {
        let mut text_area = TextArea::default();
        let style = Style::default();
        text_area.set_line_number_style(style);

        Self {
            text_area,
            file_service: FileService::new(),
            last_keystroke: None,
            typing_pause_duration: Duration::from_secs(2), // 2 seconds pause to trigger autocomplete
            last_text: String::new(),
            runtime: Runtime::new().expect("Failed to create Tokio runtime"),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        // Check for autocomplete if the widget is focused
        if focused {
            self.check_and_apply_autocomplete();
        }

        let border_color = if focused {
            ratatui::style::Color::Blue
        } else {
            ratatui::style::Color::LightYellow
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title("YOU:");

        self.text_area.set_block(block);
        frame.render_widget(&self.text_area, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, edit_mode: bool) -> bool {
        if edit_mode {
            // Record the time of this keystroke
            self.last_keystroke = Some(Instant::now());

            // Store the current text before processing the key event
            let current_text = self.get_content();

            // Process the key event
            self.text_area.input(key);

            // Update last_text if the content has changed
            let new_text = self.get_content();
            if current_text != new_text {
                self.last_text = current_text;
            }

            true
        } else {
            false
        }
    }

    // Check if the user has paused typing long enough to trigger autocomplete
    pub fn should_autocomplete(&self) -> bool {
        if let Some(last_keystroke) = self.last_keystroke {
            let elapsed = last_keystroke.elapsed();

            // Only autocomplete if:
            // 1. The user has paused typing for the specified duration
            // 2. The text area is not empty
            // 3. The text has changed since the last autocomplete
            elapsed >= self.typing_pause_duration && 
            !self.get_content().is_empty() && 
            self.get_content() != self.last_text
        } else {
            false
        }
    }

    // Get a completion from Ollama and apply it
    pub fn check_and_apply_autocomplete(&mut self) {
        if self.should_autocomplete() {
            let current_text = self.get_content();

            // Clone the text for use in the async block
            let text_to_complete = current_text.clone();

            // Use the runtime to run the async Ollama client
            let completion_result = self.runtime.block_on(async {
                match OllamaClient::get_global_client().await {
                    Ok(client_mutex) => {
                        if let Some(client) = &*client_mutex.lock().await {
                            match client.get_completion(&text_to_complete).await {
                                Ok(completion) => {
                                    // Extract only the new part of the completion
                                    if let Some(new_text) = completion.strip_prefix(&text_to_complete) {
                                        return Some(new_text.to_string());
                                    }
                                    None
                                },
                                Err(e) => {
                                    eprintln!("Error getting completion: {}", e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    },
                    Err(e) => {
                        eprintln!("Error getting Ollama client: {}", e);
                        None
                    }
                }
            });

            // Apply the completion if we got one
            if let Some(completion_text) = completion_result {
                // Insert the completion
                self.text_area.insert_str(&completion_text);

                // Update last_text to prevent repeated autocompletions
                self.last_text = self.get_content();
            }

            // Reset the last keystroke time to prevent repeated autocompletions
            self.last_keystroke = None;
        }
    }

    pub fn get_content(&self) -> String {
        self.text_area.lines().join("\n")
    }

    pub fn set_content(&mut self, content: &str) {
        self.text_area.select_all();
        self.text_area.insert_str(content);
        self.text_area.move_cursor(CursorMove::Jump(0, 0));
    }

    pub fn clear(&mut self) {
        self.text_area.select_all();
        self.text_area.delete_line_by_head();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.text_area.insert_str(text);
    }

    pub fn open_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = self.file_service.read_file(file_path)?;
        self.set_content(&content);
        Ok(())
    }

    pub fn save_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = self.get_content();
        self.file_service.write_file(file_path, &content)?;
        Ok(())
    }

    pub fn set_title(&mut self, title: &str) {
        let current_block = self.text_area.block().cloned().unwrap_or_default();
        let new_block = current_block.title(title.to_string());
        self.text_area.set_block(new_block);
    }
}
