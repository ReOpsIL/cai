use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::time::Instant;
use crate::openrouter_client::{OpenRouterClient, ChatMessage};
use crate::prompt_loader::{PromptManager, Prompt};
use crate::logger::{log_info, log_debug, log_warn, log_error, ops};
use crate::task_executor::{TaskExecutor, TaskStatus};

pub struct ChatInterface {
    openrouter_client: OpenRouterClient,
    conversation_history: Vec<ChatMessage>,
    task_executor: TaskExecutor,
}

impl ChatInterface {
    pub async fn new() -> Result<Self> {
        log_info!("chat", "💬 Initializing ChatInterface with LLM-powered task analysis");
        let client_start = Instant::now();
        
        let openrouter_client = OpenRouterClient::new().await
            .context("Failed to initialize OpenRouter client")?;
        
        // Try to create LLM-enabled task executor, fallback to basic if it fails
        let task_executor = match TaskExecutor::with_llm_analysis().await {
            Ok(executor) => {
                log_info!("chat", "✅ Initialized task executor with LLM analysis");
                executor
            }
            Err(e) => {
                log_warn!("chat", "⚠️ Failed to initialize LLM task analysis, using heuristic fallback: {}", e);
                TaskExecutor::new()
            }
        };
        
        let init_duration = client_start.elapsed().as_millis() as u64;
        ops::performance("CHAT_INIT", init_duration);
        log_info!("chat", "✅ ChatInterface initialized successfully");
        
        Ok(Self {
            openrouter_client,
            conversation_history: Vec::new(),
            task_executor,
        })
    }

    pub async fn start_chat(&mut self, manager: &mut PromptManager) -> Result<()> {
        log_info!("chat", "🚀 Starting chat session");
        ops::startup("CHAT", "interactive chat session");
        
        println!("{}", "🤖 CAI Chat Interface with Task Execution".bright_blue().bold());
        println!("{}", "I'll help you plan tasks, manage prompts, and execute tasks using MCP tools.".dimmed());
        println!("{}", "Special commands: 'status', 'execute', 'clear', 'help', 'quit'".dimmed());
        println!();

        loop {
            print!("{} ", "You:".bright_green().bold());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                log_debug!("chat", "⏭️ Empty input, continuing");
                continue;
            }

            if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                log_info!("chat", "👋 User exiting chat session");
                ops::shutdown("CHAT", "user requested exit");
                println!("{}", "Goodbye! 👋".bright_blue());
                break;
            }

            // Handle special commands
            if self.handle_special_commands(input).await? {
                continue; // Command was handled, continue to next input
            }

