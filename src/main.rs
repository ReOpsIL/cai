mod prompt_loader;
mod openrouter_client;
mod chat_interface;
mod logger;
mod mcp_config;
mod mcp_client;
mod mcp_manager;
mod task_executor;
mod feedback_loop;
mod workflow_orchestrator;
mod session_manager;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use prompt_loader::{MatchType, PromptManager, SearchResult};
use chat_interface::ChatInterface;
use logger::ops;
use task_executor::TaskExecutor;
use workflow_orchestrator::WorkflowOrchestrator;
use std::path::PathBuf;
use std::time::Instant;
use colored::control as color_control;

#[derive(Parser)]
#[command(name = "cai")]
#[command(about = "A CLI tool for managing and searching prompt collections")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = "prompts")]
    directory: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available prompts
    List,
    /// Search prompts by keyword
    Search {
        /// Search query
        query: String,
        /// Resolve URL contents during search (file:// only)
        #[arg(long, default_value_t = false)]
        resolve_urls: bool,
    },
    /// Show details of a specific prompt file
    Show {
        /// File name to show
        file_name: String,
    },
    /// Query a specific prompt
    Query {
        /// File name
        file: String,
        /// Subject name
        subject: String,
        /// Prompt title
        prompt: String,
    },
    /// Start interactive chat mode for task planning and prompt management
    Chat {
        /// Optional workflow session ID to use/resume
        #[arg(long)]
        workflow_id: Option<String>,
    },
    /// MCP (Model Context Protocol) tools management
    Mcp {
        #[command(subcommand)]
        action: McpCommands,
    },
    /// Test task execution system with demo tasks
    TaskDemo,
    /// Workflow orchestration commands
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Initialize default MCP configuration file (mcp-config.json)
    Init,
    /// List available MCP servers
    List,
    /// Start an MCP server
    Start {
        /// Server name to start
        server_name: String,
    },
    /// Stop an MCP server
    Stop {
        /// Server name to stop
        server_name: String,
    },
    /// List tools available from a server
    Tools {
        /// Server name
        server_name: String,
    },
    /// Call a tool on a server
    Call {
        /// Server name
        server_name: String,
        /// Tool name
        tool_name: String,
        /// Tool arguments as JSON
        #[arg(long)]
        args: Option<String>,
    },
    /// List resources available from a server
    Resources {
        /// Server name
        server_name: String,
    },
    /// Show server status
    Status,
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Start a new workflow with LLM-driven goal decomposition
    Start {
        /// Description of what you want to accomplish
        description: String,
    },
    /// Show status of active workflows
    Status,
    /// Show detailed status of a specific workflow
    Show {
        /// Workflow ID
        workflow_id: String,
    },
    /// Continue execution of a workflow
    Continue {
        /// Workflow ID
        workflow_id: String,
    },
    /// Clean up completed workflows
    Cleanup,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    logger::init();
    // Disable colors if NO_COLOR set or stdout is not a TTY
    let no_color_env = std::env::var("NO_COLOR").is_ok() || std::env::var("CAI_NO_COLOR").map(|v| v=="1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
    #[allow(deprecated)]
    {
        // IsTerminal is stable; avoid adding deps. If not a terminal, disable colors.
        use std::io::IsTerminal;
        if no_color_env || !std::io::stdout().is_terminal() {
            color_control::set_override(false);
        }
    }
    ops::startup("APP", "starting CAI application");

    // Initialize MCP servers on startup
    println!("{} Initializing MCP servers...", "🔧".cyan());
    match mcp_manager::initialize_mcp().await {
        Ok(_) => {
            println!("{} MCP initialization completed", "✅".green());
        }
        Err(e) => {
            println!("{} MCP initialization failed: {}", "⚠️".yellow(), e);
            println!("{} This is not critical - MCP features will be unavailable", "💡".blue());
        }
    }

    // Set up graceful shutdown
    let _shutdown_result = setup_shutdown_handler();

    let start_time = Instant::now();
    
    let cli = Cli::parse();
    // Expose selected prompts directory to URL security checks
    std::env::set_var("CAI_PROMPTS_DIR", &cli.directory);
    log_debug!("main", "📋 Parsed CLI arguments: directory={:?}", cli.directory);

    // Load prompt manager with timing
    let load_start = Instant::now();
    ops::startup("PROMPTS", &format!("loading from {:?}", cli.directory));
    let mut manager = PromptManager::load_from_directory(&cli.directory)?;
    let load_duration = load_start.elapsed().as_millis() as u64;
    ops::performance("PROMPT_LOADING", load_duration);
    
    let prompt_count: usize = manager.list_all().iter().map(|p| {
        p.prompt_file.subjects.iter().map(|s| s.prompts.len()).sum::<usize>()
    }).sum();
    log_info!("main", "📚 Loaded {} prompt files with {} total prompts", 
        manager.list_all().len(), prompt_count);

    // Execute command with timing
    let command_start = Instant::now();
    let result = match &cli.command {
        Commands::List => {
            log_info!("main", "📋 Executing LIST command");
            list_prompts(&manager)
        },
        Commands::Search { query, resolve_urls } => {
            log_info!("main", "🔍 Executing SEARCH command with query: '{}'", query);
            search_prompts(&manager, &query, *resolve_urls)
        },
        Commands::Show { file_name } => {
            log_info!("main", "👁️ Executing SHOW command for file: '{}'", file_name);
            show_prompt_file(&manager, &file_name).await
        },
        Commands::Query { file, subject, prompt } => {
            log_info!("main", "❓ Executing QUERY command: {} → {} → {}", file, subject, prompt);
            query_prompt(&manager, &file, &subject, &prompt).await
        },
        Commands::Chat { workflow_id } => {
            log_info!("main", "💬 Executing CHAT command with workflow_id: {:?}", workflow_id);
            start_chat_mode(&mut manager, workflow_id.as_deref()).await
        },
        Commands::Mcp { action } => {
            log_info!("main", "🔧 Executing MCP command");
            handle_mcp_command(action).await
        },
        Commands::TaskDemo => {
            log_info!("main", "🚀 Running task demo");
            run_task_demo().await
        },
        Commands::Workflow { action } => {
            log_info!("main", "🧠 Executing workflow command");
            handle_workflow_command(action).await
        },
    };

    let command_duration = command_start.elapsed().as_millis() as u64;
    ops::performance("COMMAND_EXECUTION", command_duration);

    let total_duration = start_time.elapsed().as_millis() as u64;
    ops::performance("TOTAL_RUNTIME", total_duration);

    if let Err(ref e) = result {
        ops::error_with_context("MAIN", &e.to_string(), None);
    } else {
        log_info!("main", "✅ Command completed successfully");
    }

    // Cleanup MCP servers before exit
    if let Err(e) = mcp_manager::shutdown_mcp().await {
        log_error!("mcp","⚠️  Error during MCP shutdown: {}", e);
    }

    result
}

