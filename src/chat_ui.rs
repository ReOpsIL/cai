use std::io::{stdout, Write};
use color_eyre::Result;
use ratatui::crossterm::execute;
use ratatui::{crossterm::event::{Event, KeyCode}, layout::{Constraint, Layout, Direction}, widgets::{
    Block, Borders,
}, DefaultTerminal, Frame};
use ratatui::widgets::{Clear, Scrollbar, ScrollbarOrientation};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use tui_textarea::{CursorMove, TextArea};
use crate::{autocomplete, commands, commands_registry, commands_selector, configuration, openrouter, terminal};
use commands_selector::CommandSelector;
use crate::chat::{check_embedded_commands, Prompt, PromptType}; // Removed highlight_code
use crate::commands_selector::CommandSelectorState;
use crate::files_selector::{FileSelector, FileSelectorState};
use std::time::Duration;
use ratatui::crossterm::style::Color;
use ratatui::crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use rustyline::KeyEvent;
use tokio::sync::oneshot;
use tui_tree_widget::{Tree, TreeItem, TreeState};
use crate::tree::{generate_md_tree, TreeNode};

pub enum FocusedInputArea {
    Question,
    Answer,
    ProjectTree,
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
    project_tree_widget_rect: Rect,
    question_prompt: Prompt,
    answer_prompt: Prompt,
    llm_rx: Option<oneshot::Receiver<String>>, // Add this field
    current_focus_area: FocusedInputArea,
    state: TreeState<String>,
    project_tree_items: Vec<TreeItem<'a, String>>,
    project_tree_do_refresh: bool,
}

