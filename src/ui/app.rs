use std::io;
use std::time::Duration;
use ratatui::crossterm::event::{EnableMouseCapture, Event, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::oneshot;

use crate::app::config::ConfigService;
use crate::app::events::{AppEvent, EventHandler};
use crate::app::state::{AppState, FocusedInputArea};
use crate::core::chat::ChatService;
use crate::core::memory::PromptType;
use crate::ui::layout::{LayoutManager, MainLayout};
use crate::ui::popups::popup_manager::PopupWrapper;
use crate::ui::popups::yes_no::{YesNoPopup, YesNoState};
use crate::ui::widgets::chat_input::ChatInputWidget;
use crate::ui::widgets::chat_output::ChatOutputWidget;
use crate::ui::widgets::project_tree::ProjectTreeWidget;
use crate::ui::popups::command_selector::{CommandSelector, CommandSelectorState};
use crate::ui::popups::file_selector::{FileSelector, FileSelectorState};
use crate::message_popup::create_message_popup;

pub struct ChatUIApp<'a> {
    state: AppState,
    config_service: ConfigService,
    chat_service: ChatService,
    
    // UI Components
    layout_manager: LayoutManager,
    chat_input: ChatInputWidget<'a>,
    chat_output: ChatOutputWidget<'a>,
    project_tree: ProjectTreeWidget<'a>,
    
    // Popups
    command_selector: CommandSelector,
    file_selector: FileSelector,
    yes_no_popup: PopupWrapper<YesNoState>,
    yes_no_callback: YesNoCallback,
    
    // Event handling
    event_handler: EventHandler,
    
    // LLM response handling
    llm_rx: Option<oneshot::Receiver<String>>,
}

type YesNoCallback = fn(&mut ChatUIApp) -> bool;

