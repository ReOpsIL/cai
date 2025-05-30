use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders},
    Frame,
};
use ratatui::crossterm::event::KeyEvent;
use tui_textarea::{TextArea};

pub struct ChatOutputWidget<'a> {
    text_area: TextArea<'a>,
}

impl<'a> ChatOutputWidget<'a> {
    pub fn new() -> Self {
        Self {
            text_area: TextArea::default(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_color = if focused {
            ratatui::style::Color::Blue
        } else {
            ratatui::style::Color::LightYellow
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title("LLM:");

        self.text_area.set_block(block);
        frame.render_widget(&self.text_area, area);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        use ratatui::crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                self.text_area.input(key);
                true
            }
            _ => false,
        }
    }

    pub fn set_content(&mut self, content: &str, area_width: u16) {
        let wrapped_content = textwrap::wrap(content, area_width as usize).join("\n");
        self.text_area.select_all();
        self.text_area.insert_str(&wrapped_content);
    }

    pub fn get_content(&self) -> String {
        self.text_area.lines().join("\n")
    }

    pub fn clear(&mut self) {
        self.text_area.select_all();
        self.text_area.delete_line_by_head();
    }

    pub fn set_title(&mut self, title: &str) {
        let current_block = self.text_area.block().cloned().unwrap_or_default();
        let new_block = current_block.title(title.to_string());
        self.text_area.set_block(new_block);
    }

    pub fn append_content(&mut self, content: &str, area_width: u16) {
        let wrapped_content = textwrap::wrap(content, area_width as usize).join("\n");
        self.text_area.insert_str(&wrapped_content);
    }
}