/// Setup signal handlers for graceful shutdown
fn setup_shutdown_handler() -> Result<()> {
    use tokio::signal;
    
    tokio::spawn(async {
        let ctrl_c = signal::ctrl_c();
        
        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };
        
        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();
        
        tokio::select! {
            _ = ctrl_c => {
                println!("\n🛑 Received Ctrl+C, shutting down gracefully...");
            }
            _ = terminate => {
                println!("\n🛑 Received SIGTERM, shutting down gracefully...");
            }
        }
        
        // Shutdown MCP servers
        if let Err(e) = mcp_manager::shutdown_mcp().await {
            log_error!("mcp","⚠️  Error during graceful MCP shutdown: {}", e);
        }
        
        std::process::exit(0);
    });
    
    Ok(())
}

async fn run_task_demo() -> Result<()> {
    println!("{}", "🚀 Task Execution Demo".bright_blue().bold());
    println!("{}", "Demonstrating MCP tool integration with task execution".dimmed());
    println!();

    // Try to create LLM-enabled task executor, fallback to basic if it fails
    let executor = match TaskExecutor::with_llm_analysis().await {
        Ok(executor) => {
            println!("{} Using LLM-powered intelligent tool analysis", "🧠".bright_blue());
            executor
        }
        Err(e) => {
            println!("{} LLM analysis not available ({}), using heuristic fallback", "⚠️".yellow(), e);
            TaskExecutor::new()
        }
    };
    
    // Add some demo tasks
    let demo_tasks = vec![
        "List all files in the current directory".to_string(),
        "Read the contents of README.md file".to_string(),
        "Show the current working directory structure".to_string(),
    ];

    println!("{} Adding demo tasks to queue...", "📝".cyan());
    executor.add_tasks(demo_tasks).await?;

    println!("{} Executing all tasks...", "⚡".yellow());
    executor.execute_all().await?;

    println!("\n{} Demo completed!", "🎉".green().bold());
    Ok(())
}