            log_info!("chat", "📥 Processing user input: '{}'", input);
            if let Err(e) = self.process_user_input(input, manager).await {
                ops::error_with_context("CHAT_INPUT", &e.to_string(), Some(input));
                println!("{} Error: {}", "❌".red(), e);
            }
        }

        log_info!("chat", "✅ Chat session ended successfully");
        Ok(())
    }

    /// Handle special chat interface commands
    async fn handle_special_commands(&mut self, input: &str) -> Result<bool> {
        let input_lower = input.trim().to_lowercase();
        
        match input_lower.as_str() {
            "status" | "queue" => {
                self.task_executor.display_queue_status().await;
                Ok(true)
            }
            "execute" | "run" => {
                match self.task_executor.execute_all().await {
                    Ok(_) => {
                        println!("{} All queued tasks completed!", "🎉".green());
                    }
                    Err(e) => {
                        println!("{} Task execution error: {}", "❌".red(), e);
                    }
                }
                Ok(true)
            }
            "clear" | "clean" => {
                let cleared = self.task_executor.clear_completed_tasks().await;
                if cleared == 0 {
                    println!("{} No completed tasks to clear", "💭".dimmed());
                }
                Ok(true)
            }
            "help" => {
                self.show_help();
                Ok(true)
            }
            _ => Ok(false) // Not a special command
        }
    }

    fn show_help(&self) {
        println!("\n{} Available chat commands:", "💡".bright_yellow().bold());
        println!("  {} - Show current task queue status", "status".cyan());
        println!("  {} - Execute all queued tasks", "execute".cyan());
        println!("  {} - Clear completed tasks from queue", "clear".cyan());
        println!("  {} - Show this help message", "help".cyan());
        println!("  {} - Exit chat mode", "quit".cyan());
        println!();
    }

    async fn process_user_input(&mut self, user_input: &str, manager: &mut PromptManager) -> Result<()> {
        let process_start = Instant::now();
        log_debug!("chat", "🔄 Starting task planning for user input");
        println!("{} Planning tasks...", "🔄".yellow());

        // Get task plan from LLM
        let planning_start = Instant::now();
        let tasks = self.openrouter_client.plan_tasks(user_input).await
            .context("Failed to generate task plan")?;
        let planning_duration = planning_start.elapsed().as_millis() as u64;
        ops::performance("TASK_PLANNING", planning_duration);
        ops::chat_operation("PLAN_TASKS", &format!("Generated {} tasks from: {}", tasks.len(), user_input));

        if tasks.is_empty() {
            log_warn!("chat", "⚠️ No tasks generated from user input");
            ops::chat_operation("PLAN_TASKS", &format!("No tasks generated from: {}", user_input));
            println!("{} No tasks generated from your request.", "⚠️".yellow());
            return Ok(());
        }

        log_info!("chat", "📋 Generated {} task(s) from user input", tasks.len());
        println!("\n{} Generated {} task(s):", "📋".cyan(), tasks.len());
        for (i, task) in tasks.iter().enumerate() {
            log_debug!("chat", "📝 Task {}: {}", i + 1, task);
            println!("{}. {}", i + 1, task.dimmed());
        }
        println!();

        // Add tasks to the execution queue
        self.task_executor.add_tasks(tasks.clone()).await?;

        // Execute tasks automatically (for now, to avoid hanging)
        println!("{} Executing tasks automatically...", "⚡".yellow());
        match self.task_executor.execute_all().await {
            Ok(_) => {
                println!("\n{} All tasks completed! Continuing chat...", "🎉".green());
            }
            Err(e) => {
                println!("\n{} Task execution error: {}", "❌".red(), e);
                log_warn!("chat", "Task execution failed: {}", e);
            }
        }

        // Process tasks for prompt management (existing functionality)
        let mut new_prompts_added = 0;
        let mut prompts_updated = 0;
        let mut prompts_scored = 0;

        log_debug!("chat", "🔄 Processing {} tasks for prompt management", tasks.len());
        for (i, task) in tasks.iter().enumerate() {
            log_debug!("chat", "⚙️ Processing task {}/{} for prompts: {}", i + 1, tasks.len(), task);
            let task_start = Instant::now();
            
            match self.process_task(&task, manager).await? {
                TaskProcessingResult::NewPromptAdded => {
                    new_prompts_added += 1;
                    ops::chat_operation("ADD_PROMPT", task);
                }
                TaskProcessingResult::PromptUpdated => {
                    prompts_updated += 1;
                    ops::chat_operation("UPDATE_PROMPT", task);
                }
                TaskProcessingResult::PromptScored => {
                    prompts_scored += 1;
                    ops::chat_operation("SCORE_PROMPT", task);
                }
            }
            
            let task_duration = task_start.elapsed().as_millis() as u64;
            ops::performance("TASK_PROCESSING", task_duration);
        }

        // Summary
        let total_duration = process_start.elapsed().as_millis() as u64;
        ops::performance("USER_INPUT_PROCESSING", total_duration);
        
        log_info!("chat", "✅ Task processing complete: {} added, {} updated, {} scored", 
            new_prompts_added, prompts_updated, prompts_scored);
        
        if new_prompts_added > 0 || prompts_updated > 0 || prompts_scored > 0 {
            println!("{} Prompt management complete:", "✅".green());
            if new_prompts_added > 0 {
                println!("  📝 {} new prompt(s) added", new_prompts_added);
            }
            if prompts_updated > 0 {
                println!("  🔄 {} prompt(s) updated", prompts_updated);
            }
            if prompts_scored > 0 {
                println!("  ⭐ {} prompt(s) scored", prompts_scored);
            }
            println!();
        }

        Ok(())
    }

    async fn process_task(&self, task: &str, manager: &mut PromptManager) -> Result<TaskProcessingResult> {
        log_debug!("chat", "🔍 Finding similar prompts for task: {}", task);
        
        // Find similar prompts (threshold: 0.5 for similarity detection)
        let similarity_start = Instant::now();
        let similar_prompts = manager.find_similar_prompts(task, 0.5).await;
        let similarity_duration = similarity_start.elapsed().as_millis() as u64;
        ops::performance("SIMILARITY_SEARCH", similarity_duration);
        
        log_debug!("chat", "📊 Found {} similar prompt(s)", similar_prompts.len());

        if similar_prompts.is_empty() {
            log_debug!("chat", "➕ No similar prompts found, adding new prompt");
            // No similar prompts found - add as new prompt
            self.add_new_prompt(task, manager).await?;
            Ok(TaskProcessingResult::NewPromptAdded)
        } else {
            let best_match = &similar_prompts[0];
            log_debug!("chat", "🎯 Best match: '{}' with similarity {:.3}", 
                best_match.prompt.title, best_match.similarity_score);
            
            if best_match.similarity_score >= 0.8 {
                log_debug!("chat", "⭐ High similarity (>= 0.8), scoring existing prompt");
                // Very similar prompt exists - increment score
                manager.increment_prompt_score(
                    &best_match.file_name,
                    &best_match.subject_name,
                    &best_match.prompt.id,
                )?;
                println!("  ⭐ Scored existing prompt: '{}'", best_match.prompt.title.cyan());
                Ok(TaskProcessingResult::PromptScored)
            } else if best_match.similarity_score >= 0.6 {
                log_debug!("chat", "🔄 Medium similarity (>= 0.6), updating existing prompt");
                // Similar but could be improved - update existing prompt
                let improve_start = Instant::now();
                let improved_content = self.openrouter_client.improve_prompt(
                    &best_match.prompt.get_resolved_content().await.unwrap_or(best_match.prompt.content.clone()),
                    task,
                ).await?;
                let improve_duration = improve_start.elapsed().as_millis() as u64;
                ops::performance("PROMPT_IMPROVEMENT", improve_duration);
                
                manager.update_prompt(
                    &best_match.file_name,
                    &best_match.subject_name,
                    &best_match.prompt.id,
                    improved_content,
                )?;
                println!("  🔄 Updated existing prompt: '{}'", best_match.prompt.title.cyan());
                Ok(TaskProcessingResult::PromptUpdated)
            } else {
                log_debug!("chat", "➕ Low similarity (< 0.6), adding new prompt");
                // Different enough to be a new prompt
                self.add_new_prompt(task, manager).await?;
                Ok(TaskProcessingResult::NewPromptAdded)
            }
        }
    }

    async fn add_new_prompt(&self, task: &str, manager: &mut PromptManager) -> Result<()> {
        log_debug!("chat", "➕ Adding new prompt for task: {}", task);
        
        let ai_file = manager.get_or_create_ai_generated_file()?;
        
        // Determine subject based on task content (simple heuristic)
        let subject_name = self.categorize_task(task);
        log_debug!("chat", "📂 Categorized task as: {}", subject_name);
        
        // Create title from first few words of task
        let title = task.split_whitespace()
            .take(6)
            .collect::<Vec<_>>()
            .join(" ");
        
        let new_prompt = Prompt {
            title: if title.len() > 50 { 
                format!("{}...", &title[..47])
            } else { 
                title 
            },
            content: task.to_string(),
            score: 0,
            id: uuid::Uuid::new_v4().to_string(),
        };

        manager.add_prompt_to_subject(&ai_file, &subject_name, new_prompt.clone())?;
        log_info!("chat", "✅ Added new prompt '{}' to subject '{}'", new_prompt.title, subject_name);
        println!("  📝 Added new prompt: '{}'", new_prompt.title.cyan());
        
        Ok(())
    }

    fn categorize_task(&self, task: &str) -> String {
        let task_lower = task.to_lowercase();
        log_debug!("chat", "🏷️ Categorizing task: {}", task);
        
        let category = if task_lower.contains("bug") || task_lower.contains("debug") || task_lower.contains("error") || task_lower.contains("fix") {
            "Bug Fixing".to_string()
        } else if task_lower.contains("test") || task_lower.contains("plan") || task_lower.contains("document") {
            "Task Creation".to_string()
        } else if task_lower.contains("analyze") || task_lower.contains("review") || task_lower.contains("audit") {
            "Code Analysis".to_string()
        } else if task_lower.contains("refactor") || task_lower.contains("improve") || task_lower.contains("optimize") {
            "Refactoring".to_string()
        } else {
            "General Tasks".to_string()
        };
        
        log_debug!("chat", "🏷️ Task categorized as: {}", category);
        category
    }
}

#[derive(Debug)]
enum TaskProcessingResult {
    NewPromptAdded,
    PromptUpdated,
    PromptScored,
}