use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Scrollbar, ScrollbarOrientation},
    Frame,
};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use tui_tree_widget::{Tree, TreeItem, TreeState};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::services::{project_service::ProjectService, file_service::FileService};

pub struct ProjectTreeWidget<'a> {
    state: TreeState<String>,
    items: Vec<TreeItem<'a, String>>,
    ids_map: HashMap<String, u32>,
    project_service: ProjectService,
    file_service: FileService,
    needs_refresh: bool,
}

impl<'a> ProjectTreeWidget<'a> {
    pub fn new() -> Self {
        Self {
            state: TreeState::default(),
            items: Vec::new(),
            ids_map: HashMap::new(),
            project_service: ProjectService::new(),
            file_service: FileService::new(),
            needs_refresh: true,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        self.refresh_if_needed();

        let border_color = if focused {
            ratatui::style::Color::Blue
        } else {
            ratatui::style::Color::LightYellow
        };

        let tree = Tree::new(&self.items)
            .expect("all item identifiers are unique")
            .block(
                Block::bordered()
                    .title("Project Tree")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .experimental_scrollbar(Some(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
            ))
            .highlight_style(
                Style::new()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(tree, area, &mut self.state);
    }

    pub fn handle_event(&mut self, event: Event) -> Result<bool, std::io::Error> {
        let update_tree = match event {
            Event::Key(key) if !matches!(key.kind, KeyEventKind::Press) => false,
            Event::Key(key) => match key.code {
                KeyCode::Char('\n' | ' ') => self.state.toggle_selected(),
                KeyCode::Left => self.state.key_left(),
                KeyCode::Right => self.state.key_right(),
                KeyCode::Down => self.state.key_down(),
                KeyCode::Up => self.state.key_up(),
                KeyCode::Home => self.state.select_first(),
                KeyCode::End => self.state.select_last(),
                KeyCode::PageDown => self.state.scroll_down(3),
                KeyCode::PageUp => self.state.scroll_up(3),
                _ => false,
            },
            Event::Resize(_, _) => true,
            _ => false,
        };

        if update_tree {
            self.needs_refresh = true;
        }

        Ok(update_tree)
    }

    pub fn refresh_if_needed(&mut self) {
        if !self.needs_refresh {
            return;
        }

        match self.project_service.generate_md_tree(".") {
            Ok((tree_items, ids_map)) => {
                self.items = tree_items;
                self.ids_map = ids_map;
                self.needs_refresh = false;
            },
            Err(e) => {
                eprintln!("Error generating project tree: {}", e);
            }
        }
    }

    pub fn force_refresh(&mut self) {
        self.needs_refresh = true;
    }

    pub fn get_selected_path(&self, file_only: bool) -> Option<String> {
        let current = ".".to_string();
        let selected_tree_item_ids = self.state.selected();
        
        if let Some(leaf_id) = selected_tree_item_ids.last() {
            let leaf_id_str = leaf_id.to_string();
            let selected_path = self.ids_map
                .iter()
                .find(|item| item.1.to_string() == leaf_id_str)
                .map(|item| item.0.to_string());

            if let Some(path) = selected_path {
                if Path::new(&path).is_dir() {
                    if file_only { 
                        None 
                    } else { 
                        Some(path) 
                    }
                } else {
                    if file_only {
                        Some(path) // This is a file
                    } else {
                        // If it's a file, get its parent directory
                        let path_buf = PathBuf::from(&path);
                        if let Some(parent) = path_buf.parent() {
                            Some(parent.to_string_lossy().to_string())
                        } else {
                            Some(current)
                        }
                    }
                }
            } else {
                Some(current)
            }
        } else {
            Some(current)
        }
    }

    pub fn create_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dir_path) = self.get_selected_path(false) {
            let new_file_path = format!("{}/new_file.md", dir_path);
            self.file_service.create_file(&new_file_path, "# New File\n\nEnter your content here.")?;
            self.force_refresh();
        }
        Ok(())
    }

    pub fn create_directory(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dir_path) = self.get_selected_path(false) {
            let new_dir_path = format!("{}/new_directory", dir_path);
            self.file_service.create_directory(&new_dir_path)?;
            
            // Create a README.md file in the new directory to make it visible in the tree
            let readme_path = format!("{}/README.md", new_dir_path);
            self.file_service.create_file(&readme_path, "# New Directory\n\nThis is a new directory.")?;
            
            self.force_refresh();
        }
        Ok(())
    }

    pub fn delete_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = self.get_selected_path(true) {
            self.file_service.delete_file(&file_path)?;
            self.force_refresh();
        }
        Ok(())
    }

    pub fn delete_directory(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dir_path) = self.get_selected_path(false) {
            self.file_service.delete_directory(&dir_path)?;
            self.force_refresh();
        }
        Ok(())
    }

    pub fn get_selected_file_for_opening(&self) -> Option<String> {
        if let Some(file_path) = self.get_selected_path(true) {
            if file_path.ends_with(".md") {
                Some(file_path)
            } else {
                None
            }
        } else {
            None
        }
    }
}