async fn start_chat_mode(manager: &mut PromptManager, workflow_id: Option<&str>) -> Result<()> {
    match ChatInterface::new().await {
        Ok(mut chat) => {
            if let Some(id) = workflow_id {
                chat.set_workflow_id(id.to_string()).await?;
            }
            chat.start_chat(manager).await?;
        }
        Err(e) => {
            println!("{} Failed to start chat mode: {}", "❌".red(), e);
            println!("{} Make sure OPENROUTER_API_KEY environment variable is set.", "💡".yellow());
            println!("{} Get your API key from: https://openrouter.ai/", "🔗".blue());
        }
    }
    Ok(())
}

async fn handle_workflow_command(action: &WorkflowCommands) -> Result<()> {
    // Initialize workflow orchestrator when needed
    let orchestrator = match WorkflowOrchestrator::new().await {
        Ok(orchestrator) => orchestrator,
        Err(e) => {
            println!("{} Failed to initialize workflow orchestrator: {}", "❌".red(), e);
            println!("{} Make sure OPENROUTER_API_KEY environment variable is set.", "💡".yellow());
            return Ok(());
        }
    };

    match action {
        WorkflowCommands::Start { description } => {
            println!("{} Starting new workflow for: {}", "🧠".bright_blue().bold(), description.bright_white());
            match orchestrator.start_workflow(description).await {
                Ok(workflow_id) => {
                    println!("{} Workflow created with ID: {}", "✅".green(), workflow_id.bright_white());
                    println!("{} Initial goals planned. Use 'cai workflow continue {}' to execute.", "💡".yellow(), workflow_id);
                    
                    // Show initial status
                    orchestrator.display_workflow_status(&workflow_id).await?;
                }
                Err(e) => {
                    println!("{} Failed to start workflow: {}", "❌".red(), e);
                }
            }
        }
        
        WorkflowCommands::Status => {
            let active_workflows = orchestrator.list_active_workflows().await?;
            
            println!("{} Active Workflows:", "📊".bright_blue().bold());
            if active_workflows.is_empty() {
                println!("  {} No active workflows", "💭".dimmed());
            } else {
                for workflow_id in active_workflows {
                    println!("  🧠 {}", workflow_id.bright_white());
                }
                println!("\n💡 Use 'cai workflow show <ID>' for detailed status");
            }
        }
        
        WorkflowCommands::Show { workflow_id } => {
            match orchestrator.display_workflow_status(workflow_id).await {
                Ok(_) => {},
                Err(e) => {
                    println!("{} Workflow '{}' not found: {}", "❌".red(), workflow_id, e);
                }
            }
        }
        
        WorkflowCommands::Continue { workflow_id } => {
            println!("{} Continuing workflow execution: {}", "⚡".yellow(), workflow_id.bright_white());
            
            // Execute workflow steps until completion or no more executable goals
            let mut steps_executed = 0;
            loop {
                match orchestrator.execute_next_goal(workflow_id).await {
                    Ok(true) => {
                        steps_executed += 1;
                        if steps_executed >= 10 {
                            println!("{} Executed {} steps. Use 'continue' again to proceed further.", "⏸️".yellow(), steps_executed);
                            break;
                        }
                    }
                    Ok(false) => {
                        println!("{} No more executable goals. Workflow may be complete.", "✅".green());
                        break;
                    }
                    Err(e) => {
                        println!("{} Error during execution: {}", "❌".red(), e);
                        break;
                    }
                }
            }
            
            // Show final status
            orchestrator.display_workflow_status(workflow_id).await?;
        }
        
        WorkflowCommands::Cleanup => {
            println!("{} Cleaning up completed workflows...", "🧹".yellow());
            // This would need implementation to identify completed workflows
            println!("{} Cleanup functionality not yet implemented", "💡".dimmed());
        }
    }
    
    Ok(())
}

