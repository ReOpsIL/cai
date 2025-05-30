use std::collections::HashMap;
use tui_tree_widget::{TreeItem, TreeState};
use crate::core::memory::Prompt;

#[derive(PartialEq)]
pub enum FocusedInputArea {
    Question,
    Answer,
    ProjectTree,
}

pub struct AppState {
    pub current_focus_area: FocusedInputArea,
    pub question_prompt: Prompt,
    pub answer_prompt: Prompt,
    pub edit_mode: bool,
    pub escape_count: u8,
    pub last_file_path: String,
    
    // Project tree state
    pub tree_state: TreeState<String>,
    pub project_tree_items: Vec<TreeItem<'static, String>>,
    pub project_tree_ids_map: HashMap<String, u32>,
    pub project_tree_do_refresh: bool,
    
    // Popup states
    pub show_commands_popup: bool,
    pub show_files_popup: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_focus_area: FocusedInputArea::ProjectTree,
            question_prompt: Prompt::default(),
            answer_prompt: Prompt::default(),
            edit_mode: false,
            escape_count: 0,
            last_file_path: String::new(),
            
            tree_state: TreeState::default(),
            project_tree_items: Vec::new(),
            project_tree_ids_map: HashMap::new(),
            project_tree_do_refresh: true,
            
            show_commands_popup: false,
            show_files_popup: false,
        }
    }

    pub fn reset_escape_count(&mut self) {
        self.escape_count = 0;
    }

    pub fn increment_escape_count(&mut self) {
        self.escape_count += 1;
    }

    pub fn should_exit(&self) -> bool {
        self.escape_count >= 5
    }

    pub fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
    }

    pub fn set_focus(&mut self, area: FocusedInputArea) {
        self.current_focus_area = area;
    }

    pub fn cycle_focus(&mut self) {
        self.current_focus_area = match self.current_focus_area {
            FocusedInputArea::Question => FocusedInputArea::Answer,
            FocusedInputArea::Answer => FocusedInputArea::ProjectTree,
            FocusedInputArea::ProjectTree => FocusedInputArea::Question,
        };
    }

    pub fn refresh_project_tree(&mut self) {
        self.project_tree_do_refresh = true;
    }

    pub fn show_commands_popup(&mut self) {
        self.show_commands_popup = true;
    }

    pub fn hide_commands_popup(&mut self) {
        self.show_commands_popup = false;
    }

    pub fn show_files_popup(&mut self) {
        self.show_files_popup = true;
    }

    pub fn hide_files_popup(&mut self) {
        self.show_files_popup = false;
    }
}