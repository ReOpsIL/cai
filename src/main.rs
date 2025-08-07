mod prompt_loader;
mod openrouter_client;
mod chat_interface;
mod logger;
mod mcp_config;
mod mcp_client;
mod mcp_manager;
mod task_executor;
mod feedback_loop;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use prompt_loader::{MatchType, PromptManager, SearchResult};
use chat_interface::ChatInterface;
use logger::ops;
use task_executor::TaskExecutor;
use std::path::PathBuf;
use std::time::Instant;

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
    Chat,
    /// MCP (Model Context Protocol) tools management
    Mcp {
        #[command(subcommand)]
        action: McpCommands,
    },
    /// Test task execution system with demo tasks
    TaskDemo,
}

#[derive(Subcommand)]
enum McpCommands {
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    logger::init();
    ops::startup("APP", "starting CAI application");

    // Initialize MCP servers
    if let Err(e) = mcp_manager::initialize_mcp().await {
        eprintln!("⚠️  Failed to initialize MCP servers: {}", e);
        eprintln!("💡 Application will continue without MCP support");
    }

    // Set up graceful shutdown
    let _shutdown_result = setup_shutdown_handler();

    let start_time = Instant::now();
    
    let cli = Cli::parse();
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
        Commands::Search { query } => {
            log_info!("main", "🔍 Executing SEARCH command with query: '{}'", query);
            search_prompts(&manager, &query)
        },
        Commands::Show { file_name } => {
            log_info!("main", "👁️ Executing SHOW command for file: '{}'", file_name);
            show_prompt_file(&manager, &file_name).await
        },
        Commands::Query { file, subject, prompt } => {
            log_info!("main", "❓ Executing QUERY command: {} → {} → {}", file, subject, prompt);
            query_prompt(&manager, &file, &subject, &prompt).await
        },
        Commands::Chat => {
            log_info!("main", "💬 Executing CHAT command");
            start_chat_mode(&mut manager).await
        },
        Commands::Mcp { action } => {
            log_info!("main", "🔧 Executing MCP command");
            handle_mcp_command(action).await
        },
        Commands::TaskDemo => {
            log_info!("main", "🚀 Running task demo");
            run_task_demo().await
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
        eprintln!("⚠️  Error during MCP shutdown: {}", e);
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
            eprintln!("⚠️  Error during graceful MCP shutdown: {}", e);
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

async fn start_chat_mode(manager: &mut PromptManager) -> Result<()> {
    match ChatInterface::new().await {
        Ok(mut chat) => {
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

async fn handle_mcp_command(action: &McpCommands) -> Result<()> {
    // Get the global MCP manager
    let global_manager = mcp_manager::get_mcp_manager();
    
    let guard = global_manager.lock().await;
    let manager = guard.as_ref()
        .ok_or_else(|| anyhow::anyhow!("MCP manager not available - try running 'cai mcp status' to check initialization"))?;

    match action {
        McpCommands::List => {
            println!("{}", "Available MCP Servers:".bright_blue().bold());
            println!();
            
            let configured_servers = manager.list_configured_servers();
            let active_servers = manager.list_active_servers().await;
            
            for server_name in configured_servers {
                let status = if active_servers.contains(server_name) {
                    "🟢 Running".green()
                } else {
                    "🔴 Stopped".red()
                };
                
                println!("📁 {} - {}", server_name.bright_white().bold(), status);
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

fn search_prompts(manager: &PromptManager, query: &str) -> Result<()> {
    let search_start = Instant::now();
    log_debug!("search", "🔍 Starting search for query: '{}'", query);
    
    let results = manager.search(query);
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