async fn handle_mcp_command(action: &McpCommands) -> Result<()> {
    // For any MCP command except Init, ensure manager is initialized if config exists
    if !matches!(action, McpCommands::Init) {
        if let Err(e) = mcp_manager::ensure_initialized().await {
            log_error!("mcp","⚠️  MCP not initialized: {}", e);
        }
    }

    if let McpCommands::Init = action {
        let path = mcp_manager::init_default_config_file()?;
        println!("✅ Created default MCP configuration at: {}", path.display());
        println!("💡 Edit this file and run 'cai mcp start <server>' to launch.");
        return Ok(());
    }

    // Get the global MCP manager (may be None if no config exists)
    let global_manager = mcp_manager::get_mcp_manager();
    
    let guard = global_manager.lock().await;
    let manager = guard.as_ref()
        .ok_or_else(|| anyhow::anyhow!("MCP manager not available (no config found). Run 'cai mcp init' or add mcp-config.json"))?;

    match action {
        McpCommands::Init => {
            // Already handled above
            return Ok(());
        },
        McpCommands::List => {
            println!("{}", "Available MCP Servers:".bright_blue().bold());
            println!();
            
            let configured_servers = manager.list_configured_servers();
            let active_servers = manager.list_active_servers().await;
            
            for server_name in configured_servers {
                let is_running = active_servers.contains(server_name);
                let status = if is_running {
                    "🟢 Running".green()
                } else {
                    "🔴 Stopped".red()
                };
                
                println!("📁 {} - {}", server_name.bright_white().bold(), status);
                
                // If server is running, show detailed tool information
                if is_running {
                    match manager.get_detailed_tools(server_name).await {
                        Ok(tools) => {
                            if tools.is_empty() {
                                println!("  {} No tools available", "💭".dimmed());
                            } else {
                                println!("  🔧 {} tool(s) available:", tools.len().to_string().bright_white());
                                
                                for tool in tools {
                                    display_tool_details(&tool);
                                }
                            }
                        }
                        Err(e) => {
                            println!("  {} Failed to get tool details: {}", "⚠️".yellow(), e.to_string().dimmed());
                        }
                    }
                } else {
                    println!("  {} Start server to see available tools", "💡".dimmed());
                }
                
                println!(); // Add spacing between servers
            }
        },
        
        McpCommands::Start { server_name } => {
            println!("{} Starting MCP server: {}", "🚀".green(), server_name.bright_white());
            match manager.start_server(server_name).await {
                Ok(_) => println!("{} Server '{}' started successfully", "✅".green(), server_name),
                Err(e) => println!("{} Failed to start server '{}': {}", "❌".red(), server_name, e),
            }
        },
        
        McpCommands::Stop { server_name } => {
            println!("{} Stopping MCP server: {}", "🛑".red(), server_name.bright_white());
            match manager.stop_server(server_name).await {
                Ok(_) => println!("{} Server '{}' stopped successfully", "✅".green(), server_name),
                Err(e) => println!("{} Failed to stop server '{}': {}", "❌".red(), server_name, e),
            }
        },
        
        McpCommands::Tools { server_name } => {
            match manager.list_tools(server_name).await {
                Ok(tools) => {
                    println!("{} Tools available from server '{}':", "🔧".blue(), server_name.bright_white());
                    for tool in tools {
                        println!("  • {}", tool.cyan());
                    }
                },
                Err(e) => println!("{} Failed to list tools: {}", "❌".red(), e),
            }
        },
        
        McpCommands::Call { server_name, tool_name, args } => {
            let arguments = match args {
                Some(args_str) => serde_json::from_str(args_str)?,
                None => serde_json::Value::Object(serde_json::Map::new()),
            };
            
            match manager.call_tool(server_name, tool_name, arguments).await {
                Ok(result) => {
                    println!("{} Tool call result:", "🎯".green());
                    println!("{}", serde_json::to_string_pretty(&result)?);
                },
                Err(e) => println!("{} Failed to call tool: {}", "❌".red(), e),
            }
        },
        
        McpCommands::Resources { server_name } => {
            match manager.list_resources(server_name).await {
                Ok(resources) => {
                    println!("{} Resources available from server '{}':", "📁".blue(), server_name.bright_white());
                    for resource in resources {
                        println!("  • {}", resource.cyan());
                    }
                },
                Err(e) => println!("{} Failed to list resources: {}", "❌".red(), e),
            }
        },
        
        McpCommands::Status => {
            let active_servers = manager.list_active_servers().await;
            let configured_servers = manager.list_configured_servers();
            
            println!("{}", "MCP Server Status:".bright_blue().bold());
            println!("📊 Configured servers: {}", configured_servers.len());
            println!("🟢 Active servers: {}", active_servers.len());
            println!();
            
            if !active_servers.is_empty() {
                println!("{}", "Active servers:".bright_green().bold());
                for server in active_servers {
                    println!("  🟢 {}", server.bright_white());
                }
            }
        },
    }
    
    Ok(())
}

