use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use colored::*;
use std::time::{Duration, Instant};
use tokio::signal;
use tokio::select;
use walkdir::WalkDir;

use crate::openrouter_client::OpenRouterClient;
use crate::mcp_manager;
use crate::task_executor::McpToolCall;
use crate::logger::{log_info, log_debug, log_warn};

/// Represents the status of a scanning plan or step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl ScanStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            ScanStatus::Pending => "⏳",
            ScanStatus::InProgress => "🔄",
            ScanStatus::Completed => "✅",
            ScanStatus::Failed => "❌",
            ScanStatus::Cancelled => "🚫",
        }
    }

    pub fn colored_description(&self) -> colored::ColoredString {
        match self {
            ScanStatus::Pending => "Pending".yellow(),
            ScanStatus::InProgress => "In Progress".blue(),
            ScanStatus::Completed => "Completed".green(),
            ScanStatus::Failed => "Failed".red(),
            ScanStatus::Cancelled => "Cancelled".magenta(),
        }
    }
}

/// Represents a scanning step that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub step_type: ScanStepType,
    pub status: ScanStatus,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub mcp_calls: Vec<McpToolCall>,
    pub execution_time: Option<Duration>,
}

/// Types of scanning steps that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum ScanStepType {
    /// List files and directories
    ListDirectory { path: String },
    /// Read file contents
    ReadFile { path: String },
    /// Execute shell command
    ExecuteCommand { command: String, args: Vec<String> },
    /// Search for files matching patterns
    SearchFiles { path: String, pattern: String },
    /// Get file information (metadata)
    GetFileInfo { path: String },
    /// Create sub-plan for deeper analysis
    CreateSubPlan { focus_area: String, context: String },
}

/// Represents a hierarchical scanning plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPlan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub objective: String,
    pub steps: Vec<ScanStep>,
    pub sub_plans: HashMap<String, ScanPlan>,
    pub status: ScanStatus,
    pub context: HashMap<String, Value>,
    pub parent_plan_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ScanStep {
    pub fn new(title: String, description: String, step_type: ScanStepType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            description,
            step_type,
            status: ScanStatus::Pending,
            result: None,
            error: None,
            mcp_calls: Vec::new(),
            execution_time: None,
        }
    }

    pub fn display_summary(&self) -> String {
        format!("{} {} {}", 
                self.status.icon(), 
                self.status.colored_description(),
                self.title.dimmed())
    }
}

impl ScanPlan {
    pub fn new(title: String, description: String, objective: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            description,
            objective,
            steps: Vec::new(),
            sub_plans: HashMap::new(),
            status: ScanStatus::Pending,
            context: HashMap::new(),
            parent_plan_id: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn add_step(&mut self, step: ScanStep) {
        self.steps.push(step);
    }

    pub fn add_sub_plan(&mut self, sub_plan: ScanPlan) {
        self.sub_plans.insert(sub_plan.id.clone(), sub_plan);
    }

    pub fn set_context(&mut self, key: String, value: Value) {
        self.context.insert(key, value);
    }

    pub fn get_context(&self, key: &str) -> Option<&Value> {
        self.context.get(key)
    }

    pub fn display_summary(&self) -> String {
        let completed_steps = self.steps.iter().filter(|s| s.status == ScanStatus::Completed).count();
        let total_steps = self.steps.len();
        let completed_subplans = self.sub_plans.values().filter(|p| p.status == ScanStatus::Completed).count();
        let total_subplans = self.sub_plans.len();
        
        format!("{} {} {} (Steps: {}/{}, Sub-plans: {}/{})", 
                self.status.icon(), 
                self.status.colored_description(),
                self.title.bright_white(),
                completed_steps, total_steps,
                completed_subplans, total_subplans)
    }
}

/// LLM-powered project scanner with hierarchical planning
pub struct LLMProjectScanner {
    openrouter_client: OpenRouterClient,
    current_plan: Arc<Mutex<Option<ScanPlan>>>,
    scan_results: Arc<Mutex<HashMap<String, Value>>>,
    cancel_token: Arc<Mutex<bool>>,
}

impl LLMProjectScanner {
    pub async fn new() -> Result<Self> {
        let client = OpenRouterClient::new().await?;
        Ok(Self {
            openrouter_client: client,
            current_plan: Arc::new(Mutex::new(None)),
            scan_results: Arc::new(Mutex::new(HashMap::new())),
            cancel_token: Arc::new(Mutex::new(false)),
        })
    }

