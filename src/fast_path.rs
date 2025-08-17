use anyhow::Result;
use crate::logger::{log_debug, log_info};
use std::time::Instant;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Fast execution path for basic commands that don't require enhancement systems
/// This bypasses heavy initialization to provide sub-second response times
pub struct FastPath {
    prompts_directory: PathBuf,
}

/// Commands that can use the fast path
#[derive(Debug, Clone)]
pub enum FastCommand {
    Help,
    Version,
    List,
    Search(String),
    Show(String),
    Query { file: String, subject: String, prompt: String },
}

impl FastPath {
    /// Create a new fast path executor
    pub fn new() -> Result<Self> {
        log_debug!("fast_path", "🚀 Initializing fast path executor");
        let start = Instant::now();
        
        let prompts_directory = PathBuf::from("prompts");
        
        let duration = start.elapsed();
        log_info!("fast_path", "✅ Fast path initialized in {:?}", duration);
        
        Ok(Self { prompts_directory })
    }

    /// Get all prompt files from the directory
    fn get_prompt_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        if !self.prompts_directory.exists() {
            return Ok(files); // Return empty list if directory doesn't exist
        }

        for entry in WalkDir::new(&self.prompts_directory) {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "yaml" || extension == "yml" {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
        
        files.sort();
        Ok(files)
    }

    /// Execute a command via the fast path
    pub async fn execute(&self, command: FastCommand) -> Result<String> {
        let start = Instant::now();
        log_debug!("fast_path", "⚡ Executing fast path command: {:?}", command);
        
        let result = match command {
            FastCommand::Help => self.show_help(),
            FastCommand::Version => self.show_version(),
            FastCommand::List => self.list_prompts().await,
            FastCommand::Search(term) => self.search_prompts(&term).await,
            FastCommand::Show(file) => self.show_prompt(&file).await,
            FastCommand::Query { file, subject, prompt } => {
                self.query_prompt(&file, &subject, &prompt).await
            }
        };
        
        let duration = start.elapsed();
        log_info!("fast_path", "✅ Fast path execution completed in {:?}", duration);
        
        result
    }

    /// Show help information
    fn show_help(&self) -> Result<String> {
        Ok(r#"CAI (Conversational AI Interface) - Fast Mode
A CLI tool for managing and searching prompt collections

Usage: cai [OPTIONS] [COMMAND]

Commands:
  list       List all available prompts
  search     Search prompts by keyword  
  show       Show details of a specific prompt file
  query      Query a specific prompt
  chat       Start interactive chat mode (uses enhanced path)
  mcp        MCP (Model Context Protocol) tools management
  workflow   Workflow orchestration commands
  help       Print this message

Options:
  -d, --directory <DIRECTORY>  Prompts directory [default: prompts]
  --debug                      Enable debugging output  
  -h, --help                   Print help
  -V, --version                Print version

Note: Basic commands (list, search, show, query) use fast path for optimal performance.
Advanced commands (chat, workflow, mcp) use enhanced path with full feature set."#.to_string())
    }

    /// Show version information
    fn show_version(&self) -> Result<String> {
        Ok(format!("cai {} (fast path enabled)", env!("CARGO_PKG_VERSION")))
    }

    /// List all prompts quickly
    async fn list_prompts(&self) -> Result<String> {
        log_debug!("fast_path", "📋 Listing prompts via fast path");
        
        let files = self.get_prompt_files()?;
        
        if files.is_empty() {
            return Ok("No prompt files found in directory.".to_string());
        }

        let mut output = String::new();
        output.push_str("Available Prompt Files:\n");
        output.push_str("======================\n\n");

        for file in files {
            let file_stem = file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            output.push_str(&format!("📁 {}\n", file_stem));
            
            // Quick preview of file contents without full parsing
            if let Ok(content) = std::fs::read_to_string(&file) {
                let lines: Vec<&str> = content.lines().take(3).collect();
                for line in lines {
                    if !line.trim().is_empty() && !line.trim_start().starts_with('#') {
                        output.push_str(&format!("   {}\n", line.trim()));
                        break;
                    }
                }
            }
            output.push_str("\n");
        }

        Ok(output)
    }

    /// Search prompts quickly
    async fn search_prompts(&self, search_term: &str) -> Result<String> {
        log_debug!("fast_path", "🔍 Searching prompts for: '{}'", search_term);
        
        if search_term.is_empty() {
            return Ok("Search term cannot be empty. Please provide a search term.".to_string());
        }

        let files = self.get_prompt_files()?;
        let mut results = Vec::new();
        let search_lower = search_term.to_lowercase();

        for file in files {
            if let Ok(content) = std::fs::read_to_string(&file) {
                let content_lower = content.to_lowercase();
                if content_lower.contains(&search_lower) {
                    let file_name = file.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    
                    // Find matching lines
                    let matching_lines: Vec<String> = content
                        .lines()
                        .enumerate()
                        .filter(|(_, line)| line.to_lowercase().contains(&search_lower))
                        .take(3) // Limit to first 3 matches per file
                        .map(|(i, line)| format!("  Line {}: {}", i + 1, line.trim()))
                        .collect();
                    
                    results.push((file_name.to_string(), matching_lines));
                }
            }
        }

        if results.is_empty() {
            return Ok(format!("No results found for '{}'", search_term));
        }

        let mut output = String::new();
        output.push_str(&format!("Search Results for '{}':\n", search_term));
        output.push_str(&format!("{}\n\n", "=".repeat(30 + search_term.len())));

        for (file_name, lines) in results {
            output.push_str(&format!("📁 {}\n", file_name));
            for line in lines {
                output.push_str(&format!("{}\n", line));
            }
            output.push_str("\n");
        }

        Ok(output)
    }

    /// Show specific prompt file quickly
    async fn show_prompt(&self, file_name: &str) -> Result<String> {
        log_debug!("fast_path", "📄 Showing prompt file: '{}'", file_name);
        
        let files = self.get_prompt_files()?;
        
        // Find matching file
        let target_file = files.iter().find(|file| {
            file.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case(file_name))
                .unwrap_or(false)
        });

        let file_path = match target_file {
            Some(path) => path,
            None => {
                return Ok(format!("Prompt file '{}' not found.", file_name));
            }
        };

        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let mut output = String::new();
                output.push_str(&format!("📄 Prompt File: {}\n", file_name));
                output.push_str(&format!("{}\n\n", "=".repeat(20 + file_name.len())));
                output.push_str(&content);
                Ok(output)
            }
            Err(e) => {
                Ok(format!("Error reading file '{}': {}", file_name, e))
            }
        }
    }

    /// Query specific prompt quickly
    async fn query_prompt(&self, file_name: &str, subject: &str, prompt_name: &str) -> Result<String> {
        log_debug!("fast_path", "🔍 Querying prompt: file='{}', subject='{}', prompt='{}'", 
                  file_name, subject, prompt_name);
        
        // For fast path, do simple text search instead of full YAML parsing
        let files = self.get_prompt_files()?;
        
        let target_file = files.iter().find(|file| {
            file.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case(file_name))
                .unwrap_or(false)
        });

        let file_path = match target_file {
            Some(path) => path,
            None => {
                return Ok(format!("Prompt file '{}' not found.", file_name));
            }
        };

        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                // Simple search for the prompt content
                let lines: Vec<&str> = content.lines().collect();
                let mut found_content = String::new();
                let mut in_target_section = false;
                let mut section_depth = 0;

                for line in lines {
                    let trimmed = line.trim();
                    
                    // Look for subject section
                    if trimmed.contains(subject) && trimmed.contains(':') {
                        in_target_section = true;
                        section_depth = line.len() - line.trim_start().len();
                        continue;
                    }
                    
                    if in_target_section {
                        let current_depth = line.len() - line.trim_start().len();
                        
                        // Check if we've moved to a different section at the same level
                        if current_depth <= section_depth && !trimmed.is_empty() && trimmed.contains(':') {
                            break;
                        }
                        
                        // Look for the specific prompt
                        if trimmed.contains(prompt_name) && trimmed.contains(':') {
                            found_content.push_str(&format!("Found prompt '{}':\n\n", prompt_name));
                        } else if !found_content.is_empty() {
                            found_content.push_str(line);
                            found_content.push_str("\n");
                        }
                    }
                }

                if found_content.is_empty() {
                    Ok(format!("Prompt '{}' not found in subject '{}' of file '{}'", 
                              prompt_name, subject, file_name))
                } else {
                    Ok(found_content)
                }
            }
            Err(e) => {
                Ok(format!("Error reading file '{}': {}", file_name, e))
            }
        }
    }
}

