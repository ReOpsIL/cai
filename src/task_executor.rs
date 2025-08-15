use anyhow::{anyhow, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::logger::{log_debug, log_error, log_info, log_warn, ops};
use crate::mcp_manager;
use crate::openrouter_client::{OpenRouterClient, ToolMetadata};
use crate::validator::ValidatorsRunner;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Waiting,
    Running,
    Done,
    Failed,
}

impl TaskStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Waiting => "⏳",
            TaskStatus::Running => "🔄",
            TaskStatus::Done => "✅",
            TaskStatus::Failed => "❌",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TaskStatus::Waiting => "Waiting",
            TaskStatus::Running => "Running",
            TaskStatus::Done => "Done",
            TaskStatus::Failed => "Failed",
        }
    }

    pub fn colored_description(&self) -> colored::ColoredString {
        match self {
            TaskStatus::Waiting => self.description().yellow(),
            TaskStatus::Running => self.description().blue(),
            TaskStatus::Done => self.description().green(),
            TaskStatus::Failed => self.description().red(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub mcp_tool_calls: Vec<McpToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub server_name: String,
    pub tool_name: String,
    pub arguments: Value,
    pub result: Option<Value>,
}

impl Task {
    pub fn new(description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description,
            status: TaskStatus::Waiting,
            result: None,
            error: None,
            mcp_tool_calls: Vec::new(),
        }
    }

    pub fn display_summary(&self) -> String {
        format!("{} {} {}", 
                self.status.icon(), 
                self.status.colored_description(),
                self.description.dimmed())
    }
}

pub struct TaskExecutor {
    queue: Arc<Mutex<VecDeque<Task>>>,
    is_running: Arc<Mutex<bool>>,
    openrouter_client: Option<OpenRouterClient>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            is_running: Arc::new(Mutex::new(false)),
            openrouter_client: None,
        }
    }

    pub async fn with_llm_analysis() -> Result<Self> {
        let client = OpenRouterClient::new().await?;
        Ok(Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            is_running: Arc::new(Mutex::new(false)),
            openrouter_client: Some(client),
        })
    }

    /// Add tasks to the execution queue
    pub async fn add_tasks(&self, task_descriptions: Vec<String>) -> Result<()> {
        let queue_size = {
            let mut queue = self.queue.lock().await;
            
            log_info!("task_executor", "📝 Adding {} task(s) to execution queue", task_descriptions.len());
            
            for description in task_descriptions {
                let task = Task::new(description.clone());
                log_debug!("task_executor", "➕ Added task: {}", description);
                queue.push_back(task);
            }

            queue.len()
        }; // Release the lock here

        println!("\n{} Added {} task(s) to execution queue", "📋".cyan(), queue_size);
        log_debug!("task_executor", "📋 About to display queue status");
        self.display_queue_status().await;
        log_debug!("task_executor", "📋 Queue status displayed");
        
        Ok(())
    }

    /// Display current queue status
    pub async fn display_queue_status(&self) {
        log_debug!("task_executor", "🔍 Attempting to lock queue for status display");
        let queue = self.queue.lock().await;
        log_debug!("task_executor", "🔓 Queue locked, checking if empty");
        
        if queue.is_empty() {
            println!("{} Task queue is empty", "📭".dimmed());
            log_debug!("task_executor", "📭 Queue is empty, returning");
            return;
        }

        log_debug!("task_executor", "📊 Queue has {} tasks, displaying status", queue.len());
        println!("\n{} Task Queue Status:", "📊".bright_blue().bold());
        
        let waiting_count = queue.iter().filter(|t| t.status == TaskStatus::Waiting).count();
        let running_count = queue.iter().filter(|t| t.status == TaskStatus::Running).count();
        let done_count = queue.iter().filter(|t| t.status == TaskStatus::Done).count();
        let failed_count = queue.iter().filter(|t| t.status == TaskStatus::Failed).count();

        log_debug!("task_executor", "📈 Counts: waiting={}, running={}, done={}, failed={}", 
                  waiting_count, running_count, done_count, failed_count);

        println!("  {} {} waiting • {} {} running • {} {} done • {} {} failed",
                 TaskStatus::Waiting.icon(), waiting_count,
                 TaskStatus::Running.icon(), running_count, 
                 TaskStatus::Done.icon(), done_count,
                 TaskStatus::Failed.icon(), failed_count);

        println!();
        log_debug!("task_executor", "📝 About to display individual tasks");
        for (index, task) in queue.iter().enumerate() {
            log_debug!("task_executor", "📝 Displaying task {}: {}", index + 1, task.description);
            println!("  {}. {}", index + 1, task.display_summary());
        }
        println!();
        log_debug!("task_executor", "✅ Queue status display completed");
    }

    /// Execute all tasks in the queue
    pub async fn execute_all(&self) -> Result<()> {
        {
            let mut is_running = self.is_running.lock().await;
            if *is_running {
                return Err(anyhow!("Task execution is already in progress"));
            }
            *is_running = true;
        }

        log_info!("task_executor", "🚀 Starting task execution");
        println!("{} Starting task execution...", "🚀".green().bold());

        let result = self.execute_tasks_internal().await;

        {
            let mut is_running = self.is_running.lock().await;
            *is_running = false;
        }

        // Optionally validate after executing all tasks
        if result.is_ok() {
            let validators = ValidatorsRunner::from_env();
            if validators.when() == crate::validator::ValidateWhen::AfterAll {
                let _ = validators.maybe_validate_project().await;
            }
        }

        result
    }

    async fn execute_tasks_internal(&self) -> Result<()> {
        loop {
            let next_task = {
                let mut queue = self.queue.lock().await;
                queue.iter_mut()
                    .find(|task| task.status == TaskStatus::Waiting)
                    .map(|task| {
                        task.status = TaskStatus::Running;
                        task.clone()
                    })
            };

            match next_task {
                Some(mut task) => {
                    println!("\n{} Executing: {}", "🔄".blue(), task.description.bright_white());
                    log_info!("task_executor", "🔄 Executing task: {}", task.description);
                    
                    match self.execute_single_task(&mut task).await {
                        Ok(_) => {
                            task.status = TaskStatus::Done;
                            println!("{} Completed: {}", "✅".green(), task.description);
                            
                            // Display the result if available
                            if let Some(ref result) = task.result {
                                println!("\n{} Result:", "📋".bright_cyan());
                                println!("{}", result);
                                println!(); // Add spacing
                            }
                            
                            log_info!("task_executor", "✅ Task completed: {}", task.description);
                        }
                        Err(e) => {
                            task.status = TaskStatus::Failed;
                            task.error = Some(e.to_string());
                            println!("{} Failed: {} - {}", "❌".red(), task.description, e);
                            log_warn!("task_executor", "❌ Task failed: {} - {}", task.description, e);
                        }
                    }

                    // Update task in queue
                    {
                        let mut queue = self.queue.lock().await;
                        if let Some(queue_task) = queue.iter_mut().find(|t| t.id == task.id) {
                            *queue_task = task;
                        }
                    }
                }
                None => {
                    // No more waiting tasks
                    break;
                }
            }
        }

        println!("\n{} All tasks completed!", "🎉".green().bold());
        self.display_queue_status().await;
        log_info!("task_executor", "🎉 All tasks completed");

        Ok(())
    }

    async fn execute_single_task(&self, task: &mut Task) -> Result<()> {
        log_debug!("task_executor", "🔄 Starting single task execution for: {}", task.description);
        
        // Try to determine what MCP tools this task might need
        let tool_suggestions = self.analyze_task_for_tools(&task.description).await?;
        
        if tool_suggestions.is_empty() {
            // No specific MCP tools identified, provide helpful error message
            task.result = Some("No MCP tools were identified for this task. Please check MCP server configuration.".to_string());
            println!("    {} No MCP tools available for task: {}", "⚠️".yellow(), task.description);
            println!("    {} Consider checking your MCP server configuration in mcp-config.json", "💡".yellow());
            log_debug!("task_executor", "⚠️ No MCP tools available for task execution");
            return Ok(());
        }

        // Execute suggested tool calls
        for suggestion in tool_suggestions {
            log_debug!("task_executor", "🔧 Executing MCP tool: {} on server {}", 
                      suggestion.tool_name, suggestion.server_name);
            
            match self.execute_mcp_tool_call(&suggestion).await {
                Ok(result) => {
                    // Print the MCP tool result for debugging
                    println!("    {} {}: {}", 
                             "📋".cyan(), 
                             suggestion.tool_name.bright_white(),
                             serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()));
                    
                    let mut tool_call = suggestion.clone();
                    tool_call.result = Some(result);
                    task.mcp_tool_calls.push(tool_call);
                    log_debug!("task_executor", "✅ MCP tool call successful");
                }
                Err(e) => {
                    println!("    {} {}: Failed - {}", 
                             "❌".red(), 
                             suggestion.tool_name.bright_white(), 
                             e);
                    log_warn!("task_executor", "⚠️ MCP tool call failed: {}", e);
                    // Continue with other tools, don't fail the entire task
                }
            }
        }

        if task.mcp_tool_calls.is_empty() {
            task.result = Some("Task completed but no MCP tools were successfully executed.".to_string());
        } else {
            task.result = Some(format!("Task completed with {} MCP tool call(s)", task.mcp_tool_calls.len()));
        }

        log_debug!("task_executor", "✅ Single task execution completed");
        Ok(())
    }

    async fn analyze_task_for_tools(&self, task_description: &str) -> Result<Vec<McpToolCall>> {
        log_debug!("task_executor", "🔍 Analyzing task for tools: '{}'", task_description);

        // If LLM client is available, use intelligent analysis
        if let Some(ref client) = self.openrouter_client {
            log_debug!("task_executor", "🧠 Using LLM-based tool analysis");
            
            // Try LLM analysis with timeout and fallback
            match tokio::time::timeout(
                std::time::Duration::from_secs(15),
                self.llm_analyze_task_for_tools(client, task_description)
            ).await {
                Ok(Ok(result)) => {
                    log_info!("task_executor", "✅ LLM analysis successful with {} tools", result.len());
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    log_warn!("task_executor", "⚠️ LLM analysis failed: {}", e);
                    println!("    {} LLM analysis failed: {}", "⚠️".yellow(), e);
                    println!("    {} Falling back to heuristic analysis", "💡".yellow());
                }
                Err(_) => {
                    log_warn!("task_executor", "⏰ LLM analysis timed out");
                    println!("    {} LLM analysis timed out", "⏰".yellow());
                    println!("    {} Falling back to heuristic analysis", "💡".yellow());
                }
            }
        } else {
            log_info!("task_executor", "💡 LLM client not available, using heuristic analysis");
            println!("    {} LLM not available, using heuristic analysis", "💡".yellow());
            println!("    {} Set OPENROUTER_API_KEY for intelligent tool selection", "💡".cyan());
        }

        // Use heuristic fallback
        log_debug!("task_executor", "🧮 Using heuristic-based tool analysis");
        self.heuristic_analyze_task_for_tools(task_description).await
    }

    async fn llm_analyze_task_for_tools(&self, client: &OpenRouterClient, task_description: &str) -> Result<Vec<McpToolCall>> {
        // Collect available tools metadata
        let tool_metadata = self.collect_tool_metadata().await?;
        
        if tool_metadata.is_empty() {
            log_error!("task_executor", "❌ No MCP tools available for analysis");
            println!("    {} No MCP tools available for LLM analysis", "❌".red());
            println!("    {} MCP servers may not be running or configured (check mcp-config.json)", "💡".yellow());
            return Err(anyhow!("MCP tools are required but not available"));
        }

        log_debug!("task_executor", "🔧 Collected metadata for {} tools", tool_metadata.len());

        // Use LLM to analyze and select tools
        let tool_selections = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            client.analyze_task_for_tools(task_description, &tool_metadata)
        ).await
        .map_err(|_| anyhow!("LLM tool analysis timed out"))?
        .map_err(|e| anyhow!("LLM tool analysis failed: {}", e))?;

        log_info!("task_executor", "🎯 LLM selected {} tools for task", tool_selections.len());

        // Convert tool selections to MCP tool calls
        let mut mcp_calls = Vec::new();
        for selection in tool_selections {
            // Find the server for this tool
            if let Some((server_name, _)) = self.find_tool_server(&selection.tool_name).await? {
                let mcp_call = McpToolCall {
                    server_name,
                    tool_name: selection.tool_name.clone(),
                    arguments: serde_json::to_value(selection.parameters)?,
                    result: None,
                };
                mcp_calls.push(mcp_call);
                log_debug!("task_executor", "✅ Added MCP call for tool '{}': {}", 
                          selection.tool_name, selection.rationale);
            } else {
                log_warn!("task_executor", "⚠️ Could not find server for tool '{}'", selection.tool_name);
            }
        }

        Ok(mcp_calls)
    }

    async fn collect_tool_metadata(&self) -> Result<Vec<ToolMetadata>> {
        let mut metadata = Vec::new();
        
        // First, collect local tools
        log_debug!("task_executor", "🏠 Collecting local tools");
        let local_tools = crate::local_tools::list_local_tools();
        for tool_name in local_tools {
            let tool_metadata = ToolMetadata {
                name: tool_name.clone(),
                description: self.get_tool_description(&tool_name),
                parameters: self.get_tool_parameters(&tool_name),
            };
            metadata.push(tool_metadata);
        }
        log_debug!("task_executor", "🔧 Added {} local tools", metadata.len());
        
        // Then, collect MCP tools
        let global_manager = mcp_manager::get_mcp_manager();
        
        let analysis_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            async {
                let guard = global_manager.lock().await;
                let Some(manager) = guard.as_ref() else {
                    // No MCP configured; return local tools only
                    return Ok::<Vec<ToolMetadata>, anyhow::Error>(metadata);
                };

                let active_servers = manager.list_active_servers().await;
                log_debug!("task_executor", "📡 Collecting tools from {} active MCP servers", active_servers.len());

                for server_name in active_servers {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(3),
                        manager.list_tools(&server_name)
                    ).await {
                        Ok(Ok(tools)) => {
                            log_debug!("task_executor", "🔧 Server '{}' has {} tools", server_name, tools.len());
                            for tool_name in tools {
                                let tool_metadata = ToolMetadata {
                                    name: tool_name.clone(),
                                    description: self.get_tool_description(&tool_name),
                                    parameters: self.get_tool_parameters(&tool_name),
                                };
                                metadata.push(tool_metadata);
                            }
                        }
                        Ok(Err(e)) => {
                            log_debug!("task_executor", "⚠️ Could not get tools for server {}: {}", server_name, e);
                        }
                        Err(_) => {
                            log_debug!("task_executor", "⏰ Timeout getting tools for server {}", server_name);
                        }
                    }
                }

                Ok::<Vec<ToolMetadata>, anyhow::Error>(metadata)
            }
        ).await;

        match analysis_result {
            Ok(result) => result,
            Err(_) => {
                log_warn!("task_executor", "⏰ Tool metadata collection timed out");
                Ok(Vec::new())
            }
        }
    }

    async fn find_tool_server(&self, tool_name: &str) -> Result<Option<(String, Vec<String>)>> {
        // First check local tools
        let local_tools = crate::local_tools::list_local_tools();
        if local_tools.contains(&tool_name.to_string()) {
            return Ok(Some((crate::local_tools::LOCAL_SERVER_NAME.to_string(), local_tools)));
        }
        
        // Then check MCP servers
        let global_manager = mcp_manager::get_mcp_manager();
        let guard = global_manager.lock().await;
        let Some(manager) = guard.as_ref() else {
            return Ok(None);
        };

        let active_servers = manager.list_active_servers().await;
        
        for server_name in active_servers {
            if let Ok(tools) = manager.list_tools(&server_name).await {
                if tools.contains(&tool_name.to_string()) {
                    return Ok(Some((server_name, tools)));
                }
            }
        }
        
        Ok(None)
    }

    fn get_tool_description(&self, tool_name: &str) -> String {
        match tool_name {
            // File operations
            "list_directory" => "List files and directories in a specified path with detailed information".to_string(),
            "read_file" => "Read the contents of a file with optional line range support".to_string(),
            "write_file" => "Write content to a file, creating directories if needed".to_string(),
            "edit_file" => "Edit file content by replacing old text with new text, with validation".to_string(),
            "delete_path" => "Delete files or directories with optional recursive deletion".to_string(),
            
            // Search operations
            "search_files" => "Search for text patterns in files with regex, fuzzy search, and context".to_string(),
            "glob_files" => "Find files using glob patterns with advanced filtering".to_string(),
            
            // Command execution
            "execute_command" => "Execute shell commands with timeout, safety checks, and output capture".to_string(),
            
            // Web operations
            "web_fetch" => "Fetch content from URLs with timeout and size limits".to_string(),
            "download_file" => "Download files from URLs to local filesystem".to_string(),
            
            // Task management
            "create_tasks" => "Create structured task lists for workflow management".to_string(),
            "update_tasks" => "Update task status and add notes to existing tasks".to_string(),
            
            // Multi-file operations
            "multiedit_file" => "Apply multiple atomic edits to a single file in one operation".to_string(),
            
            _ => format!("MCP tool: {}", tool_name),
        }
    }

    fn get_tool_parameters(&self, tool_name: &str) -> Vec<String> {
        match tool_name {
            // File operations
            "list_directory" => vec!["path".to_string(), "recursive".to_string(), "show_hidden".to_string()],
            "read_file" => vec!["path".to_string(), "start_line".to_string(), "end_line".to_string()],
            "write_file" => vec!["path".to_string(), "content".to_string()],
            "edit_file" => vec!["path".to_string(), "old_text".to_string(), "new_text".to_string(), "replace_all".to_string()],
            "delete_path" => vec!["path".to_string(), "recursive".to_string()],
            
            // Search operations
            "search_files" => vec!["pattern".to_string(), "directory".to_string(), "pattern_type".to_string(), "file_pattern".to_string()],
            "glob_files" => vec!["pattern".to_string(), "directory".to_string(), "max_results".to_string()],
            
            // Command execution
            "execute_command" => vec!["command".to_string(), "working_directory".to_string(), "timeout".to_string(), "command_type".to_string()],
            
            // Web operations
            "web_fetch" => vec!["url".to_string(), "timeout".to_string()],
            "download_file" => vec!["url".to_string(), "file_path".to_string(), "timeout".to_string()],
            
            // Task management
            "create_tasks" => vec!["user_query".to_string(), "tasks".to_string()],
            "update_tasks" => vec!["task_updates".to_string()],
            
            // Multi-file operations
            "multiedit_file" => vec!["file_path".to_string(), "edits".to_string()],
            
            _ => vec!["args".to_string()],
        }
    }

    /// Heuristic-based task analysis as fallback when LLM is unavailable
    async fn heuristic_analyze_task_for_tools(&self, task_description: &str) -> Result<Vec<McpToolCall>> {
        let mut suggestions = Vec::new();
        let task_lower = task_description.to_lowercase();

        // Get available MCP servers and their tools with timeout
        let global_manager = mcp_manager::get_mcp_manager();
        
        // Add timeout to prevent hanging
        let analysis_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            async {
                let guard = global_manager.lock().await;
                let Some(manager) = guard.as_ref() else {
                    println!("    {} MCP manager is not initialized", "⚠️".yellow());
                    println!("    {} Check if MCP servers are properly configured in mcp-config.json", "💡".yellow());
                    return Ok::<Vec<McpToolCall>, anyhow::Error>(Vec::new());
                };

                let active_servers = manager.list_active_servers().await;
                log_debug!("task_executor", "📡 Found {} active servers", active_servers.len());
                
                if active_servers.is_empty() {
                    println!("    {} No active MCP servers found", "⚠️".yellow());
                    println!("    {} MCP servers may not be started or configured properly", "💡".yellow());
                    println!("    {} Try running: cargo run -- mcp list", "💡".yellow());
                    return Ok(Vec::new());
                }

                for server_name in active_servers {
                    // Get tools for this server with individual timeout
                    let tools_result = tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        manager.list_tools(&server_name)
                    ).await;

                    match tools_result {
                        Ok(Ok(tools)) => {
                            log_debug!("task_executor", "🔧 Server '{}' has {} tools", server_name, tools.len());
                            for tool_name in tools {
                                // Simple heuristic-based tool matching
                                let should_use_tool = match tool_name.as_str() {
                                    "list_directory" if task_lower.contains("list") || 
                                                       task_lower.contains("directory") ||
                                                       task_lower.contains("structure") ||
                                                       task_lower.contains("files") => true,
                                    "read_file" if (task_lower.contains("read") || 
                                                   task_lower.contains("show") ||
                                                   task_lower.contains("content")) &&
                                                   (task_lower.contains("file") || 
                                                    task_lower.contains("readme") ||
                                                    task_lower.contains(".md") ||
                                                    task_lower.contains(".txt")) => true,
                                    "write_file" if task_lower.contains("write") || 
                                                   task_lower.contains("create") || 
                                                   task_lower.contains("save") => true,
                                    _ => false,
                                };

                                if should_use_tool {
                                    let arguments = self.generate_tool_arguments(&tool_name, task_description);
                                    log_debug!("task_executor", "✅ Matched tool '{}' for task", tool_name);
                                    suggestions.push(McpToolCall {
                                        server_name: server_name.clone(),
                                        tool_name: tool_name.clone(),
                                        arguments,
                                        result: None,
                                    });
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            log_debug!("task_executor", "⚠️ Could not get tools for server {}: {}", server_name, e);
                        }
                        Err(_) => {
                            log_debug!("task_executor", "⏰ Timeout getting tools for server {}", server_name);
                        }
                    }
                }

                Ok::<Vec<McpToolCall>, anyhow::Error>(suggestions)
            }
        ).await;

        match analysis_result {
            Ok(result) => {
                let final_suggestions = result?;
                log_debug!("task_executor", "🎯 Found {} tool suggestions for task", final_suggestions.len());
                Ok(final_suggestions)
            }
            Err(_) => {
                log_warn!("task_executor", "⏰ Task analysis timed out, proceeding without MCP tools");
                Ok(Vec::new())
            }
        }
    }

    /// Generate default tool arguments when LLM analysis is unavailable
    fn generate_tool_arguments(&self, tool_name: &str, task_description: &str) -> Value {
        // Generate reasonable default arguments based on tool type and task description
        match tool_name {
            "list_directory" => {
                serde_json::json!({
                    "path": "/project"
                })
            }
            "read_file" => {
                // Try to extract file path from task description
                serde_json::json!({
                    "path": "/project/README.md"  // Default, could be improved with NLP
                })
            }
            "write_file" => {
                serde_json::json!({
                    "path": "/project/output.txt",
                    "content": format!("Task execution result: {}", task_description)
                })
            }
            _ => serde_json::Value::Object(serde_json::Map::new())
        }
    }

    async fn execute_mcp_tool_call(&self, tool_call: &McpToolCall) -> Result<Value> {
        log_debug!("task_executor", "🔧 Calling tool: {} on server {} with args: {}", 
                  tool_call.tool_name, tool_call.server_name, tool_call.arguments);
        
        ops::mcp_operation("TOOL_CALL", &format!("{}:{}", tool_call.server_name, tool_call.tool_name));
        
        // SAFETY: Check tool permissions before execution
        let safety_validator = crate::tool_safety::get_global_safety_validator();
        if let Err(safety_error) = safety_validator.check_tool_permission(&tool_call.tool_name, &tool_call.arguments).await {
            log_warn!("task_executor", "🛡️ Tool execution blocked by safety validator: {}", safety_error);
            return Err(anyhow!("Tool execution blocked by safety policy: {}", safety_error));
        }
        
        // Check if this is a local tool
        if tool_call.server_name == crate::local_tools::LOCAL_SERVER_NAME {
            log_debug!("task_executor", "🏠 Executing local tool: {}", tool_call.tool_name);
            return crate::local_tools::execute_local_tool(&tool_call.tool_name, tool_call.arguments.clone())
                .map_err(|e| anyhow!("Local tool execution failed: {}", e));
        }
        
        // Execute MCP tool with timeout
        let call_result = tokio::time::timeout(
            std::time::Duration::from_secs(30), // Increased timeout for complex operations
            async {
                let global_manager = mcp_manager::get_mcp_manager();
                let guard = global_manager.lock().await;
                let manager = guard.as_ref()
                    .ok_or_else(|| anyhow!("MCP manager not available"))?;

                manager.call_tool(
                    &tool_call.server_name,
                    &tool_call.tool_name,
                    tool_call.arguments.clone()
                ).await
            }
        ).await;

        match call_result {
            Ok(result) => {
                log_debug!("task_executor", "✅ MCP tool call completed successfully");
                result
            }
            Err(_) => {
                log_warn!("task_executor", "⏰ MCP tool call timed out after 30 seconds");
                Err(anyhow!("MCP tool call timed out"))
            }
        }
    }

    /// Check if all tasks are completed
    pub async fn all_tasks_completed(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.iter().all(|task| matches!(task.status, TaskStatus::Done | TaskStatus::Failed))
    }

    /// Clear completed tasks from queue
    pub async fn clear_completed_tasks(&self) -> usize {
        let mut queue = self.queue.lock().await;
        let initial_size = queue.len();
        queue.retain(|task| !matches!(task.status, TaskStatus::Done | TaskStatus::Failed));
        let cleared_count = initial_size - queue.len();
        
        if cleared_count > 0 {
            log_info!("task_executor", "🧹 Cleared {} completed task(s)", cleared_count);
            println!("{} Cleared {} completed task(s)", "🧹".yellow(), cleared_count);
        }
        
        cleared_count
    }
}