    /// Generate an initial scanning plan using LLM
    pub async fn generate_scanning_plan(&self, project_path: &str, scan_objective: Option<&str>) -> Result<ScanPlan> {
        let objective = scan_objective.unwrap_or("Comprehensively analyze this project to understand its structure, functionality, dependencies, and architecture");
        
        log_info!("scanner", "🧠 Generating LLM-based scanning plan for: {}", project_path);
        
        // Create context for LLM
        let context = format!(
            "You are an expert project scanner. Create a comprehensive scanning plan for the project at: {}
            
            Objective: {}
            
            Create a structured plan that uses MCP tools to:
            1. Understand the project structure (directories, key files)
            2. Identify the technology stack and dependencies
            3. Analyze code organization and architecture
            4. Understand the project's purpose and functionality
            5. Identify testing patterns and build processes
            
            Available MCP tools:
            - list_directory: List files and directories
            - read_file: Read file contents
            - search_files: Search for files matching patterns
            - get_file_info: Get file metadata
            - directory_tree: Get recursive directory structure
            
            Return a JSON object with this structure:
            {{
                \"title\": \"Project Analysis Plan\",
                \"description\": \"Brief description of the scanning approach\",
                \"objective\": \"{}\",
                \"steps\": [
                    {{
                        \"title\": \"Step title\",
                        \"description\": \"What this step accomplishes\",
                        \"step_type\": {{
                            \"type\": \"ListDirectory\",
                            \"params\": {{\"path\": \"/project\"}}
                        }}
                    }}
                ]
            }}
            
