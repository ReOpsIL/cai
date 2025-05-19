use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
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

use crate::files::files::{list_files, read_file};
#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorState {
    Selected,
    NotSelected,
    Exit,
}

pub struct FileSelector {
    list_source :  Vec<String>,
    current_index: usize
}

impl FileSelector {

    pub fn new() -> Self {
        Self {
            list_source: Vec::new(),
            current_index: 0
        }
    }

    fn select_next(&mut self) {
        if self.current_index <  self.list_source.len()-1 { self.current_index+=1 };
    }

    fn select_previous(&mut self) {
        if self.current_index > 0 { self.current_index-=1 };
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> (Option<String>, FileSelectorState) {

        if key.kind != KeyEventKind::Press {
            return (None, FileSelectorState::NotSelected);
        }
        match key.code {
            KeyCode::Down => {
                self.select_next();
            },
            KeyCode::Up => {
                self.select_previous();
            },
            KeyCode::Esc => {
                return (None, FileSelectorState::Exit);
            }
            KeyCode::Right | KeyCode::Enter => {
                return (Some(self.list_source[self.current_index].clone()), FileSelectorState::Selected);
            }
            _ => {
                return (None, FileSelectorState::NotSelected);
            }
        }
        return (None, FileSelectorState::NotSelected);
    }

    pub fn render_files_popup(&mut self, frame: &mut Frame) {

        self.list_source =  list_files(&"./**/*.md").expect("REASON");

        let items: Vec<ListItem> = self.list_source
            .iter()
            .enumerate()
            .map(|(i, value)| {
                if i == self.current_index {
                    ListItem::from(value.clone()).bg(SKY.c900)
                }
                else {
                    ListItem::from(value.clone())
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
            .block(Block::bordered().title("Files: ('Enter' to select, 'Esc' to close)"))
            .style(Style::new().white())
            .highlight_style(Style::new().italic())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);

        frame.render_widget(Clear, popup_area);
        frame.render_widget(list, list_area);
        self.render_file_snipest(frame, details_area);
    }

    fn popup_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn render_file_snipest(&self, frame: &mut Frame, area: Rect) {

        if self.current_index < self.list_source.len() {
            let value = &self.list_source[self.current_index];

            match read_file(value) {
                Ok(contents) => {
                    let details_widget = Paragraph::new(contents)
                        .block(Block::bordered().title("Content:"))
                        .style(Style::new().white())
                        .wrap(Wrap { trim: true });

                    frame.render_widget(details_widget, area);
                }
                Err(_) => {}
            }
        }
    }
}
