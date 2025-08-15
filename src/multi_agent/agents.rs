use super::*;
use crate::local_tools;
use async_trait::async_trait;
use serde_json::json;
use std::time::Instant;

/// Filesystem operations agent - handles all file and directory operations
pub struct FileSystemAgent;

#[async_trait]
impl Agent for FileSystemAgent {
    fn agent_type(&self) -> &'static str {
        "filesystem"
    }
    
    fn description(&self) -> String {
        "Handles file and directory operations including reading, writing, editing, and searching files".to_string()
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::FileSystem,
            AgentCapability::SearchOperations,
        ]
    }
    
    async fn can_handle_task(&self, task: &AgentTask) -> bool {
        matches!(task.task_type, TaskType::FileOperation) ||
        task.description.contains("file") ||
        task.description.contains("directory") ||
        task.description.contains("read") ||
        task.description.contains("write") ||
        task.description.contains("search")
    }
    
    async fn execute_task(&self, task: &AgentTask, context: &mut AgentContext) -> Result<AgentResult> {
        let start_time = Instant::now();
        
        // Analyze task description to determine the appropriate local tool
        let (tool_name, tool_args) = self.analyze_task_for_tool(task, context).await?;
        
        log_info!("filesystem_agent", "Executing {} for task: {}", tool_name, task.description);
        
        // Execute the local tool with safety validation
        let result_data = match local_tools::execute_local_tool(&tool_name, tool_args) {
            Ok(result) => {
                log_debug!("filesystem_agent", "Tool {} succeeded", tool_name);
                result
            }
            Err(e) => {
                log_warn!("filesystem_agent", "Tool {} failed: {}", tool_name, e);
                return Ok(AgentResult {
                    task_id: task.id.clone(),
                    agent_type: self.agent_type().to_string(),
                    success: false,
                    result_data: Value::Null,
                    error_message: Some(e.to_string()),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    sub_tasks_created: Vec::new(),
                });
            }
        };
        
        Ok(AgentResult {
            task_id: task.id.clone(),
            agent_type: self.agent_type().to_string(),
            success: true,
            result_data,
            error_message: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            sub_tasks_created: Vec::new(),
        })
    }
}

impl FileSystemAgent {
    /// Analyze task to determine which tool to use and what arguments
    async fn analyze_task_for_tool(&self, task: &AgentTask, context: &AgentContext) -> Result<(String, Value)> {
        // Check if parameters specify the tool directly
        if let Some(tool_name) = task.parameters.get("tool").and_then(|v| v.as_str()) {
            let args = task.parameters.get("args").unwrap_or(&Value::Null).clone();
            return Ok((tool_name.to_string(), args));
        }
        
        // Use LLM analysis if available
        if let Some(ref llm_client) = context.llm_client {
            if let Ok((tool_name, args)) = self.llm_analyze_task(task, llm_client).await {
                return Ok((tool_name, args));
            }
        }
        
        // Fallback to heuristic analysis
        self.heuristic_analyze_task(task)
    }
    
    /// Use LLM to analyze task and select appropriate tool
    async fn llm_analyze_task(&self, task: &AgentTask, llm_client: &OpenRouterClient) -> Result<(String, Value)> {
        let available_tools = vec![
            "list_directory", "read_file", "write_file", "edit_file", 
            "delete_path", "search_files", "glob_files", "multiedit_file"
        ];
        
        let prompt = format!(
            "Task: {}\nParameters: {}\n\nAvailable filesystem tools: {:?}\n\nSelect the most appropriate tool and generate arguments. Respond with JSON: {{\"tool\": \"tool_name\", \"args\": {{...}}}}",
            task.description,
            task.parameters,
            available_tools
        );
        
        // This would use the LLM client to analyze the task
        // For now, returning an error to fall back to heuristic analysis
        Err(anyhow!("LLM analysis not yet implemented for filesystem agent"))
    }
    