fn list_prompts(manager: &PromptManager) -> Result<()> {
    println!("{}", "Available Prompt Files:".bright_blue().bold());
    println!();

    for prompt_data in manager.list_all() {
        println!("📁 {}", prompt_data.file_name.bright_green().bold());
        println!("   {}", prompt_data.prompt_file.name); // human-friendly collection name
        println!("   {}", prompt_data.prompt_file.description.dimmed());
        println!("   📍 {}", prompt_data.file_path.dimmed());
        
        for subject in &prompt_data.prompt_file.subjects {
            println!("   └── 📂 {}", subject.name.yellow());
            for prompt in &subject.prompts {
                let score_display = if prompt.score > 0 {
                    format!(" (⭐ {})", prompt.score).bright_yellow()
                } else {
                    "".normal()
                };
                println!("       └── 📝 {}{}", prompt.title.cyan(), score_display);
            }
        }
        println!();
    }

    Ok(())
}

fn search_prompts(manager: &PromptManager, query: &str, resolve_urls: bool) -> Result<()> {
    let search_start = Instant::now();
    log_debug!("search", "🔍 Starting search for query: '{}'", query);
    
    let results = manager.search(query, resolve_urls);
    let search_duration = search_start.elapsed().as_millis() as u64;
    ops::performance("SEARCH", search_duration);
    ops::search_operation(query, results.len());

    if results.is_empty() {
        log_info!("search", "❌ No results found for query: '{}'", query);
        println!("{} No results found for '{}'", "❌".red(), query.bright_white());
        return Ok(());
    }

    log_info!("search", "✅ Found {} result(s) for query: '{}'", results.len(), query);
    println!("{} Found {} result(s) for '{}':", 
        "🔍".green(), 
        results.len().to_string().bright_white().bold(), 
        query.bright_white()
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        log_debug!("search", "📄 Result {}: {} in {}", i + 1, 
            result.match_type, result.file_name);
        print_search_result(&result);
        println!();
    }

    Ok(())
}

async fn show_prompt_file(manager: &PromptManager, file_name: &str) -> Result<()> {
    log_debug!("show", "👁️ Looking for prompt file: '{}'", file_name);
    
    if let Some(prompt_data) = manager.get_by_file_name(file_name) {
        let prompt_count: usize = prompt_data.prompt_file.subjects.iter()
            .map(|s| s.prompts.len()).sum();
        log_info!("show", "✅ Found file '{}' with {} subjects and {} prompts", 
            file_name, prompt_data.prompt_file.subjects.len(), prompt_count);
        
        ops::file_operation("SHOW", &prompt_data.file_path, true);
        
        println!("📁 {}", prompt_data.prompt_file.name.bright_green().bold());
        println!("{}", prompt_data.prompt_file.description.dimmed());
        println!("📍 {}", prompt_data.file_path.dimmed());
        println!();

        for subject in &prompt_data.prompt_file.subjects {
            log_debug!("show", "📂 Processing subject: '{}' with {} prompts", 
                subject.name, subject.prompts.len());
            
            println!("📂 {}", subject.name.yellow().bold());
            for prompt in &subject.prompts {
                let score_display = if prompt.score > 0 {
                    format!(" (⭐ {})", prompt.score).bright_yellow()
                } else {
                    "".normal()
                };
                println!("  📝 {}{}", prompt.title.cyan().bold(), score_display);
                
                if prompt.is_url_reference() {
                    log_debug!("show", "🔗 Loading URL reference: {}", prompt.content);
                    println!("     {} {}", "🔗".dimmed(), prompt.content.dimmed());
                    
                    let load_start = Instant::now();
                    match prompt.get_resolved_content().await {
                        Ok(content) => {
                            let load_duration = load_start.elapsed().as_millis() as u64;
                            ops::performance("URL_CONTENT_LOAD", load_duration);
                            
                            let truncated = if content.len() > 200 {
                                format!("{}...", &content[..200])
                            } else {
                                content
                            };
                            println!("     {}", truncated.dimmed());
                            log_debug!("show", "✅ Successfully loaded URL content ({} chars)", 
                                truncated.len());
                        }
                        Err(e) => {
                            ops::error_with_context("URL_LOAD", &e.to_string(), 
                                Some(&format!("Failed to load: {}", prompt.content)));
                            println!("     {} Failed to load: {}", "❌".red(), e.to_string().red());
                        }
                    }
                } else {
                    println!("     {}", prompt.content.dimmed());
                }
                println!();
            }
        }
    } else {
        log_info!("show", "❌ File '{}' not found", file_name);
        ops::file_operation("SHOW", file_name, false);
        println!("{} File '{}' not found", "❌".red(), file_name.bright_white());
    }

    Ok(())
}

