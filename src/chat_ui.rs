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
use std::time::Duration;  
use tokio::sync::oneshot;  

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
    cmd_sel : CommandSelector,
    file_sel : FileSelector,
    question_text_area: TextArea<'a>,
    answer_text_area: TextArea<'a>,
    llm_rx: Option<oneshot::Receiver<String>>, // Add this field
}

impl ChatUIApp<'_> {
    pub fn new() -> Self {
        Self {
            show_commands_popup: false,
            show_files_popup: false,
            cmd_sel: CommandSelector::new(),
            file_sel: FileSelector::new(),
            question_text_area: TextArea::default(),
            answer_text_area: TextArea::default(),
            llm_rx: None, // Initialize here
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

            // Check for LLM response non-blockingly
            if let Some(rx) = self.llm_rx.as_mut() { // Borrow mutably to call try_recv
                match rx.try_recv() {
                    Ok(response) => {
                        self.answer_text_area.insert_str(&response);
                        self.llm_rx = None; // Clear the receiver once handled
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {
                        // Not ready yet, do nothing, will check next loop iteration
                    }
                    Err(oneshot::error::TryRecvError::Closed) => {
                        // Sender dropped (task panicked or completed without sending)
                        self.answer_text_area.insert_str("Error: LLM task failed or was cancelled.");
                        self.llm_rx = None; // Clear the receiver
                    }
                }
            }

            // Poll for crossterm events with a timeout
            // This makes the loop iterate even if there are no key presses,
            // allowing the llm_rx check above to run.
            if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
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
                                // let content: Vec<String> = self.question_text_area.lines().to_vec();
                                // tokio::spawn(async move { execute_offline_command(&content).await });
                            }
                            KeyCode::Char('>') => {
                                // let content: Vec<String> = self.question_text_area.lines().to_vec();
                                // tokio::spawn(async move { execute_offline_command(&content).await });
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
                            KeyCode::Char('±') => {
                                if key.kind == KeyEventKind::Press {
                                    if self.llm_rx.is_none() {
                                        self.execute_llm_command();
                                    } else {
                                        // Optionally, provide feedback that a command is already in progress
                                        // self.answer_text_area.insert_str("An LLM command is already running...\n");
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
    }

    fn execute_llm_command(&mut self) {
        let content: Vec<String> = self.question_text_area.lines().to_vec();
        let content = content.join(&"\n");

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.llm_rx = Some(rx); // Store the receiver

        // This tokio::spawn will use the existing runtime (e.g., from #[tokio::main])
        tokio::spawn(async move {
            let (enriched_input, _offline) = check_embedded_commands(content.as_str()).await;
            let _question = Prompt::new(enriched_input.clone(), PromptType::QUESTION);

            match openrouter::call_openrouter_api(&enriched_input).await {
                Ok(response_text) => {
                    let ans_prompt = Prompt::new(response_text, PromptType::ANSWER);
                    let highlighted_response = highlight_code(ans_prompt.value.as_str());
                    if tx.send(highlighted_response).is_err() {
                        // Receiver was dropped, maybe UI closed or another command started
                        eprintln!("LLM task: Receiver for response was dropped.");
                    }
                },
                Err(e) => {
                    let error_msg = format!("Error calling OpenRouter API: {}", e);
                    eprintln!("{}", error_msg); // Log to console
                    if tx.send(format!("Error: {}", e)).is_err() {
                        eprintln!("LLM task: Receiver for error response was dropped.");
                    }
                }
            }
        });

    }
}