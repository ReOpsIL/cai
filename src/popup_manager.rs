use std::io;
use ratatui::{Frame};

pub enum PopupState {
    Continue,
    Exit,
}
pub(crate) trait Popup<T> {

    fn handle_key_event( &mut self, key: ratatui::crossterm::event::KeyEvent ) -> PopupState;
    fn render_popup(&mut self, f: &mut Frame);
    fn get_result(&self) -> &T;
}

pub struct PopupWrapper<T> {
    current_popup: Option<Box<dyn Popup<T> + Send + Sync>>,
}

impl<T> PopupWrapper<T> {
    pub fn new() -> Self {
        PopupWrapper {
            current_popup: None,
        }
    }
    pub fn is_visible(&self) -> bool {
        self.current_popup.is_some()
    }
    pub fn is_hidden(&self) -> bool {
        self.current_popup.is_none()
    }
    pub fn get_result(&self) -> Option<&T> {
        self.current_popup.as_ref().map(|popup| popup.get_result())
    }
    
    pub fn show(&mut self, popup: Box<dyn Popup<T> + Send + Sync>) -> Result<(), io::Error> {
        self.current_popup = Some(popup);
        Ok(())
    }
    
    pub fn hide(&mut self) -> Result<(), io::Error> {
        self.current_popup = None;
        Ok(())
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        if let Some(ref mut popup) = self.current_popup {
            popup.render_popup(frame);
        }
    }

    pub fn handle_key_event(&mut self, key: ratatui::crossterm::event::KeyEvent) -> PopupState {
        if let Some(ref mut popup) = self.current_popup {
            popup.handle_key_event(key)
        } else { 
            PopupState::Continue
        }
    }
}