            Focus on creating 8-12 initial steps that will provide a solid foundation for understanding the project.",
            project_path, objective, objective
        );
        
        // Create a simple chat message for the LLM
        let messages = vec![
            crate::openrouter_client::ChatMessage {
                role: "user".to_string(),
                content: context,
            }
        ];
        
        let response = self.openrouter_client.chat_completion(messages).await?;
        
        // Parse the JSON response
        let plan_data: Value = self.extract_json_from_response(&response)
            .ok_or_else(|| anyhow!("Failed to parse scanning plan from LLM response"))?;
        
        let mut plan = ScanPlan::new(
            plan_data["title"].as_str().unwrap_or("Project Analysis Plan").to_string(),
            plan_data["description"].as_str().unwrap_or("LLM-generated project scanning plan").to_string(),
            objective.to_string(),
        );
        
        // Add steps from LLM response
        if let Some(steps_array) = plan_data["steps"].as_array() {
            for step_data in steps_array {
                if let Some(step) = self.parse_step_from_json(step_data)? {
                    plan.add_step(step);
                }
            }
        }
        
        // Set initial context
        plan.set_context("project_path".to_string(), Value::String(project_path.to_string()));
        plan.set_context("scan_objective".to_string(), Value::String(objective.to_string()));
        
        log_info!("scanner", "✅ Generated scanning plan with {} steps", plan.steps.len());
        Ok(plan)
    }
    
    /// Execute a scanning plan with feedback loops and sub-plan generation
    pub async fn execute_scanning_plan(&self, mut plan: ScanPlan) -> Result<HashMap<String, Value>> {
        log_info!("scanner", "🚀 Starting execution of scanning plan: {}", plan.title);
        
        // Store the current plan
        {
            let mut current_plan = self.current_plan.lock().await;
            *current_plan = Some(plan.clone());
        }
        
        // Reset cancel token
        {
            let mut cancel_token = self.cancel_token.lock().await;
            *cancel_token = false;
        }
        
        // Set up signal handling for graceful cancellation
        let cancel_token_clone = self.cancel_token.clone();
        tokio::spawn(async move {
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to install SIGINT handler");
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler");
            
            select! {
                _ = sigint.recv() => {
                    println!("\\n{} Received Ctrl+C, cancelling scan...", "🚫".yellow());
                    let mut cancel = cancel_token_clone.lock().await;
                    *cancel = true;
                }
                _ = sigterm.recv() => {
                    println!("\\n{} Received termination signal, cancelling scan...", "🚫".yellow());
                    let mut cancel = cancel_token_clone.lock().await;
                    *cancel = true;
                }
            }
        });
        
        plan.status = ScanStatus::InProgress;
        
        // Execute main plan steps
        let mut execution_results = HashMap::new();
        
        for step in &mut plan.steps {
            // Check for cancellation
            if *self.cancel_token.lock().await {
                step.status = ScanStatus::Cancelled;
                plan.status = ScanStatus::Cancelled;
                log_info!("scanner", "🚫 Scan cancelled by user");
                return Err(anyhow!("Scan cancelled by user"));
            }
            
            println!("\\n{} Executing: {}", "🔄".blue(), step.title.bright_white());
            
            let step_start = Instant::now();
            step.status = ScanStatus::InProgress;
            
            match self.execute_scan_step(step, &plan.context).await {
                Ok(result) => {
                    step.execution_time = Some(step_start.elapsed());
                    step.status = ScanStatus::Completed;
                    step.result = Some(result.clone());
                    execution_results.insert(step.id.clone(), result);
                    
                    println!("{} Completed: {}", "✅".green(), step.title);
                    log_info!("scanner", "✅ Step completed: {}", step.title);
                }
                Err(e) => {
                    step.execution_time = Some(step_start.elapsed());
                    step.status = ScanStatus::Failed;
                    step.error = Some(e.to_string());
                    
                    println!("{} Failed: {} - {}", "❌".red(), step.title, e);
                    log_warn!("scanner", "❌ Step failed: {} - {}", step.title, e);
                }
            }
            
            // Update current plan
            {
                let mut current_plan = self.current_plan.lock().await;
                if let Some(ref mut current) = current_plan.as_mut() {
                    if let Some(current_step) = current.steps.iter_mut().find(|s| s.id == step.id) {
                        *current_step = step.clone();
                    }
                }
            }
        }
        
        // Check if we need to generate sub-plans based on results
        let sub_plan_needed = self.should_generate_sub_plans(&plan, &execution_results).await?;
        
        if sub_plan_needed && !*self.cancel_token.lock().await {
            let sub_plans = self.generate_sub_plans(&plan, &execution_results).await?;
            
            for mut sub_plan in sub_plans {
                sub_plan.parent_plan_id = Some(plan.id.clone());
                
                println!("\\n{} Executing sub-plan: {}", "🧠".bright_blue(), sub_plan.title.bright_white());
                
                match Box::pin(self.execute_scanning_plan(sub_plan.clone())).await {
                    Ok(sub_results) => {
                        // Merge sub-plan results
                        for (key, value) in sub_results {
                            execution_results.insert(format!("{}_{}", sub_plan.id, key), value);
                        }
                        sub_plan.status = ScanStatus::Completed;
                    }
                    Err(e) => {
                        sub_plan.status = ScanStatus::Failed;
                        log_warn!("scanner", "❌ Sub-plan failed: {} - {}", sub_plan.title, e);
                    }
                }
                
                plan.add_sub_plan(sub_plan);
            }
        }
        
        // Finalize plan status
        if !*self.cancel_token.lock().await {
            plan.status = ScanStatus::Completed;
            log_info!("scanner", "🎉 Scanning plan completed: {}", plan.title);
        }
        
        // Store final results
        {
            let mut scan_results = self.scan_results.lock().await;
            for (key, value) in &execution_results {
                scan_results.insert(key.clone(), value.clone());
            }
        }
        
        Ok(execution_results)
    }
    
    /// Execute a single scanning step
    async fn execute_scan_step(&self, step: &mut ScanStep, _context: &HashMap<String, Value>) -> Result<Value> {
        log_debug!("scanner", "🔧 Executing step: {} (type: {:?})", step.title, step.step_type);
        
        match step.step_type.clone() {
            ScanStepType::ListDirectory { path } => {
                self.execute_list_directory(step, &path).await
            }
            ScanStepType::ReadFile { path } => {
                self.execute_read_file(step, &path).await
            }
            ScanStepType::SearchFiles { path, pattern } => {
                self.execute_search_files(step, &path, &pattern).await
            }
            ScanStepType::GetFileInfo { path } => {
                self.execute_get_file_info(step, &path).await
            }
            ScanStepType::ExecuteCommand { command, args } => {
                self.execute_shell_command(step, &command, &args).await
            }
            ScanStepType::CreateSubPlan { focus_area, context: sub_context } => {
                // This is handled during sub-plan generation
                Ok(json!({
                    "type": "sub_plan_marker",
                    "focus_area": focus_area,
                    "context": sub_context
                }))
            }
        }
    }
    
    async fn execute_list_directory(&self, step: &mut ScanStep, path: &str) -> Result<Value> {
        let tool_call = McpToolCall {
            server_name: "filesystem".to_string(),
            tool_name: "list_directory".to_string(),
            arguments: json!({ "path": path }),
            result: None,
        };
        
        let result = self.execute_mcp_tool_call(&tool_call).await?;
        step.mcp_calls.push(tool_call);
        
        Ok(result)
    }
    
    async fn execute_read_file(&self, step: &mut ScanStep, path: &str) -> Result<Value> {
        let tool_call = McpToolCall {
            server_name: "filesystem".to_string(),
            tool_name: "read_file".to_string(),
            arguments: json!({ "path": path }),
            result: None,
        };
        
        let result = self.execute_mcp_tool_call(&tool_call).await?;
        step.mcp_calls.push(tool_call);
        
        Ok(result)
    }
    
    async fn execute_search_files(&self, step: &mut ScanStep, path: &str, pattern: &str) -> Result<Value> {
        let tool_call = McpToolCall {
            server_name: "filesystem".to_string(),
            tool_name: "search_files".to_string(),
            arguments: json!({ "path": path, "pattern": pattern }),
            result: None,
        };
        
        let result = self.execute_mcp_tool_call(&tool_call).await?;
        step.mcp_calls.push(tool_call);
        
        Ok(result)
    }
    
    async fn execute_get_file_info(&self, step: &mut ScanStep, path: &str) -> Result<Value> {
        let tool_call = McpToolCall {
            server_name: "filesystem".to_string(),
            tool_name: "get_file_info".to_string(),
            arguments: json!({ "path": path }),
            result: None,
        };
        
        let result = self.execute_mcp_tool_call(&tool_call).await?;
        step.mcp_calls.push(tool_call);
        
        Ok(result)
    }
    
    async fn execute_shell_command(&self, _step: &mut ScanStep, command: &str, args: &[String]) -> Result<Value> {
        // For security, we'll limit shell commands to safe read-only operations
        match command {
            "ls" | "find" | "grep" | "wc" | "head" | "tail" | "cat" | "file" => {
                // These are considered safe read-only commands
                log_debug!("scanner", "🔧 Would execute shell command: {} {:?}", command, args);
                // In a real implementation, we'd execute this safely
                Ok(json!({
                    "command": command,
                    "args": args,
                    "status": "simulated",
                    "note": "Shell command execution is simulated for security"
                }))
            }
            _ => {
                Err(anyhow!("Shell command '{}' not allowed for security reasons", command))
            }
        }
    }
    
    async fn execute_mcp_tool_call(&self, tool_call: &McpToolCall) -> Result<Value> {
        let global_manager = mcp_manager::get_mcp_manager();
        let guard = global_manager.lock().await;
        let manager = guard.as_ref()
            .ok_or_else(|| anyhow!("MCP manager not available"))?;
        
        tokio::time::timeout(
            Duration::from_secs(10),
            manager.call_tool(
                &tool_call.server_name,
                &tool_call.tool_name,
                tool_call.arguments.clone()
            )
        ).await
        .map_err(|_| anyhow!("MCP tool call timed out"))?
    }
    
    /// Determine if sub-plans should be generated based on current results
    async fn should_generate_sub_plans(&self, plan: &ScanPlan, results: &HashMap<String, Value>) -> Result<bool> {
        // Use LLM to decide if we need deeper analysis
        let context = format!(
            "Based on the following scanning results, determine if we need to create sub-plans for deeper analysis.
            
            Plan objective: {}
            Completed steps: {}
            
            Results summary:
            {}
            
            Should we create sub-plans for more detailed analysis? Consider:
            1. Are there complex subdirectories that need focused analysis?
            2. Did we discover interesting patterns that need deeper investigation?
            3. Are there specific technology stacks or frameworks that need specialized scanning?
            4. Are there configuration files or build systems that need detailed analysis?
            
            Respond with just 'true' or 'false'.",
            plan.objective,
            plan.steps.len(),
            self.summarize_results(results)
        );
        
        // Create a simple chat message for the LLM
        let messages = vec![
            crate::openrouter_client::ChatMessage {
                role: "user".to_string(),
                content: context,
            }
        ];
        
        let response = self.openrouter_client.chat_completion(messages).await?;
        let should_generate = response.trim().to_lowercase().contains("true");
        
        log_debug!("scanner", "🤔 Should generate sub-plans: {}", should_generate);
        Ok(should_generate)
    }
    
    /// Generate sub-plans for deeper analysis
    async fn generate_sub_plans(&self, parent_plan: &ScanPlan, results: &HashMap<String, Value>) -> Result<Vec<ScanPlan>> {
        log_info!("scanner", "🧠 Generating sub-plans for deeper analysis");
        
        let context = format!(
            "Based on the scanning results from the parent plan, create 1-3 focused sub-plans for deeper analysis.
            
            Parent plan: {}
            Parent objective: {}
            
            Current results:
            {}
            
            Create sub-plans that focus on specific areas that need deeper investigation. Each sub-plan should have:
            - A clear focus area (e.g., 'Frontend Architecture Analysis', 'Build System Investigation', 'API Structure Analysis')
            - 5-8 specific steps using MCP tools
            - Clear objective for what it will accomplish
            
            Return a JSON array of sub-plan objects:
            [
                {{
                    \"title\": \"Sub-plan title\",
                    \"description\": \"What this sub-plan focuses on\",
                    \"objective\": \"Specific objective for this sub-plan\",
                    \"steps\": [
                        // 5-8 steps using MCP tools
                    ]
                }}
            ]",
            parent_plan.title,
            parent_plan.objective,
            self.summarize_results(results)
        );
        
        // Create a simple chat message for the LLM
        let messages = vec![
            crate::openrouter_client::ChatMessage {
                role: "user".to_string(),
                content: context,
            }
        ];
        
        let response = self.openrouter_client.chat_completion(messages).await?;
        
        let sub_plans_data: Value = self.extract_json_from_response(&response)
            .ok_or_else(|| anyhow!("Failed to parse sub-plans from LLM response"))?;
        
        let mut sub_plans = Vec::new();
        
        if let Some(plans_array) = sub_plans_data.as_array() {
            for plan_data in plans_array {
                let mut sub_plan = ScanPlan::new(
                    plan_data["title"].as_str().unwrap_or("Sub-plan").to_string(),
                    plan_data["description"].as_str().unwrap_or("Generated sub-plan").to_string(),
                    plan_data["objective"].as_str().unwrap_or("Sub-plan objective").to_string(),
                );
                
                // Add steps from LLM response
                if let Some(steps_array) = plan_data["steps"].as_array() {
                    for step_data in steps_array {
                        if let Some(step) = self.parse_step_from_json(step_data)? {
                            sub_plan.add_step(step);
                        }
                    }
                }
                
                // Inherit context from parent
                for (key, value) in &parent_plan.context {
                    sub_plan.set_context(key.clone(), value.clone());
                }
                
                sub_plans.push(sub_plan);
            }
        }
        
        log_info!("scanner", "✅ Generated {} sub-plans", sub_plans.len());
        Ok(sub_plans)
    }
    
    /// Parse a scanning step from JSON data
    fn parse_step_from_json(&self, step_data: &Value) -> Result<Option<ScanStep>> {
        let title = step_data["title"].as_str().unwrap_or("Untitled Step").to_string();
        let description = step_data["description"].as_str().unwrap_or("").to_string();
        
        let step_type = match step_data["step_type"]["type"].as_str() {
            Some("ListDirectory") => {
                let path = step_data["step_type"]["params"]["path"].as_str().unwrap_or("/project").to_string();
                ScanStepType::ListDirectory { path }
            }
            Some("ReadFile") => {
                let path = step_data["step_type"]["params"]["path"].as_str().unwrap_or("/project/README.md").to_string();
                ScanStepType::ReadFile { path }
            }
            Some("SearchFiles") => {
                let path = step_data["step_type"]["params"]["path"].as_str().unwrap_or("/project").to_string();
                let pattern = step_data["step_type"]["params"]["pattern"].as_str().unwrap_or("*").to_string();
                ScanStepType::SearchFiles { path, pattern }
            }
            Some("GetFileInfo") => {
                let path = step_data["step_type"]["params"]["path"].as_str().unwrap_or("/project").to_string();
                ScanStepType::GetFileInfo { path }
            }
            Some("ExecuteCommand") => {
                let command = step_data["step_type"]["params"]["command"].as_str().unwrap_or("ls").to_string();
                let args = step_data["step_type"]["params"]["args"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_else(Vec::new);
                ScanStepType::ExecuteCommand { command, args }
            }
            _ => {
                log_warn!("scanner", "⚠️ Unknown step type in JSON, skipping step");
                return Ok(None);
            }
        };
        
        Ok(Some(ScanStep::new(title, description, step_type)))
    }
    
    /// Extract JSON from LLM response (handles markdown code blocks)
    fn extract_json_from_response(&self, response: &str) -> Option<Value> {
        // Try to find JSON in markdown code blocks first
        if let Some(start_pos) = response.find("```json") {
            if let Some(end_pos) = response[start_pos + 7..].find("```") {
                let json_str = &response[start_pos + 7..start_pos + 7 + end_pos];
                if let Ok(json_val) = serde_json::from_str(json_str) {
                    return Some(json_val);
                }
            }
        }
        
        // Try alternative markdown format
        if let Some(start_pos) = response.find("```") {
            if let Some(end_pos) = response[start_pos + 3..].find("```") {
                let potential_json = &response[start_pos + 3..start_pos + 3 + end_pos];
                // Skip any language identifier line
                let json_start = if potential_json.starts_with("json\n") {
                    5
                } else if potential_json.contains('\n') {
                    potential_json.find('\n').unwrap() + 1
                } else {
                    0
                };
                
                let json_str = &potential_json[json_start..];
                if let Ok(json_val) = serde_json::from_str(json_str) {
                    return Some(json_val);
                }
            }
        }
        
        // Try to find JSON in the response directly
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end >= start {
                    let json_str = &response[start..end + 1];
                    if let Ok(json_val) = serde_json::from_str(json_str) {
                        return Some(json_val);
                    }
                }
            }
        }
        
        None
    }
    
    /// Create a summary of scanning results
    fn summarize_results(&self, results: &HashMap<String, Value>) -> String {
        let mut summary = String::new();
        
        for (step_id, result) in results.iter().take(10) { // Limit to first 10 for brevity
            summary.push_str(&format!(
                "Step {}: {}\n", 
                step_id,
                result.to_string().chars().take(200).collect::<String>()
            ));
        }
        
        if results.len() > 10 {
            summary.push_str(&format!("... and {} more results\n", results.len() - 10));
        }
        
        summary
    }
    
    /// Get the current scanning plan status
    pub async fn get_current_plan_status(&self) -> Option<ScanPlan> {
        let current_plan = self.current_plan.lock().await;
        current_plan.clone()
    }
    
    /// Cancel the current scanning operation
    pub async fn cancel_scan(&self) {
        let mut cancel_token = self.cancel_token.lock().await;
        *cancel_token = true;
        log_info!("scanner", "🚫 Scan cancellation requested");
    }
    
    /// Get all scanning results
    pub async fn get_scan_results(&self) -> HashMap<String, Value> {
        let scan_results = self.scan_results.lock().await;
        scan_results.clone()
    }
}

