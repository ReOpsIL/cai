use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::{Borders, Clear, List, ListDirection, ListItem, Paragraph, Wrap};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect, Direction},
    widgets::{Block},
};

use ratatui::{
    prelude::*,
    style::{
        palette::tailwind::{SKY}, Style, Stylize,
    },

};
use ratatui::layout::Alignment;
use crate::files::files::{list_files, read_file};
#[derive(Debug, Clone, PartialEq)]
pub enum YesNoState {
    Yes,
    No,
    Exit,
    NotSelected,
}

pub struct YesNoPopup {
    current_state : YesNoState
}

impl YesNoPopup {

    pub fn new() -> Self {
        Self {
            current_state: YesNoState::No,
        }
    }

    fn select_next(&mut self) {
        match self.current_state {
            YesNoState::Yes => {
                self.current_state = YesNoState::No;
            },
            YesNoState::No => {
                self.current_state = YesNoState::Yes;
            },
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> YesNoState {

        if key.kind != KeyEventKind::Press {
            return YesNoState::NotSelected;
        }
        match key.code {
            KeyCode::Right | KeyCode::Left => {
                self.select_next();
            },
            KeyCode::Esc => {
                return YesNoState::Exit;
            }
            KeyCode::Enter => {
                return self.current_state.clone()
            }
            _ => {

            }
        }
        YesNoState::NotSelected
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
    
    pub fn render_yes_no_popup(&mut self, f: &mut Frame) {
        let size = f.size();

        let popup_title = "Confirmation";
        let popup_text = "Are you sure you want to proceed?";
        let yes_text = " Yes ";
        let no_text = " No ";

        // Calculate popup area (e.g., 60% width, 25% height, centered)
        let popup_area = self.centered_rect(60, 25, size);
        f.render_widget(Clear, popup_area); // Clear the area under the popup

        let popup_block = Block::default()
            .title(popup_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        // Layout for popup content (message + buttons)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0), // For message (flexible height)
                Constraint::Length(3), // For buttons (fixed height for border + text)
            ])
            .margin(1) // Margin inside the popup block
            .split(popup_area);

        let message_area = chunks[0];
        let buttons_area = chunks[1];

        // Render popup block (covers message and buttons area effectively)
        f.render_widget(popup_block, popup_area);


        let question_paragraph = Paragraph::new(popup_text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        f.render_widget(question_paragraph, message_area);


        // Layout for buttons (horizontal)
        // Give a bit of spacing around buttons
        let button_chunks = Layout::horizontal([
                Constraint::Percentage(20), // Spacer
                Constraint::Length(yes_text.len() as u16 + 2), // Yes button + padding
                Constraint::Percentage(10), // Spacer
                Constraint::Length(no_text.len() as u16 + 2),  // No button + padding
                Constraint::Percentage(20), // Spacer
            ])
            .flex(Flex::Center)
            .split(buttons_area);


        let yes_style = if self.current_state == YesNoState::Yes {
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let no_style = if self.current_state == YesNoState::No {
            Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let yes_button = Paragraph::new(yes_text)
            .style(yes_style)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        let no_button = Paragraph::new(no_text)
            .style(no_style)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(yes_button, button_chunks[1]);
        f.render_widget(no_button, button_chunks[3]);

    }

}