    /// Heuristic task analysis based on keywords and patterns
    fn heuristic_analyze_task(&self, task: &AgentTask) -> Result<(String, Value)> {
        let desc = task.description.to_lowercase();
        
        // Extract common patterns
        if desc.contains("list") && (desc.contains("directory") || desc.contains("folder")) {
            let path = self.extract_path(&task.parameters).unwrap_or_else(|| ".".to_string());
            return Ok(("list_directory".to_string(), json!({"path": path})));
        }
        
        if desc.contains("read") && desc.contains("file") {
            let path = self.extract_path(&task.parameters)
                .ok_or_else(|| anyhow!("File path required for read operation"))?;
            return Ok(("read_file".to_string(), json!({"path": path})));
        }
        
        if desc.contains("write") && desc.contains("file") {
            let path = self.extract_path(&task.parameters)
                .ok_or_else(|| anyhow!("File path required for write operation"))?;
            let content = task.parameters.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(("write_file".to_string(), json!({"path": path, "content": content})));
        }
        
        if desc.contains("edit") && desc.contains("file") {
            let path = self.extract_path(&task.parameters)
                .ok_or_else(|| anyhow!("File path required for edit operation"))?;
            let old_text = task.parameters.get("old_text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("old_text required for edit operation"))?;
            let new_text = task.parameters.get("new_text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("new_text required for edit operation"))?;
            return Ok(("edit_file".to_string(), json!({
                "path": path,
                "old_text": old_text,
                "new_text": new_text
            })));
        }
        
        if desc.contains("search") {
            let pattern = task.parameters.get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Search pattern required"))?;
            let directory = self.extract_path(&task.parameters).unwrap_or_else(|| ".".to_string());
            return Ok(("search_files".to_string(), json!({
                "pattern": pattern,
                "directory": directory
            })));
        }
        
        if desc.contains("delete") {
            let path = self.extract_path(&task.parameters)
                .ok_or_else(|| anyhow!("Path required for delete operation"))?;
            return Ok(("delete_path".to_string(), json!({"path": path})));
        }
        
        // Default to directory listing if no specific operation identified
        let path = self.extract_path(&task.parameters).unwrap_or_else(|| ".".to_string());
        Ok(("list_directory".to_string(), json!({"path": path})))
    }
    
    /// Extract file path from task parameters
    fn extract_path(&self, parameters: &Value) -> Option<String> {
        parameters.get("path")
            .or_else(|| parameters.get("file_path"))
            .or_else(|| parameters.get("directory"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

/// Code processing agent - handles code analysis, generation, and modification
pub struct CodeAgent {
    llm_client: Option<Arc<OpenRouterClient>>,
}

impl CodeAgent {
    pub fn new(llm_client: Option<Arc<OpenRouterClient>>) -> Self {
        Self { llm_client }
    }
}

#[async_trait]
impl Agent for CodeAgent {
    fn agent_type(&self) -> &'static str {
        "code_processor"
    }
    
    fn description(&self) -> String {
        "Analyzes, generates, and modifies code across multiple programming languages".to_string()
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::CodeProcessing,
            AgentCapability::FileSystem,
            AgentCapability::DataProcessing,
        ]
    }
    
    async fn can_handle_task(&self, task: &AgentTask) -> bool {
        matches!(task.task_type, TaskType::CodeAnalysis) ||
        task.description.contains("code") ||
        task.description.contains("function") ||
        task.description.contains("class") ||
        task.description.contains("refactor") ||
        task.description.contains("analyze") ||
        task.description.contains("generate")
    }
    
    async fn execute_task(&self, task: &AgentTask, context: &mut AgentContext) -> Result<AgentResult> {
        let start_time = Instant::now();
        
        log_info!("code_agent", "Processing code task: {}", task.description);
        
        // For code tasks, we typically need to read files first, then process them
        let result_data = match self.process_code_task(task, context).await {
            Ok(data) => data,
            Err(e) => {
                return Ok(AgentResult {
                    task_id: task.id.clone(),
                    agent_type: self.agent_type().to_string(),
                    success: false,
                    result_data: Value::Null,
                    error_message: Some(e.to_string()),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    sub_tasks_created: Vec::new(),
                });
            }
        };
        
        Ok(AgentResult {
            task_id: task.id.clone(),
            agent_type: self.agent_type().to_string(),
            success: true,
            result_data,
            error_message: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            sub_tasks_created: Vec::new(),
        })
    }
}

impl CodeAgent {
    async fn process_code_task(&self, task: &AgentTask, context: &AgentContext) -> Result<Value> {
        // Extract file paths if provided
        if let Some(file_path) = task.parameters.get("file_path").and_then(|v| v.as_str()) {
            // Read the file first
            let read_result = local_tools::execute_local_tool("read_file", json!({"path": file_path}))?;
            
            if let Some(content) = read_result.get("content").and_then(|v| v.as_str()) {
                return self.analyze_code_content(content, task, context).await;
            }
        }
        
        // If no file specified, treat as code generation/analysis task
        self.generate_or_analyze_code(task, context).await
    }
    
    async fn analyze_code_content(&self, content: &str, task: &AgentTask, _context: &AgentContext) -> Result<Value> {
        // Basic code analysis without LLM
        let lines = content.lines().count();
        let chars = content.chars().count();
        let functions = content.matches("fn ").count() + content.matches("function ").count();
        let classes = content.matches("class ").count() + content.matches("struct ").count();
        
        Ok(json!({
            "analysis": {
                "lines": lines,
                "characters": chars,
                "estimated_functions": functions,
                "estimated_classes": classes,
                "language": self.detect_language(content),
                "complexity": if lines > 1000 { "high" } else if lines > 100 { "medium" } else { "low" }
            },
            "content_preview": if content.len() > 500 { 
                format!("{}...", &content[..500]) 
            } else { 
                content.to_string() 
            }
        }))
    }
    
    async fn generate_or_analyze_code(&self, task: &AgentTask, _context: &AgentContext) -> Result<Value> {
        // For now, return a basic response indicating code processing capability
        Ok(json!({
            "message": "Code processing task received",
            "task_description": task.description,
            "note": "Full LLM-powered code generation/analysis will be implemented when LLM client is available"
        }))
    }
    
    fn detect_language(&self, content: &str) -> &str {
        if content.contains("fn ") && content.contains("use ") {
            "rust"
        } else if content.contains("function ") || content.contains("const ") {
            "javascript"
        } else if content.contains("def ") && content.contains("import ") {
            "python"
        } else if content.contains("class ") && content.contains("#include") {
            "cpp"
        } else if content.contains("public class") {
            "java"
        } else {
            "unknown"
        }
    }
}

/// Web operations agent - handles HTTP requests, API calls, and web scraping
pub struct WebAgent;

#[async_trait]
impl Agent for WebAgent {
    fn agent_type(&self) -> &'static str {
        "web_operations"
    }
    
    fn description(&self) -> String {
        "Handles web requests, API calls, data fetching, and file downloads".to_string()
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::WebOperations,
            AgentCapability::DataProcessing,
            AgentCapability::FileSystem,
        ]
    }
    
    async fn can_handle_task(&self, task: &AgentTask) -> bool {
        matches!(task.task_type, TaskType::WebRequest) ||
        task.description.contains("http") ||
        task.description.contains("api") ||
        task.description.contains("download") ||
        task.description.contains("fetch") ||
        task.description.contains("web")
    }
    
    async fn execute_task(&self, task: &AgentTask, _context: &mut AgentContext) -> Result<AgentResult> {
        let start_time = Instant::now();
        
        log_info!("web_agent", "Executing web task: {}", task.description);
        
        let result_data = match self.execute_web_operation(task).await {
            Ok(data) => data,
            Err(e) => {
                return Ok(AgentResult {
                    task_id: task.id.clone(),
                    agent_type: self.agent_type().to_string(),
                    success: false,
                    result_data: Value::Null,
                    error_message: Some(e.to_string()),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    sub_tasks_created: Vec::new(),
                });
            }
        };
        
        Ok(AgentResult {
            task_id: task.id.clone(),
            agent_type: self.agent_type().to_string(),
            success: true,
            result_data,
            error_message: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            sub_tasks_created: Vec::new(),
        })
    }
}

impl WebAgent {
    async fn execute_web_operation(&self, task: &AgentTask) -> Result<Value> {
        let desc = task.description.to_lowercase();
        
        if desc.contains("fetch") || desc.contains("get") {
            if let Some(url) = task.parameters.get("url").and_then(|v| v.as_str()) {
                return local_tools::execute_local_tool("web_fetch", json!({"url": url}));
            }
        }
        
        if desc.contains("download") {
            if let Some(url) = task.parameters.get("url").and_then(|v| v.as_str()) {
                if let Some(file_path) = task.parameters.get("file_path").and_then(|v| v.as_str()) {
                    return local_tools::execute_local_tool("download_file", json!({
                        "url": url,
                        "file_path": file_path
                    }));
                }
            }
        }
        
        Err(anyhow!("Unable to determine web operation from task: {}", task.description))
    }
}