impl<'a> ChatUIApp<'a> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            state: AppState::new(),
            config_service: ConfigService::new()?,
            chat_service: ChatService::new()?,
            
            layout_manager: LayoutManager::new(),
            chat_input: ChatInputWidget::new(),
            chat_output: ChatOutputWidget::new(),
            project_tree: ProjectTreeWidget::new(),
            
            command_selector: CommandSelector::new(),
            file_selector: FileSelector::new(),
            yes_no_popup: PopupWrapper::new(),
            yes_no_callback: dummy_callback(),
            
            event_handler: EventHandler::new(),
            
            llm_rx: None,
        })
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<(), io::Error> {
        ratatui::crossterm::terminal::enable_raw_mode()?;

        execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            ratatui::crossterm::terminal::Clear(ratatui::crossterm::terminal::ClearType::All),
            ratatui::crossterm::cursor::MoveTo(0, 0)
        )?;

        loop {
            terminal.draw(|frame| {
                if self.yes_no_popup.is_visible() {
                    self.yes_no_popup.draw(frame);
                } else {
                    self.draw_main_screen(frame);
                }
            })?;

            // Check for LLM response
            self.check_llm_response();

            // Handle events
            if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
                match ratatui::crossterm::event::read()? {
                    Event::Key(key) => {
                        if self.yes_no_popup.is_visible() {
                            match self.yes_no_popup.handle_key_event(key) {
                                crate::ui::popups::popup_manager::PopupState::Exit => {
                                    self.handle_yes_no_popup_result();
                                },
                                _ => {}
                            }
                        } else if let Some(app_event) = self.event_handler.handle_key_event(key, &self.state) {
                            if self.handle_app_event(app_event, key)? {
                                break;
                            }
                        }
                    },
                    _ => {
                        // Handle answer text wrapping if needed
                        if !self.chat_output.get_content().is_empty() {
                            let layout = self.layout_manager.create_main_layout(&terminal.get_frame());
                            self.chat_output.set_content(
                                &self.state.answer_prompt.value,
                                layout.answer_area.width
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn draw_main_screen(&mut self, frame: &mut Frame) {
        let layout = self.layout_manager.create_main_layout(frame);

        // Render widgets
        self.project_tree.render(
            frame, 
            layout.project_tree_area, 
            self.state.current_focus_area == FocusedInputArea::ProjectTree
        );

        self.chat_input.render(
            frame, 
            layout.question_area, 
            self.state.current_focus_area == FocusedInputArea::Question
        );

        self.chat_output.render(
            frame, 
            layout.answer_area, 
            self.state.current_focus_area == FocusedInputArea::Answer
        );

        // Render popups
        if self.state.show_commands_popup {
            self.command_selector.render_commands_popup(frame);
        } else if self.state.show_files_popup {
            self.file_selector.render_files_popup(frame);
        }
    }

    fn handle_app_event(&mut self, event: AppEvent, key: ratatui::crossterm::event::KeyEvent) -> Result<bool, io::Error> {
        use crate::autocomplete;
        
        match event {
            AppEvent::Exit => {
                autocomplete::save_history();
                return Ok(true);
            },
            AppEvent::FocusNext => {
                if self.state.current_focus_area == FocusedInputArea::Question && !self.state.edit_mode {
                    self.state.cycle_focus();
                } else if self.state.current_focus_area == FocusedInputArea::Question {
                    self.chat_input.handle_key_event(key, self.state.edit_mode);
                } else {
                    self.state.cycle_focus();
                }
                self.state.reset_escape_count();
            },
            AppEvent::SaveFile => {
                if let Some(_file_path) = self.project_tree.get_selected_file_for_opening() {
                    if !self.state.last_file_path.is_empty() {
                        let _ = self.chat_input.save_file(&self.state.last_file_path);
                    }
                }
                self.state.set_focus(FocusedInputArea::ProjectTree);
                self.state.edit_mode = false;
                self.state.increment_escape_count();
            },
            AppEvent::ExecuteLLM => {
                if self.llm_rx.is_none() {
                    self.execute_llm_command();
                }
            },
            AppEvent::OpenFile => {
                if let Some(file_path) = self.project_tree.get_selected_file_for_opening() {
                    self.state.last_file_path = file_path.clone();
                    let _ = self.chat_input.open_file(&file_path);
                }
            },
            AppEvent::CreateFile => {
                let _ = self.project_tree.create_file();
            },
            AppEvent::CreateDirectory => {
                let _ = self.project_tree.create_directory();
            },
            AppEvent::DeleteFile => {
                self.show_yes_no_popup("Delete file?".to_string(), |app| {
                    match app.project_tree.delete_file() {
                        Ok(_) => {
                            create_message_popup("File removed!".to_string());
                            true
                        },
                        Err(e) => {
                            create_message_popup(e.to_string());
                            false
                        }
                    }
                });
            },
            AppEvent::DeleteDirectory => {
                self.show_yes_no_popup("Delete directory?".to_string(), |app| {
                    match app.project_tree.delete_directory() {
                        Ok(_) => {
                            create_message_popup("Directory removed!".to_string());
                            true
                        },
                        Err(e) => {
                            create_message_popup(e.to_string());
                            false
                        }
                    }
                });
            },
            AppEvent::RenameItem => {
                // TODO: Implement rename functionality
            },
            AppEvent::ShowCommandsPopup => {
                self.state.show_commands_popup();
            },
            AppEvent::ShowFilesPopup => {
                self.state.show_files_popup();
            },
            AppEvent::Continue => {
                self.state.reset_escape_count();
                self.handle_continue_event(key)?;
            },
            AppEvent::ToggleEditMode => {
                self.state.toggle_edit_mode();
            },
        }
        Ok(false)
    }

    fn handle_continue_event(&mut self, key: ratatui::crossterm::event::KeyEvent) -> Result<(), io::Error> {
        if self.state.show_commands_popup {
            let (command, state) = self.command_selector.handle_key(key);
            if command.is_some() && state == CommandSelectorState::Selected {
                self.chat_input.insert_text(&command.unwrap().usage_example);
                self.state.hide_commands_popup();
            } else if state == CommandSelectorState::Exit {
                self.state.hide_commands_popup();
            }
        } else if self.state.show_files_popup {
            let (file_name, state) = self.file_selector.handle_key(key);
            if file_name.is_some() && state == FileSelectorState::Selected {
                self.chat_input.insert_text(&file_name.unwrap());
                self.state.hide_files_popup();
            } else if state == FileSelectorState::Exit {
                self.state.hide_files_popup();
            }
        } else {
            match self.state.current_focus_area {
                FocusedInputArea::ProjectTree => {
                    self.project_tree.handle_event(Event::Key(key))?;
                },
                FocusedInputArea::Question => {
                    if !self.state.edit_mode && key.code != ratatui::crossterm::event::KeyCode::Tab {
                        self.state.edit_mode = true;
                    }
                    if self.state.edit_mode {
                        self.chat_input.handle_key_event(key, self.state.edit_mode);
                    }
                },
                FocusedInputArea::Answer => {
                    self.chat_output.handle_key_event(key);
                },
            }
        }
        Ok(())
    }

    fn check_llm_response(&mut self) {
        if let Some(rx) = self.llm_rx.as_mut() {
            match rx.try_recv() {
                Ok(response_text) => {
                    self.state.answer_prompt = self.chat_service.add_answer(response_text.clone());
                    // Use a reasonable default width for wrapping, will be updated on next draw
                    self.chat_output.set_content(&response_text, 80);
                    self.llm_rx = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // Not ready yet
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.chat_output.set_content("Error: LLM task failed or was cancelled.", 80);
                    self.llm_rx = None;
                }
            }
        }
    }

    fn execute_llm_command(&mut self) {
        let content = self.chat_input.get_content();
        let (enriched_input, offline) = self.chat_service.process_input(&content);
        
        if offline {
            self.chat_output.set_content(&enriched_input, 80);
            return;
        }

        self.state.question_prompt = self.chat_service.add_question(enriched_input.clone());
        self.chat_input.set_title(&format!("YOU: [ID:{}]", self.state.question_prompt.id));

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.llm_rx = Some(rx);

        let chat_service = self.chat_service.clone();
        let config = self.config_service.get_config().clone();
        
        tokio::spawn(async move {
            match chat_service.get_llm_response(&enriched_input, &config).await {
                Ok(response_text) => {
                    if tx.send(response_text).is_err() {
                        eprintln!("LLM task: Receiver for response was dropped.");
                    }
                },
                Err(e) => {
                    let error_msg = format!("Error calling LLM API: {}", e);
                    eprintln!("{}", error_msg);
                }
            }
        });
    }

    fn show_yes_no_popup(&mut self, title: String, callback: YesNoCallback) {
        self.yes_no_callback = callback;
        let _ = self.yes_no_popup.show(Box::new(YesNoPopup::new(title)));
    }

    fn handle_yes_no_popup_result(&mut self) {
        if let Some(result) = self.yes_no_popup.get_result() {
            match result {
                YesNoState::Yes => {
                    (self.yes_no_callback)(self);
                    let _ = self.yes_no_popup.hide();
                },
                _ => {
                    let _ = self.yes_no_popup.hide();
                }
            }
        }
    }
}

fn dummy_callback() -> YesNoCallback {
    fn empty_callback(_: &mut ChatUIApp) -> bool { false }
    empty_callback
}