/// Backward compatibility function for the existing interface
pub async fn summarize_project<P: AsRef<Path>>(root: P) -> Result<Value> {
    let project_path = root.as_ref().to_string_lossy().to_string();
    
    // Try to use the new LLM-based scanner
    match LLMProjectScanner::new().await {
        Ok(scanner) => {
            log_info!("scanner", "🧠 Using LLM-based project scanner");
            
            match scanner.generate_scanning_plan(&project_path, None).await {
                Ok(plan) => {
                    match scanner.execute_scanning_plan(plan).await {
                        Ok(results) => {
                            // Convert results to a summary format
                            let mut summary = json!({
                                "scan_type": "llm_based",
                                "project_path": project_path,
                                "total_results": results.len(),
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            });
                            
                            // Add key findings
                            for (key, value) in results.iter().take(5) {
                                summary[format!("result_{}", key)] = value.clone();
                            }
                            
                            Ok(summary)
                        }
                        Err(e) => {
                            log_warn!("scanner", "⚠️ LLM scan execution failed: {}, falling back to basic scan", e);
                            basic_project_scan(root)
                        }
                    }
                }
                Err(e) => {
                    log_warn!("scanner", "⚠️ LLM plan generation failed: {}, falling back to basic scan", e);
                    basic_project_scan(root)
                }
            }
        }
        Err(e) => {
            log_warn!("scanner", "⚠️ LLM scanner initialization failed: {}, falling back to basic scan", e);
            basic_project_scan(root)
        }
    }
}