/// Determine if a command can use the fast path
pub fn can_use_fast_path(args: &[String]) -> bool {
    if args.is_empty() {
        return false;
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => true,
        "--version" | "-V" | "version" => true,
        "list" => true,
        "search" => true,
        "show" => true,
        "query" => true,
        // Complex commands require enhanced path
        "chat" | "workflow" | "mcp" | "scan" | "task-demo" => false,
        _ => false,
    }
}

/// Parse arguments into a fast command
pub fn parse_fast_command(args: &[String]) -> Result<FastCommand> {
    if args.is_empty() {
        return Ok(FastCommand::Help);
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => Ok(FastCommand::Help),
        "--version" | "-V" | "version" => Ok(FastCommand::Version),
        "list" => Ok(FastCommand::List),
        "search" => {
            let term = if args.len() > 1 {
                args[1].clone()
            } else {
                String::new()
            };
            Ok(FastCommand::Search(term))
        }
        "show" => {
            if args.len() > 1 {
                Ok(FastCommand::Show(args[1].clone()))
            } else {
                Err(anyhow::anyhow!("show command requires a file name"))
            }
        }
        "query" => {
            if args.len() >= 4 {
                Ok(FastCommand::Query {
                    file: args[1].clone(),
                    subject: args[2].clone(),
                    prompt: args[3].clone(),
                })
            } else {
                Err(anyhow::anyhow!("query command requires file, subject, and prompt arguments"))
            }
        }
        _ => Err(anyhow::anyhow!("Command '{}' cannot use fast path", args[0])),
    }
}