use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::{Clear, List, ListDirection, ListItem, Paragraph, Wrap};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect, Direction},
    widgets::{Block},
};

use ratatui::{
    style::{
        palette::tailwind::{SKY}, Style, Stylize,
    },

};


use crate::commands_registry;
use crate::commands_registry::Command;


#[derive(Debug, Clone, PartialEq)]
pub enum CommandSelectorState {
    Selected,
    NotSelected,
    Exit,
}

pub struct CommandSelector {
    current_index: usize
}

impl CommandSelector {

    pub fn new() -> Self {
        let commands = commands_registry::get_all_commands();
        Self {
            current_index: 0
        }
    }

    fn select_next(&mut self) {
        let commands = commands_registry::get_all_commands();
        if self.current_index <  commands.len()-1 { self.current_index+=1 };
    }

    fn select_previous(&mut self) {
        if self.current_index > 0 { self.current_index-=1 };
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> (Option<Command>, CommandSelectorState) {
        let commands = commands_registry::get_all_commands();

        if key.kind != KeyEventKind::Press {
            return (None, CommandSelectorState::NotSelected);
        }
        match key.code {
            KeyCode::Down => {
                self.select_next();
            },
            KeyCode::Up => {
                self.select_previous();
            },
            KeyCode::Esc => {
                return (None, CommandSelectorState::Exit);
            }
            KeyCode::Right | KeyCode::Enter => {
                return (Some(commands[self.current_index].clone()), CommandSelectorState::Selected);
            }
            _ => {
                return (None, CommandSelectorState::NotSelected);
            }
        }
        return (None, CommandSelectorState::NotSelected);
    }

    pub fn render_commands_popup(&self, frame: &mut Frame) {
        let commands = commands_registry::get_all_commands();

        let items: Vec<ListItem> = commands
            .iter()
            .enumerate()
            .map(|(i, command)| {
                if i == self.current_index {
                    ListItem::from(command.name.clone()).bg(SKY.c900)
                }
                else {
                    ListItem::from(command.name.clone())
                }
            }).collect();

        // Get the overall popup area
        let popup_area = self.popup_area(frame.area(), 80, 60);

        // Split the popup area into two parts: list on the left, details on the right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60),  // 40% for the list
                Constraint::Percentage(40),  // 60% for the details
            ])
            .split(popup_area);

        let list_area = chunks[0];
        let details_area = chunks[1];

        let list = List::new(items)
            .block(Block::bordered().title("Commands (Press 'Esc' to close)"))
            .style(Style::new().white())
            .highlight_style(Style::new().italic())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);

        frame.render_widget(Clear, popup_area);
        frame.render_widget(list, list_area);
        self.render_command_details(frame, details_area);
    }

    fn popup_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn render_command_details(&self, frame: &mut Frame, area: Rect) {
        let commands = commands_registry::get_all_commands();
        if self.current_index < commands.len() {
            let command = &commands[self.current_index];

            let details = format!(
                "Name: {}\n\nDescription: {}\n\nUsage Example: {}\n\nSection: {}\n\nType: {:?}",
                command.name,
                command.description,
                command.usage_example,
                command.section,
                command.command_type
            );

            let details_widget = Paragraph::new(details)
                .block(Block::bordered().title("Command Details"))
                .style(Style::new().white())
                .wrap(Wrap { trim: true });

            frame.render_widget(details_widget, area);
        }
    }
}