async fn query_prompt(manager: &PromptManager, file: &str, subject: &str, prompt: &str) -> Result<()> {
    if let Some(prompt_data) = manager.get_by_file_name(file) {
        if let Some(subject_data) = prompt_data.prompt_file.subjects.iter().find(|s| s.name == subject) {
            if let Some(prompt_data) = subject_data.prompts.iter().find(|p| p.title == prompt) {
                println!("📝 {}", prompt_data.title.cyan().bold());
                println!("📁 {} → 📂 {}", file.green(), subject.yellow());
                // Plain line for test-friendly matching without icons/colors
                println!("{} → {}", file, subject);
                
                if prompt_data.is_url_reference() {
                    println!("🔗 {}", prompt_data.content.dimmed());
                }
                
                println!();
                
                match prompt_data.get_resolved_content().await {
                    Ok(content) => println!("{}", content),
                    Err(e) => {
                        println!("{} Failed to load content: {}", "❌".red(), e.to_string().red());
                        if prompt_data.is_url_reference() {
                            println!("URL: {}", prompt_data.content);
                        }
                    }
                }
            } else {
                println!("{} Prompt '{}' not found in subject '{}'", "❌".red(), prompt.bright_white(), subject.bright_white());
            }
        } else {
            println!("{} Subject '{}' not found in file '{}'", "❌".red(), subject.bright_white(), file.bright_white());
        }
    } else {
        println!("{} File '{}' not found", "❌".red(), file.bright_white());
    }

    Ok(())
}

/// Display detailed information about an MCP tool
fn display_tool_details(tool: &serde_json::Value) {
    // Extract basic tool information
    let name = tool.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown");
    
    let description = tool.get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("No description available");

    println!("    🔧 {}", name.bright_cyan().bold());
    println!("       {}", description.dimmed());

    // Display input schema if available
    if let Some(input_schema) = tool.get("inputSchema") {
        display_schema("Parameters", input_schema);
    }
}

/// Display JSON schema information for tool parameters
fn display_schema(label: &str, schema: &serde_json::Value) {
    if let Some(schema_obj) = schema.as_object() {
        // Check if there are properties to display
        if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
            if !properties.is_empty() {
                println!("       📋 {}:", label.bright_blue());
                
                // Get required fields
                let required_fields: std::collections::HashSet<String> = schema_obj
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect())
                    .unwrap_or_default();

                for (param_name, param_schema) in properties {
                    let is_required = required_fields.contains(param_name);
                    let required_indicator = if is_required { 
                        " (required)".red() 
                    } else { 
                        " (optional)".dimmed() 
                    };

                    let param_type = param_schema.get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");

                    let param_description = param_schema.get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");

                    println!("         • {} ({}){}", 
                        param_name.yellow(), 
                        param_type.green(),
                        required_indicator
                    );

                    if !param_description.is_empty() {
                        println!("           {}", param_description.dimmed());
                    }

                    // Show enum values if present
                    if let Some(enum_values) = param_schema.get("enum").and_then(|e| e.as_array()) {
                        let values: Vec<String> = enum_values.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        if !values.is_empty() {
                            println!("           Allowed values: {}", values.join(", ").bright_blue());
                        }
                    }
                }
            }
        }
    }
}

fn print_search_result(result: &SearchResult) {
    let match_icon = match result.match_type {
        MatchType::FileName => "📁",
        MatchType::FileDescription => "📄",
        MatchType::SubjectName => "📂",
        MatchType::PromptTitle => "📝",
        MatchType::PromptContent => "💬",
    };

    let match_type_str = format!("{:?}", result.match_type).to_lowercase().replace('_', " ");
    
    println!("{} {} in {}", 
        match_icon, 
        match_type_str.bright_blue(), 
        result.file_name.green().bold()
    );

    if let Some(subject) = &result.subject_name {
        println!("   📂 Subject: {}", subject.yellow());
    }

    if let Some(title) = &result.prompt_title {
        println!("   📝 Prompt: {}", title.cyan());
    }

    if let Some(content) = &result.prompt_content {
        let truncated = if content.len() > 100 {
            format!("{}...", &content[..100])
        } else {
            content.clone()
        };
        println!("   💬 {}", truncated.dimmed());
    }
}