impl ChatUIApp<'_> {
    pub fn new() -> Self {
        let mut ret = Self {
            show_commands_popup: false,
            show_files_popup: false,
            cmd_sel: CommandSelector::new(),
            file_sel: FileSelector::new(),
            question_text_widget: TextArea::default(),
            answer_text_widget: TextArea::default(),
            question_text_rect: Rect::default(),
            answer_text_rect: Rect::default(),
            project_tree_widget_rect: Rect::default(),
            question_prompt: Prompt::default(),
            answer_prompt: Prompt::default(),
            llm_rx: None,
            current_focus_area: FocusedInputArea::Question,
            state: TreeState::default(),
            project_tree_items: Vec::new(),
            project_tree_do_refresh: true,
        };
        let style = Style::default();
        ret.question_text_widget.set_line_number_style(style);

        ret
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
    fn handle_mouse_click_in_widget(widget: &mut TextArea, widget_rect: Rect, col: u16, row: u16) {

        let (cur_row, _cur_col) = widget.cursor();
        if  widget_rect.height == 0 || widget_rect.width == 0 {
            return ;
        }

        let mut page_start = 0;
        if cur_row > widget_rect.height as usize  {
            page_start = (cur_row  / widget_rect.height as usize) * widget_rect.height as usize;
        }

        widget.move_cursor(CursorMove::Jump((page_start + row as usize) as u16, col))
    }
    fn focus_at_mouse_pos(&mut self, col: u16, row: u16, is_click: bool) {
        let mouse_pos = Position { x: col, y: row };
        if self.question_text_rect.contains(mouse_pos) {
            self.current_focus_area = FocusedInputArea::Question;
            // if is_click {
            //     Self::handle_mouse_click_in_widget(&mut self.question_text_widget, self.question_text_rect, col, row);
            // }
        } else if self.answer_text_rect.contains(mouse_pos) {
            self.current_focus_area = FocusedInputArea::Answer;
            // if is_click {
            //     Self::handle_mouse_click_in_widget(&mut self.answer_text_widget, self.answer_text_rect, col, row);
            // }

        } else if self.project_tree_widget_rect.contains(mouse_pos) {
            self.current_focus_area = FocusedInputArea::ProjectTree;
        }
    }

    fn refresh_project_tree(&mut self) {
        if self.project_tree_do_refresh == false {
            return
        }

        match generate_md_tree(".") {
            Ok(tree_items) => {
                self.project_tree_items = tree_items;
                self.project_tree_do_refresh = false
            },
            Err(e) => {
                eprintln!("Error generating project tree: {}", e);
                return;
            }
        };
    }
    fn create_project_tree_widget<'a>(tree_items: &'a Vec<TreeItem<String>>) -> Tree<'a, String> {
        Tree::new(tree_items)
            .expect("all item identifiers are unique")
            .block(
                Block::bordered()
                    .title("Tree Widget"),
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
                    .bg(ratatui::style::Color::LightYellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
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

                let outer_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ])
                    .split(frame.area());

                let qa_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ])
                    .split(outer_layout[1]);

                frame.render_widget(Clear, frame.area());

                self.project_tree_widget_rect = outer_layout[0];
                self.question_text_rect = qa_layout[0];
                self.answer_text_rect = qa_layout[1];

                self.refresh_project_tree();
                let tree_widget = ChatUIApp::create_project_tree_widget(&self.project_tree_items);
                frame.render_stateful_widget(tree_widget, self.project_tree_widget_rect, &mut self.state);

                frame.render_widget(&self.question_text_widget, self.question_text_rect);
                frame.render_widget(&self.answer_text_widget, self.answer_text_rect);
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
                        self.handle_tree_key_event(Event::Key(key))?;
                    },
                    Event::Mouse(mouse_event) => {
                        self.handle_mouse_event(mouse_event);
                        self.handle_tree_key_event(Event::Mouse(mouse_event))?;
                    }
                    _ => {
                        if self.answer_text_widget.lines().len() > 0 {
                            self.wrap_answer_widget()
                        }
                    }

                };
            }
        }

    }
    fn handle_tree_key_event(&mut self, event: Event) -> std::io::Result<()> {
        let update_tree = match event {
            Event::Key(key) if !matches!(key.kind, KeyEventKind::Press) => false,
            Event::Key(key) => match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('\n' | ' ') => self.state.toggle_selected(),
                KeyCode::Left => self.state.key_left(),
                KeyCode::Right => self.state.key_right(),
                KeyCode::Down => self.state.key_down(),
                KeyCode::Up => self.state.key_up(),
                KeyCode::Esc => self.state.select(Vec::new()),
                KeyCode::Home => self.state.select_first(),
                KeyCode::End => self.state.select_last(),
                KeyCode::PageDown => self.state.scroll_down(3),
                KeyCode::PageUp => self.state.scroll_up(3),
                _ => false,
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => self.state.scroll_down(1),
                MouseEventKind::ScrollUp => self.state.scroll_up(1),
                MouseEventKind::Down(_button) => {
                    self.state.click_at(Position::new(mouse.column, mouse.row))
                }
                _ => false,
            },
            Event::Resize(_, _) => true,
            _ => false,
        };

        self.project_tree_do_refresh = update_tree;
        Ok(())
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

    fn handle_prompting_key(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        match self.current_focus_area {
            FocusedInputArea::ProjectTree => {
                // Handle tree navigation
                match key.code {
                    KeyCode::Up => { self.state.key_up(); }
                    KeyCode::Down => { self.state.key_down(); }
                    KeyCode::Left => { self.state.key_left(); }
                    KeyCode::Right => { self.state.key_right(); }
                    _ => {}
                }
            }
            FocusedInputArea::Question => {
                let _ = self.question_text_widget.input(key);
            }
            FocusedInputArea::Answer => {
                match key.code {
                    KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End => {
                        let _ = self.answer_text_widget.input(key);
                    }
                    _ => {}
                }
            }
        }
    }
    fn handle_key_event(&mut self, key: ratatui::crossterm::event::KeyEvent) -> color_eyre::Result<Option<()>> {
        match key.code {
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
                else {
                    self.handle_prompting_key(key)
                }
            }
        }
        Ok(None) // Continue running
    }

    fn handle_mouse_event(&mut self, mouse_event: ratatui::crossterm::event::MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let is_click = mouse_event.kind == MouseEventKind::Down(MouseButton::Left);
                self.focus_at_mouse_pos(mouse_event.column, mouse_event.row, is_click);
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
