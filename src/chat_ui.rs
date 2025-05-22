use std::io::{stdout, Write};
use color_eyre::Result;
use ratatui::crossterm::execute;
use ratatui::{
    crossterm::event::{Event, KeyCode },
    layout::{Constraint, Layout, Direction},
    widgets::{
        Block, Borders,
    },
    DefaultTerminal,
};
use ratatui::widgets::Clear;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use tui_textarea::{ TextArea };
use crate::{autocomplete, commands, commands_registry, commands_selector, configuration, openrouter, terminal};
use commands_selector::CommandSelector;
use crate::chat::{check_embedded_commands, Prompt, PromptType}; // Removed highlight_code
use crate::commands_selector::CommandSelectorState;
use crate::files_selector::{FileSelector, FileSelectorState};
use std::time::Duration;
use ratatui::crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Position, Rect};
use tokio::sync::oneshot;

pub enum FocusedInputArea {
    Question,
    Answer,
}

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
    question_text_widget: TextArea<'a>,
    answer_text_widget: TextArea<'a>,
    question_text_rect: Rect,
    answer_text_rect: Rect,
    question_prompt: Prompt,
    answer_prompt: Prompt,
    llm_rx: Option<oneshot::Receiver<String>>, // Add this field
    current_focus_area: FocusedInputArea,
}

impl ChatUIApp<'_> {
    pub fn new() -> Self {
        Self {
            show_commands_popup: false,
            show_files_popup: false,
            cmd_sel: CommandSelector::new(),
            file_sel: FileSelector::new(),
            question_text_widget: TextArea::default(),
            answer_text_widget: TextArea::default(),
            question_text_rect: Rect::default(),
            answer_text_rect: Rect::default(),
            question_prompt: Prompt::default(),
            answer_prompt: Prompt::default(),
            llm_rx: None,
            current_focus_area: FocusedInputArea::Question,
        }
    }
}

