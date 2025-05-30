use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::app::state::{AppState, FocusedInputArea};

pub enum AppEvent {
    Exit,
    Continue,
    ExecuteLLM,
    SaveFile,
    OpenFile,
    CreateFile,
    CreateDirectory,
    DeleteFile,
    DeleteDirectory,
    RenameItem,
    ShowCommandsPopup,
    ShowFilesPopup,
    FocusNext,
    ToggleEditMode,
}

pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_key_event(&self, key: KeyEvent, state: &AppState) -> Option<AppEvent> {
        match key.code {
            KeyCode::Tab => Some(AppEvent::FocusNext),
            KeyCode::Esc => {
                if state.show_commands_popup || state.show_files_popup {
                    Some(AppEvent::Continue) // Let popup handle it
                } else if state.should_exit() {
                    Some(AppEvent::Exit)
                } else {
                    Some(AppEvent::SaveFile)
                }
            }
            KeyCode::F(1) => {
                if key.kind == KeyEventKind::Press {
                    Some(AppEvent::ExecuteLLM)
                } else {
                    Some(AppEvent::Continue)
                }
            }
            KeyCode::Char(':') => {
                if !state.show_commands_popup && !state.show_files_popup 
                   && state.current_focus_area != FocusedInputArea::Question {
                    Some(AppEvent::ShowCommandsPopup)
                } else {
                    Some(AppEvent::Continue)
                }
            }
            KeyCode::Char('$') => {
                if !state.show_commands_popup && !state.show_files_popup 
                   && state.current_focus_area != FocusedInputArea::Question {
                    Some(AppEvent::ShowFilesPopup)
                } else {
                    Some(AppEvent::Continue)
                }
            }
            // Project tree specific events
            KeyCode::Char('n') if state.current_focus_area == FocusedInputArea::ProjectTree => {
                Some(AppEvent::CreateFile)
            }
            KeyCode::Char('N') if state.current_focus_area == FocusedInputArea::ProjectTree => {
                Some(AppEvent::CreateDirectory)
            }
            KeyCode::Char('d') if state.current_focus_area == FocusedInputArea::ProjectTree => {
                Some(AppEvent::DeleteFile)
            }
            KeyCode::Char('D') if state.current_focus_area == FocusedInputArea::ProjectTree => {
                Some(AppEvent::DeleteDirectory)
            }
            KeyCode::Char('r') if state.current_focus_area == FocusedInputArea::ProjectTree => {
                Some(AppEvent::RenameItem)
            }
            KeyCode::Enter if state.current_focus_area == FocusedInputArea::ProjectTree => {
                Some(AppEvent::OpenFile)
            }
            _ => Some(AppEvent::Continue),
        }
    }
}