use std::collections::HashMap;
use std::io;
use std::time::Duration;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::{Borders, Clear, List, ListDirection, ListItem, Paragraph, Wrap};
use ratatui::{Frame, layout::{Constraint, Flex, Layout, Rect, Direction}, widgets::{Block}, DefaultTerminal};

use ratatui::{
    prelude::*,
    style::{
        palette::tailwind::{SKY}, Style, Stylize,
    },
};
use ratatui::layout::Alignment;
use crate::popup_manager::{Popup, PopupManager, PopupState};
use crate::popup_util::centered_rect;
use crate::yes_no_popup::YesNoPopup;

#[derive(Debug, Clone, PartialEq)]
pub enum MessageState {
    Ok,
    Exit,
    NotSelected,
}

pub struct MessagePopup {
    current_state : MessageState,
    title: String,
}


impl MessagePopup {
    pub fn new(title: String) -> Self {
        Self {
            title,
            current_state: MessageState::NotSelected,
        }
    }

    fn select_next(&mut self) {
        match self.current_state {
            MessageState::Ok => {
                self.current_state = MessageState::NotSelected;
            },
            MessageState::NotSelected => {
                self.current_state = MessageState::Ok;
            },
            _ => {}
        }
    }

}

impl Popup<MessageState> for MessagePopup {

    fn handle_key_event(&mut self, key: KeyEvent) -> PopupState {
        match key.code {
            KeyCode::Right | KeyCode::Left => {
                self.select_next();
                PopupState::Continue
            },
            KeyCode::Esc => {
                self.current_state = MessageState::Exit;
                PopupState::Exit
            }
            KeyCode::Enter => {
                PopupState::Exit
            }
            _ => {
                PopupState::Continue
            }
        }
    }
    fn render_popup(&mut self, f: &mut Frame) {
        let size = f.size();

        let ok_text = " Ok ";

        // Calculate popup area (e.g., 60% width, 25% height, centered)
        let popup_area = centered_rect(60, 25, size);
        f.render_widget(Clear, popup_area); // Clear the area under the popup

        let popup_block = Block::default()
            .title(self.title.clone())
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


        let question_paragraph = Paragraph::new(self.title.clone())
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        f.render_widget(question_paragraph, message_area);


        // Layout for buttons (horizontal)
        // Give a bit of spacing around buttons
        let button_chunks = Layout::horizontal([
            Constraint::Percentage(20), // Spacer
            Constraint::Length(ok_text.len() as u16 + 2), // Yes button + padding
            Constraint::Percentage(20), // Spacer
        ])
        .flex(Flex::Center)
        .split(buttons_area);


        let ok_style = if self.current_state == MessageState::Ok {
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };


        let ok_button = Paragraph::new(ok_text)
            .style(ok_style)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(ok_button, button_chunks[1]);

    }

    fn get_result(&self) -> MessageState {
        self.current_state.clone()
    }
}


pub fn create_message_popup(title: String)  {
    let mut manager = PopupManager::new();
    let message_popup = Box::new(MessagePopup::new(title));
    manager.show(message_popup).expect("TODO: panic message");
}