impl ChatUIApp<'_> {

    fn create_textarea_block<'a>(&self, title: String) -> Block<'a> {
        Block::default().borders(Borders::ALL).title(title)
    }

    fn wrap_answer_widget(&mut self) {
        let wrapped_str = textwrap::wrap(self.answer_prompt.value.as_str(), self.answer_text_rect.width as usize).join("\n");
        //let highlighted_response = highlight_code(ans_prompt.value.as_str());
        self.answer_text_widget = TextArea::default();
        self.answer_text_widget.set_block(self.create_textarea_block(format!("LLM: [ID:{}]", self.answer_prompt.id)));
        self.answer_text_widget.insert_str(wrapped_str);
    }
    fn focus_at_mouse_pos(&mut self, col: u16, row: u16) {
        let mouse_pos = Position { x: col, y: row };
        if self.question_text_rect.contains(mouse_pos) {
            self.current_focus_area = FocusedInputArea::Question;
        } else if self.answer_text_rect.contains(mouse_pos) {
            self.current_focus_area = FocusedInputArea::Answer;
        }
    }


    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut stdout = stdout();
        ratatui::crossterm::terminal::enable_raw_mode()?;

        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        execute!(
            stdout,
            EnterAlternateScreen, EnableMouseCapture,
            ratatui::crossterm::terminal::Clear(ratatui::crossterm::terminal::ClearType::All),
            ratatui::crossterm::cursor::MoveTo(0, 0)
        )?;
        stdout.flush()?;

        self.question_text_widget.set_block(self.create_textarea_block("YOU:".to_string()));
        self.answer_text_widget.set_block(self.create_textarea_block("LLM:".to_string()));

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

                self.question_text_rect = layout[0];
                self.answer_text_rect = layout[1];

                // Render the TextAreas - TextArea has built-in scrolling functionality
                frame.render_widget(&self.question_text_widget, layout[0]);
                frame.render_widget(&self.answer_text_widget, layout[1]);
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
                    Ok(response_text) => {
                        self.answer_prompt = Prompt::new(response_text, PromptType::ANSWER);

                        self.answer_text_widget = TextArea::default();
                        self.answer_text_widget.set_block(self.create_textarea_block(format!("LLM: [ID:{}]", self.answer_prompt.id)));

                        let wrapped_str = textwrap::wrap(self.answer_prompt.value.as_str(), self.answer_text_rect.width as usize).join("\n");
                        //let highlighted_response = highlight_code(ans_prompt.value.as_str());
                        self.answer_text_widget.insert_str(wrapped_str);

                        //self.answer_text_widget.insert_str(prompt.value.as_str());
                        self.llm_rx = None; // Clear the receiver once handled
                        
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {
                        // Not ready yet, do nothing, will check next loop iteration
                    }
                    Err(oneshot::error::TryRecvError::Closed) => {
                        // Sender dropped (task panicked or completed without sending)
                        self.answer_text_widget.insert_str("Error: LLM task failed or was cancelled.");
                        self.llm_rx = None; // Clear the receiver
                    }
                }
            }

            // Poll for crossterm events with a timeout
            // This makes the loop iterate even if there are no key presses,
            // allowing the llm_rx check above to run.
            if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
                match ratatui::crossterm::event::read()? {
                    Event::Key(key) => {
                        if let Some(_) = self.handle_key_event(key)? {
                            break Ok(());
                        }
                    },
                    Event::Mouse(mouse_event) => {
                        self.handle_mouse_event(mouse_event);
                    }
                    _ => {
                        if self.answer_text_widget.lines().len() > 0 {  
                            self.wrap_answer_widget()
                        }
                    }

                }
            }
        }

    }
    fn handle_command_key(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        let (command, state) = self.cmd_sel.handle_key(key);
        if command.is_some() && state == CommandSelectorState::Selected {
            self.question_text_widget.insert_str(command.unwrap().usage_example.as_str());
            self.show_commands_popup = false;
        } else if state == CommandSelectorState::Exit {
            self.show_commands_popup = false;
        }
    }
    
    fn handle_files_key(&mut self , key: ratatui::crossterm::event::KeyEvent) {
        let (file_name, state) = self.file_sel.handle_key(key);
        if file_name.is_some() && state == FileSelectorState::Selected {
            self.question_text_widget.insert_str(file_name.unwrap().as_str());
            self.show_files_popup = false;
        } else if state == FileSelectorState::Exit {
            self.show_files_popup = false;
        }
    }

    fn handle_key_event(&mut self, key: ratatui::crossterm::event::KeyEvent) -> color_eyre::Result<Option<()>> {
        match key.code {
            KeyCode::Char('?') => {
                commands_registry::print_help();
            }
            KeyCode::Char('!') => {
                // Functionality for '!' was commented out
            }
            KeyCode::Char('>') => {
                // Functionality for '>' was commented out
            }
            KeyCode::Char('@') => {
                self.show_commands_popup = true;
            }
            KeyCode::Char('$') => {
                self.show_files_popup = true;
            }
            KeyCode::Esc => {
                if self.show_commands_popup {
                    self.handle_command_key(key)
                } else if self.show_files_popup {
                    self.handle_files_key(key)
                }
                else {
                    autocomplete::save_history();
                    return Ok(Some(())); // Signal to exit    
                }
            }
            KeyCode::F(1) => {
                if key.kind == KeyEventKind::Press {
                    if self.llm_rx.is_none() {
                        self.execute_llm_command();
                    } else {
                        // Optionally, provide feedback that a command is already in progress
                        // self.answer_text_area.insert_str("An LLM command is already running...\n");
                    }
                }
            }
            _ => {
                if self.show_commands_popup {
                   self.handle_command_key(key)
                } else if self.show_files_popup {
                   self.handle_files_key(key)
                }

                match self.current_focus_area {
                    FocusedInputArea::Question => {
                        self.question_text_widget.input(key);
                    }
                    FocusedInputArea::Answer => {
                        match key.code {
                            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                                self.answer_text_widget.input(key);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(None) // Continue running
    }

    fn handle_mouse_event(&mut self, mouse_event: ratatui::crossterm::event::MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                self.focus_at_mouse_pos(mouse_event.column, mouse_event.row);
            }
            _ => {}
        }
    }

    fn execute_llm_command(&mut self) {
        let content: Vec<String> = self.question_text_widget.lines().to_vec();
        let content = content.join(&"\n");

        let (enriched_input, _offline) = check_embedded_commands(content.as_str());
        if _offline {
            self.answer_text_widget = TextArea::default();
            self.answer_text_widget.set_block(self.create_textarea_block("LLM: [LOCAL]".to_string()));

            self.answer_text_widget.insert_str(enriched_input.as_str());
            return;
        }
        self.question_prompt = Prompt::new(enriched_input.clone(), PromptType::QUESTION);

        self.question_text_widget.set_block(self.create_textarea_block(format!("YOU: [ID:{}]", self.question_prompt.id)));

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.llm_rx = Some(rx); // Store the receiver

        // This tokio::spawn will use the existing runtime (e.g., from #[tokio::main])
        tokio::spawn(async move {
            match openrouter::call_openrouter_api(&enriched_input).await {
                Ok(response_text) => {
                    if tx.send(response_text).is_err() {
                        // Receiver was dropped, maybe UI closed or another command started
                        eprintln!("LLM task: Receiver for response was dropped.");
                    }
                },
                Err(e) => {
                    let error_msg = format!("Error calling OpenRouter API: {}", e);
                    eprintln!("{}", error_msg); // Log to console
                    eprintln!("LLM task: Receiver for error response was dropped.");
                }
            }
        });

    }


}
