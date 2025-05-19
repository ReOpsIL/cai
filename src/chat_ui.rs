use std::io::{stdout, Write};
use color_eyre::Result;
use ratatui::crossterm::execute;
use ratatui::{
    crossterm::event::{Event, KeyCode },
    layout::{Constraint, Layout, Direction,},

    widgets::{
        Block, Borders,
    },
    DefaultTerminal,
};
use ratatui::crossterm::event::{KeyEventKind, KeyModifiers};
use ratatui::widgets::Clear;
use tui_textarea::{ TextArea };
use crate::{autocomplete, commands, commands_registry, commands_selector, configuration, openrouter, terminal};
use commands_selector::CommandSelector;
use crate::chat::{check_embedded_commands, highlight_code, Prompt, PromptType};
use crate::commands_selector::CommandSelectorState;
use crate::files_selector::{FileSelector, FileSelectorState};

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

struct ChatUIApp<'a> {
    show_commands_popup: bool,
    show_files_popup: bool,
    start_llm: bool,
    cmd_sel : CommandSelector,
    file_sel : FileSelector,
    question_text_area: TextArea<'a>,
    answer_text_area: TextArea<'a>,
    llm_question: String,
}

impl ChatUIApp<'_> {
    pub fn new() -> Self {
        Self {
            show_commands_popup: false,
            show_files_popup: false,
            start_llm: false,
            cmd_sel: CommandSelector::new(),
            file_sel: FileSelector::new(),
            question_text_area: TextArea::default(),
            answer_text_area: TextArea::default(),
            llm_question: String::new(),
        }
    }

}

impl ChatUIApp<'_> {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {

        let mut stdout = stdout();
        ratatui::crossterm::terminal::enable_raw_mode()?;

        execute!(
            stdout,
            ratatui::crossterm::terminal::Clear(ratatui::crossterm::terminal::ClearType::All),
            ratatui::crossterm::cursor::MoveTo(0, 0)
        )?;
        stdout.flush()?;

        self.question_text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("YOU:"),
        );

        self.answer_text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("LLM:"),
        );

        loop {
            if self.start_llm {
                self.start_llm = false;
                let question = self.llm_question.clone();
                self.execute_llm_command(question);
            }
            terminal.draw(|frame| {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ])
                    .split(frame.area());

                frame.render_widget(Clear, frame.area());
                frame.render_widget(&self.question_text_area, layout[0]);
                frame.render_widget(&self.answer_text_area, layout[1]);
                if self.show_commands_popup {
                    self.cmd_sel.render_commands_popup(frame);
                }
                if self.show_files_popup {
                    self.file_sel.render_files_popup(frame)
                }
            })?;

            if let Event::Key(key) = ratatui::crossterm::event::read()? {
                // Your own key mapping to break the event loop
                if self.show_commands_popup {
                    let (command, state) = self.cmd_sel.handle_key(key);
                    if command.is_some() && state == CommandSelectorState::Selected {
                        self.question_text_area.insert_str(command.unwrap().usage_example.as_str());
                        self.show_commands_popup = false
                    }
                    else if state == CommandSelectorState::Exit{
                        self.show_commands_popup = false
                    }
                }
                else if self.show_files_popup {
                    let (file_name, state) = self.file_sel.handle_key(key);
                    if file_name.is_some() && state == FileSelectorState::Selected {
                        self.question_text_area.insert_str(file_name.unwrap().as_str());
                        self.show_files_popup = false
                    }
                    else if state == FileSelectorState::Exit{
                        self.show_files_popup = false
                    }
                }
                else {
                    match key.code {
                        KeyCode::Char('?') => {
                            commands_registry::print_help();
                        }
                        KeyCode::Char('!') => {
                            let content: Vec<String> = self.question_text_area.lines().to_vec();
                            //tokio::spawn(async move { execute_offline_command(&content).await });
                        }
                        KeyCode::Char('>') => {
                            let content: Vec<String> = self.question_text_area.lines().to_vec();
                            //tokio::spawn(async move { execute_offline_command(&content).await });
                        }
                        KeyCode::Char('@') => {
                            self.show_commands_popup = true
                        },
                        KeyCode::Char('$') => {
                            self.show_files_popup = true
                        },
                        KeyCode::Esc => {
                            autocomplete::save_history();
                            break Ok(())
                        },
                        KeyCode::Char('\\') => {
                            if key.kind == KeyEventKind::Press {  // First check if it's a press event
                                if key.modifiers == KeyModifiers::ALT {
                                    self.prepare_to_run_llm();
                                }
                            }
                        },
                        _ => {
                            self.question_text_area.input(key);
                        }
                    }
                }

            }
        }
    }
    fn prepare_to_run_llm(&mut self) {
        let content: Vec<String> = self.answer_text_area.lines().to_vec();
        self.llm_question = content.join(&"\n");
        self.start_llm = true;
    }

    fn execute_llm_command(&mut self, content:String) {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let (enriched_input, _offline) = check_embedded_commands(content.as_str()).await;
            let _question = Prompt::new(enriched_input.clone(), PromptType::QUESTION);

            match openrouter::call_openrouter_api(&enriched_input).await {
                Ok(response_text) => {
                    let ans_prompt = Prompt::new(response_text, PromptType::ANSWER);
                    let highlighted_response = highlight_code(ans_prompt.value.as_str());
                    let _ = tx.send(highlighted_response);
                },
                Err(e) => {
                    println!("Error calling OpenRouter API: {}", e);
                    let _ = tx.send(format!("Error: {}", e));
                }
            }
        });

        if let Ok(response) = rx.blocking_recv() {
            self.answer_text_area.insert_str(&response);
        }
    }

}


async fn execute_offline_command(lines: &[String]) {
    let content = lines.join(&"\n");

    match crate::chat::execute_command(content.as_str()).await {
        Ok(Some(output)) => match output.command_output {
            Ok(Some(output_str)) => {
                println!("{}", output_str);
            }
            Err(e) => {
                println!("Error executing command: {}", e);
            }
            Ok(None) => {
                println!(
                    "{}",
                    terminal::format_error("Sorry - Unrecognized command...")
                );
                commands_registry::print_help();
            }
        },
        Ok(None) => {
            println!(
                "{}",
                terminal::format_error("Sorry - Unrecognized command...")
            );
            commands_registry::print_help();
        }
        Err(e) => {
            println!("Error executing command: {}", e);
        }
    }

}
