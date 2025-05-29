use std::io;
use std::time::Duration;
use ratatui::crossterm::event::{Event, KeyEvent};
use ratatui::{DefaultTerminal, Frame};

pub enum PopupState {
    Continue,
    Exit,
}
pub(crate) trait Popup<T> {

    fn handle_key_event( &mut self, key: KeyEvent ) -> PopupState;
    fn render_popup(&mut self, f: &mut Frame);
    fn get_result(&self) -> T;
}

pub struct PopupManager<T> {
    current_popup: Option<Box<dyn Popup<T> + Send + Sync>>,
    terminal: DefaultTerminal
}

impl<T> PopupManager<T> {
    pub fn new() -> Self {
        PopupManager {
            current_popup: None,
            terminal:  ratatui::init()
        }
    }
    pub fn get_result(&self) -> Option<T> {
        self.current_popup.as_ref().map(|popup| popup.get_result())
    }
    pub fn show(&mut self, popup: Box<dyn Popup<T> + Send + Sync>) -> Result<(), io::Error> {
        self.current_popup = Some(popup);

        if let Some(ref mut popup) = self.current_popup {
            loop {
                self.terminal.draw(|frame| {
                    popup.render_popup(frame);
                })?;

                if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
                    match ratatui::crossterm::event::read()? {
                        Event::Key(key) => {
                            match popup.handle_key_event(key) {
                                PopupState::Continue => {}
                                PopupState::Exit => {
                                    break;
                                }
                            }
                        },
                        _ => {}
                    };
                }
            }
        }
        Ok(())
    }
}