/// Basic project scan as fallback when LLM is not available
fn basic_project_scan<P: AsRef<Path>>(root: P) -> Result<Value> {
    use std::fs;
    
    let root = root.as_ref();
    let cargo_toml = root.join("Cargo.toml");
    let has_cargo = cargo_toml.exists();
    let mut file_count = 0usize;
    let mut rust_count = 0usize;
    let mut test_count = 0usize;

    if root.exists() {
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            file_count += 1;
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if ext == "rs" { rust_count += 1; }
                if ext == "rs" && p.to_string_lossy().contains("tests/") { test_count += 1; }
            }
        }
    }

    let cargo_name = if has_cargo {
        fs::read_to_string(&cargo_toml)
            .ok()
            .and_then(|s| {
                s.lines()
                    .skip_while(|l| !l.trim().starts_with("[package]"))
                    .skip(1)
                    .take_while(|l| !l.trim().starts_with('['))
                    .find_map(|l| {
                        let t = l.trim();
                        if let Some(rest) = t.strip_prefix("name") {
                            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=').trim();
                            let name = rest.trim_matches('"').to_string();
                            if !name.is_empty() { return Some(name); }
                        }
                        None
                    })
            })
    } else { None };

    Ok(json!({
        "scan_type": "basic_fallback",
        "has_cargo": has_cargo,
        "package_name": cargo_name,
        "file_count": file_count,
        "rust_files": rust_count,
        "approx_test_files": test_count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}