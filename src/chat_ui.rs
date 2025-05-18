
use color_eyre::Result;
use ratatui::{
    crossterm::event::{Event, KeyCode },
    layout::{Constraint, Layout, Direction,},

    widgets::{
        Block, Borders,
    },
    DefaultTerminal,
};
use ratatui::widgets::Clear;
use tui_textarea::{ TextArea };
use crate::{commands, commands_selector, configuration, terminal};
use commands_selector::CommandSelector;
use crate::commands_selector::CommandSelectorState;

pub fn main_ui() -> Result<()> {
    color_eyre::install()?;

    commands::register_all_commands();
    let config = configuration::load_configuration();

    println!(
        "{}",
        terminal::format_info(&format!("Loaded config: {:?}", config))
    );

    let terminal = ratatui::init();
    let app_result = ChatUIApp::new().run(terminal);

    ratatui::restore();
    app_result
}

struct ChatUIApp {
    show_commands_popup: bool,
    cmd_sel : CommandSelector
}

impl ChatUIApp {
    pub fn new() -> Self {
        Self {
            show_commands_popup: false,
            cmd_sel: CommandSelector::new()
        }
    }

}

impl ChatUIApp {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut question_text_area = TextArea::default();
        question_text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("YOU:"),
        );

        let mut answer_text_area = TextArea::default();
        answer_text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("LLM:"),
        );


        loop {
            terminal.draw(|frame| {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(frame.area());

                frame.render_widget(Clear, frame.area());
                frame.render_widget(&question_text_area, layout[0]);
                frame.render_widget(&answer_text_area, layout[1]);
                if self.show_commands_popup {
                    self.cmd_sel.render_commands_popup(frame);
                }
            })?;

            if let Event::Key(key) = crossterm::event::read()? {
                // Your own key mapping to break the event loop
                if self.show_commands_popup {
                    let (command, state) = self.cmd_sel.handle_key(key);
                    if command.is_some() && state == CommandSelectorState::Selected {
                        question_text_area.insert_str(command.unwrap().usage_example.as_str());
                        self.show_commands_popup = false
                    }
                    else if state == CommandSelectorState::Exit{
                        self.show_commands_popup = false
                    }
                }
                else {
                    match key.code {
                        KeyCode::Char('@') => {
                            self.show_commands_popup = true
                        },
                        KeyCode::Esc => {
                            break Ok(())
                        }
                        _ => {
                            question_text_area.input(key);
                        }
                    }
                }

            }
        }
    }
}