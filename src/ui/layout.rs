use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct LayoutManager;

impl LayoutManager {
    pub fn new() -> Self {
        Self
    }

    pub fn create_main_layout(&self, frame: &Frame) -> MainLayout {
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(20), // Project tree
                Constraint::Percentage(80), // Chat area
            ])
            .split(frame.area());

        let chat_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(30), // Question area
                Constraint::Percentage(70), // Answer area
            ])
            .split(main_layout[1]);

        MainLayout {
            project_tree_area: main_layout[0],
            question_area: chat_layout[0],
            answer_area: chat_layout[1],
        }
    }
}

pub struct MainLayout {
    pub project_tree_area: Rect,
    pub question_area: Rect,
    pub answer_area: Rect,
}