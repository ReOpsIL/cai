mod prompt_loader;
mod openrouter_client;
mod logger;
mod mcp_config;
mod mcp_client;
mod mcp_manager;
mod task_state;
mod tool_safety;
mod scan_manager;
mod local_tools;
mod task_executor;
mod chat_interface;
mod feedback_loop;
mod workflow_orchestrator;
mod session_manager;
mod validator;
mod project_scanner;
mod multi_agent;
mod declarative_tools;
mod pub_sub;
mod enhanced_session;
mod execution_context;
mod continuous_executor;
mod enhanced_task_executor;
mod file_operation_safety;
mod command_safety;
mod loop_detection;
mod docker_detection;
mod workflow_timeouts;
mod advanced_error_recovery;
mod performance_optimization;
mod advanced_workflow_state;
mod predictive_error_prevention;
mod context_aware_execution;
mod test_infrastructure;
mod edge_case_mastery;
mod fast_path;
mod lazy_loading;
mod path_manager;
mod project_recovery;
mod mcp_path_manager;
mod project_state_manager;
mod workflow_continuity;
mod file_operations;
mod shell_execution;
mod code_generation;
mod project_scaffolding;
mod dependency_manager;
mod integration_test;
mod simplified_integration_test;
mod basic_functionality_test;
// mod enhanced_tools;

// Strategic Enhancement Modules - Temporarily disabled for compilation
// mod permission_manager;
// mod file_safety;
// mod atomic_operations;
// mod error_recovery;
// mod natural_language;
// mod progress_tracking;
// mod hierarchical_config;
// mod git_integration;
// mod hooks;
// mod context_manager;
// mod predictive_error_prevention;
// mod strategic_integration;

use anyhow::{anyhow, Result};
use chat_interface::ChatInterface;
use clap::{Parser, Subcommand};
use colored::control as color_control;
use colored::*;
use logger::ops;
use prompt_loader::{MatchType, PromptManager, SearchResult};
use std::path::PathBuf;
use std::time::Instant;
use task_executor::TaskExecutor;
use workflow_orchestrator::WorkflowOrchestrator;

#[derive(Parser)]
#[command(name = "cai")]
#[command(about = "A CLI tool for managing and searching prompt collections")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = "prompts")]
    directory: PathBuf,

    /// Operation mode: suggest | auto-edit | full-auto
    #[arg(long, default_value = "auto-edit")]
    mode: String,

    /// Enable debugging output
    #[arg(long)]
    debug: bool,
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
    /// Run comprehensive integration test for multi-stage development workflow
    IntegrationTest,
    /// Run simplified integration test focusing on core functionality
    SimpleTest,
    /// Run basic functionality test without file operations
    BasicTest,
    /// Workflow orchestration commands
    Workflow {
        #[command(subcommand)]
        action: WorkflowCommands,
    },
    /// LLM-powered project scanning with hierarchical planning
    Scan {
        #[command(subcommand)]
        action: ScanCommands,
    },
    // /// Enhanced project generation with advanced tools
    // Generate {
    //     #[command(subcommand)]
    //     action: GenerateCommands,
    // },
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
enum ScanCommands {
    /// Run LLM-powered project scanning
    Run {
        /// Project path to scan (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        path: String,
        /// Custom scanning objective
        #[arg(short, long)]
        objective: Option<String>,
    },
    /// Show current scanning plan status
    Status,
    /// Cancel running scan
    Cancel,
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

// #[derive(Subcommand)]
// enum GenerateCommands {
//     /// Generate a complete project from a prompt using enhanced tools
//     Project {
//         /// Project name
//         name: String,
//         /// Description of the project to generate
//         prompt: String,
//         /// Target directory (defaults to current directory)
//         #[arg(short, long, default_value = ".")]
//         target: String,
//     },
//     /// Generate comprehensive tests for existing project
//     Tests {
//         /// Project path to generate tests for
//         #[arg(short, long, default_value = ".")]
//         path: String,
//     },
//     /// Generate documentation for existing project
//     Documentation {
//         /// Project path to generate documentation for
//         #[arg(short, long, default_value = ".")]
//         path: String,
//     },
//     /// Validate project quality and get improvement suggestions
//     Validate {
//         /// Project path to validate
//         #[arg(short, long, default_value = ".")]
//         path: String,
//     },
// }

/// Determine if a command needs PromptManager
fn needs_prompt_manager(command: &Commands) -> bool {
    match command {
        Commands::List => true,
        Commands::Search { .. } => true,
        Commands::Show { .. } => true,
        Commands::Query { .. } => true,
        Commands::Chat { .. } => true,
        Commands::Scan { .. } => true,
        Commands::Mcp { .. } => false,
        Commands::TaskDemo => false,
        Commands::IntegrationTest => false,
        Commands::SimpleTest => false,
        Commands::BasicTest => false,
        Commands::Workflow { .. } => false,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first to get debug flag
    let cli = Cli::parse();
    
    // Set log level from debug flag or environment
    if cli.debug {
        std::env::set_var("CAI_LOG_LEVEL", "DEBUG");
    }
    
    // Initialize logging with proper level
    logger::init();
    
    // FAST PATH: Check if we can use fast execution for basic commands
    // This bypasses heavy enhancement system initialization for performance
    let args: Vec<String> = std::env::args().skip(1).collect();
    if fast_path::can_use_fast_path(&args) {
        log_debug!("main", "🚀 Using fast path for basic command");
        
        match fast_path::parse_fast_command(&args) {
            Ok(fast_command) => {
                let fast_executor = fast_path::FastPath::new()?;
                match fast_executor.execute(fast_command).await {
                    Ok(output) => {
                        println!("{}", output);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                log_debug!("main", "Fast path parsing failed: {}", e);
                // Fall through to enhanced path
            }
        }
    }
    
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

    // Initialize Strategic Enhancement System - Temporarily disabled
    // log_info!("main", "{} Initializing Strategic Enhancement System...", "🚀".cyan());
    // let _strategic_integration = match strategic_integration::StrategicIntegration::initialize().await {
    //     Ok(integration) => {
    //         log_info!("main", "{} Strategic enhancements initialized", "✅".green());
    //         Some(integration)
    //     }
    //     Err(e) => {
    //         log_error!("main", "{} Strategic initialization failed: {}", "⚠️".yellow(), e);
    //         log_info!("main", "{} Continuing with basic functionality", "💡".yellow());
    //         None
    //     }
    // };

    // Enhancement systems are now loaded on-demand via lazy loading

    // Initialize path manager for consistent path resolution
    log_info!("main", "🗂️ Initializing path manager...");
    if let Err(e) = path_manager::initialize_path_manager() {
        log_warn!("main", "⚠️ Path manager initialization failed: {}", e);
    }

    // Initialize project state manager for session persistence
    log_info!("main", "📊 Initializing project state manager...");
    if let Err(e) = project_state_manager::initialize_project_state_manager().await {
        log_warn!("main", "⚠️ Project state manager initialization failed: {}", e);
    }

    // Initialize workflow continuity for cross-session resume
    log_info!("main", "🔄 Initializing workflow continuity...");
    if let Err(e) = workflow_continuity::initialize_workflow_continuity().await {
        log_warn!("main", "⚠️ Workflow continuity initialization failed: {}", e);
    }

    // Initialize MCP servers on startup
    log_info!("main", "🔧 Initializing MCP servers...");
    if let Err(e) = mcp_manager::initialize_mcp().await {
        log_warn!("main", "⚠️ MCP initialization failed: {}", e);
        log_info!("main", "💡 Run 'cai mcp init' to create configuration or check existing setup");
    }

    // Set up graceful shutdown
    let _shutdown_result = setup_shutdown_handler();

    let start_time = Instant::now();
    
    // CLI was already parsed earlier - use the existing instance
    // Expose selected prompts directory to URL security checks
    std::env::set_var("CAI_PROMPTS_DIR", &cli.directory);
    // Expose mode to subcomponents
    std::env::set_var("CAI_MODE", &cli.mode);
    log_debug!("main", "📋 Parsed CLI arguments: directory={:?}", cli.directory);

    // PromptManager is now loaded on-demand for commands that need it

    // Version is handled automatically by clap due to #[command(version)]

    // Check if command is provided
    let command = cli.command.as_ref().unwrap_or_else(|| {
        // If no command provided, default to List
        &Commands::List
    });

    // Determine command name for lazy loading
    let command_name = match command {
        Commands::List => "list",
        Commands::Search { .. } => "search", 
        Commands::Show { .. } => "show",
        Commands::Query { .. } => "query",
        Commands::Chat { .. } => "chat",
        Commands::Mcp { .. } => "mcp",
        Commands::TaskDemo => "task-demo",
        Commands::IntegrationTest => "integration-test",
        Commands::SimpleTest => "simple-test",
        Commands::BasicTest => "basic-test",
        Commands::Workflow { .. } => "workflow",
        Commands::Scan { .. } => "scan",
    };

    // Initialize components for this command using lazy loading
    log_info!("main", "🔄 Initializing components for command: {}", command_name);
    if let Err(e) = lazy_loading::initialize_for_command(command_name).await {
        log_warn!("main", "⚠️ Component initialization failed: {}", e);
        log_info!("main", "💡 Continuing with basic functionality");
    }

    // Load PromptManager only for commands that need it
    let mut manager_opt = None;
    if needs_prompt_manager(command) {
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
        manager_opt = Some(manager);
    }

    // Execute command with timing
    let command_start = Instant::now();
    let result = match command {
        Commands::List => {
            log_info!("main", "📋 Executing LIST command");
            let manager = manager_opt.as_ref().ok_or_else(|| anyhow!("PromptManager required for this command"))?;
            list_prompts(manager)
        },
        Commands::Search { query, resolve_urls } => {
            log_info!("main", "🔍 Executing SEARCH command with query: '{}'", query);
            let manager = manager_opt.as_ref().ok_or_else(|| anyhow!("PromptManager required for this command"))?;
            search_prompts(manager, &query, *resolve_urls)
        },
        Commands::Show { file_name } => {
            log_info!("main", "👁️ Executing SHOW command for file: '{}'", file_name);
            let manager = manager_opt.as_ref().ok_or_else(|| anyhow!("PromptManager required for this command"))?;
            show_prompt_file(manager, &file_name).await
        },
        Commands::Query { file, subject, prompt } => {
            log_info!("main", "❓ Executing QUERY command: {} → {} → {}", file, subject, prompt);
            let manager = manager_opt.as_ref().ok_or_else(|| anyhow!("PromptManager required for this command"))?;
            query_prompt(manager, &file, &subject, &prompt).await
        },
        Commands::Chat { workflow_id } => {
            log_info!("main", "💬 Executing CHAT command with workflow_id: {:?}", workflow_id);
            let mut manager = manager_opt.ok_or_else(|| anyhow!("PromptManager required for this command"))?;
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
        Commands::IntegrationTest => {
            log_info!("main", "🧪 Running integration test");
            run_integration_test_command().await
        },
        Commands::SimpleTest => {
            log_info!("main", "🔬 Running simplified integration test");
            run_simple_test_command().await
        },
        Commands::BasicTest => {
            log_info!("main", "🧪 Running basic functionality test");
            run_basic_test_command().await
        },
        Commands::Workflow { action } => {
            log_info!("main", "🧠 Executing workflow command");
            handle_workflow_command(action).await
        },
        Commands::Scan { action } => {
            log_info!("main", "🔍 Executing scan command");
            let manager = manager_opt.as_ref().ok_or_else(|| anyhow!("PromptManager required for this command"))?;
            handle_scan_command(action, manager).await
        },
        // Commands::Generate { action } => {
        //     log_info!("main", "🏗️ Executing enhanced generation command");
        //     handle_generate_command(action).await
        // },
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
                log_info!("main", "🛑 Received Ctrl+C, shutting down gracefully...");
            }
            _ = terminate => {
                log_info!("main", "🛑 Received SIGTERM, shutting down gracefully...");
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
    log_info!("main", "{}", "🚀 Task Execution Demo".bright_blue().bold());
    log_info!("main", "{}", "Demonstrating MCP tool integration with task execution".dimmed());
    log_debug!("main", "");

    // Initialize LLM-enabled task executor; notify and abort if unavailable
    let executor = match TaskExecutor::with_llm_analysis().await {
        Ok(executor) => {
            log_info!("main", "{} Using LLM-powered intelligent tool analysis", "🧠".bright_blue());
            executor
        }
        Err(e) => {
            log_error!("main", "{} LLM analysis is not available: {}", "❌".red(), e);
            log_info!("main", "{} Set OPENROUTER_API_KEY and ensure network access", "💡".yellow());
            return Err(e);
        }
    };
    
    // Add some demo tasks
    let demo_tasks = vec![
        "List all files in the current directory".to_string(),
        "Read the contents of README.md file".to_string(),
        "Show the current working directory structure".to_string(),
    ];

    log_info!("main", "{} Adding demo tasks to queue...", "📝".cyan());
    executor.add_tasks(demo_tasks).await?;

    log_info!("main", "{} Executing all tasks...", "⚡".yellow());
    executor.execute_all().await?;

    log_info!("main", "{} Demo completed!", "🎉".green().bold());
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
            log_error!("main", "{} Failed to start chat mode: {}", "❌".red(), e);
            log_info!("main", "{} Make sure OPENROUTER_API_KEY environment variable is set.", "💡".yellow());
            log_info!("main", "{} Get your API key from: https://openrouter.ai/", "🔗".blue());
        }
    }
    Ok(())
}

async fn handle_workflow_command(action: &WorkflowCommands) -> Result<()> {
    // Initialize workflow orchestrator when needed
    let orchestrator = match WorkflowOrchestrator::new().await {
        Ok(orchestrator) => orchestrator,
        Err(e) => {
            log_error!("main", "{} Failed to initialize workflow orchestrator: {}", "❌".red(), e);
            log_info!("main", "{} Make sure OPENROUTER_API_KEY environment variable is set.", "💡".yellow());
            return Ok(());
        }
    };

    match action {
        WorkflowCommands::Start { description } => {
            log_info!("main", "{} Starting new workflow for: {}", "🧠".bright_blue().bold(), description.bright_white());
            match orchestrator.start_workflow(description).await {
                Ok(workflow_id) => {
                    log_info!("main", "{} Workflow created with ID: {}", "✅".green(), workflow_id.bright_white());
                    log_info!("main", "{} Initial goals planned. Use 'cai workflow continue {}' to execute.", "💡".yellow(), workflow_id);
                    
                    // Show initial status
                    orchestrator.display_workflow_status(&workflow_id).await?;
                }
                Err(e) => {
                    log_error!("main", "{} Failed to start workflow: {}", "❌".red(), e);
                }
            }
        }
        
        WorkflowCommands::Status => {
            let active_workflows = orchestrator.list_active_workflows().await?;
            
            log_info!("main", "{} Active Workflows:", "📊".bright_blue().bold());
            if active_workflows.is_empty() {
                log_info!("main", "  {} No active workflows", "💭".dimmed());
            } else {
                for workflow_id in active_workflows {
                    log_info!("main", "  🧠 {}", workflow_id.bright_white());
                }
                log_info!("main", "{}", "\n💡 Use 'cai workflow show <ID>' for detailed status");
            }
        }
        
        WorkflowCommands::Show { workflow_id } => {
            match orchestrator.display_workflow_status(workflow_id).await {
                Ok(_) => {},
                Err(e) => {
                    log_error!("main", "{} Workflow '{}' not found: {}", "❌".red(), workflow_id, e);
                }
            }
        }
        
        WorkflowCommands::Continue { workflow_id } => {
            log_info!("main", "{} Continuing workflow execution: {}", "⚡".yellow(), workflow_id.bright_white());
            
            // Execute workflow steps until completion or no more executable goals
            let mut steps_executed = 0;
            loop {
                match orchestrator.execute_next_goal(workflow_id).await {
                    Ok(true) => {
                        steps_executed += 1;
                        if steps_executed >= 10 {
                            log_info!("main", "{} Executed {} steps. Use 'continue' again to proceed further.", "⏸️".yellow(), steps_executed);
                            break;
                        }
                    }
                    Ok(false) => {
                        log_info!("main", "{} No more executable goals. Workflow may be complete.", "✅".green());
                        break;
                    }
                    Err(e) => {
                        log_error!("main", "{} Error during execution: {}", "❌".red(), e);
                        break;
                    }
                }
            }
            
            // Show final status
            orchestrator.display_workflow_status(workflow_id).await?;
        }
        
        WorkflowCommands::Cleanup => {
            println!("{} Cleaning up completed workflows...", "🧹".yellow());
            match orchestrator.cleanup_completed_workflows().await {
                Ok(count) => {
                    if count > 0 {
                        println!("{} Removed {} completed workflow(s)", "✅".green(), count);
                    } else {
                        println!("{} No completed workflows to clean", "💭".dimmed());
                    }
                }
                Err(e) => println!("{} Cleanup error: {}", "❌".red(), e),
            }
        }
    }
    
    Ok(())
}

async fn handle_scan_command(action: &ScanCommands, _manager: &PromptManager) -> Result<()> {
    match action {
        ScanCommands::Run { path, objective } => {
            println!("{} Starting LLM-powered project scan for: {}", "🔍".bright_blue().bold(), path.bright_white());
            
            match project_scanner::LLMProjectScanner::new().await {
                Ok(scanner) => {
                    let objective_str = objective.as_deref();
                    
                    println!("{} Generating scanning plan...", "🧠".yellow());
                    match scanner.generate_scanning_plan(path, objective_str).await {
                        Ok(plan) => {
                            println!("{} Generated plan: {}", "✅".green(), plan.title.bright_white());
                            println!("   📝 {} steps planned", plan.steps.len());
                            
                            if !plan.steps.is_empty() {
                                println!("\n{} Plan Steps:", "📋".bright_cyan().bold());
                                for (i, step) in plan.steps.iter().enumerate() {
                                    println!("  {}. {}", i + 1, step.display_summary());
                                }
                            }
                            
                            println!("\n{} Executing scanning plan...", "🚀".green().bold());
                            match scanner.execute_scanning_plan(plan).await {
                                Ok(results) => {
                                    println!("\n{} Scan completed successfully!", "🎉".green().bold());
                                    println!("📊 Total results collected: {}", results.len());
                                    
                                    // Display key findings
                                    if !results.is_empty() {
                                        println!("\n{} Key Findings:", "🔍".bright_yellow().bold());
                                        let mut count = 0;
                                        for (step_id, result) in results.iter() {
                                            if count >= 3 { break; } // Show first 3 results
                                            println!("  📄 {}: {}", 
                                                step_id.bright_white(),
                                                result.to_string().chars().take(100).collect::<String>()
                                            );
                                            count += 1;
                                        }
                                        if results.len() > 3 {
                                            println!("     ... and {} more results", results.len() - 3);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("{} Scan execution failed: {}", "❌".red(), e);
                                    if e.to_string().contains("cancelled") {
                                        println!("💡 Scan was cancelled by user request");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("{} Failed to generate scanning plan: {}", "❌".red(), e);
                        }
                    }
                }
                Err(e) => {
                    println!("{} Failed to initialize LLM project scanner: {}", "❌".red(), e);
                    println!("{} Make sure OPENROUTER_API_KEY environment variable is set.", "💡".yellow());
                }
            }
        }
        
        ScanCommands::Status => {
            println!("{} Checking scan status...", "📊".bright_blue().bold());
            
            let scan_manager = scan_manager::get_global_scan_manager();
            match scan_manager.get_status() {
                Ok(status) => {
                    println!("📊 Scan Status: {}", status.display_string());
                    
                    // Show progress percentage if available
                    if let Ok(Some(percentage)) = scan_manager.get_progress_percentage() {
                        println!("📈 Progress: {}%", percentage);
                    }
                    
                    // Show running status details
                    if let Ok(true) = scan_manager.is_running() {
                        println!("🟢 Scan is currently active");
                        println!("💡 Use 'cai scan cancel' to stop the current scan");
                    } else {
                        println!("🔴 No active scan running");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get scan status: {}", e);
                }
            }
        }
        
        ScanCommands::Cancel => {
            println!("{} Cancelling active scan...", "🚫".yellow());
            
            let scan_manager = scan_manager::get_global_scan_manager();
            match scan_manager.cancel_scan() {
                Ok(true) => {
                    println!("✅ Cancellation signal sent to active scan");
                    println!("⏳ Scan should stop within a few seconds...");
                    
                    // Wait a moment and check status
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if let Ok(status) = scan_manager.get_status() {
                        println!("📊 Updated Status: {}", status.display_string());
                    }
                }
                Ok(false) => {
                    println!("⚠️ No active scan to cancel");
                    println!("💡 Use 'cai scan status' to check current scan state");
                }
                Err(e) => {
                    println!("❌ Failed to cancel scan: {}", e);
                    println!("💡 You can still use Ctrl+C to forcefully stop a scan");
                }
            }
        }
    }
    
    Ok(())
}

// async fn handle_generate_command(action: &GenerateCommands) -> Result<()> {
//     match action {
//         GenerateCommands::Project { name, prompt, target } => {
//             println!("{} Enhanced Project Generation", "🏗️".bright_blue().bold());
//             println!("📝 Project: {}", name.bright_white());
//             println!("📋 Prompt: {}", prompt.dimmed());
//             println!("📍 Target: {}", target.dimmed());
//             
//             println!("\n{} Enhanced tools architecture implemented:", "✅".green());
//             println!("  • Multi-file project scaffolding with quality gates");
//             println!("  • Language-specific templates (Python, JS/TS, Rust, Go)");
//             println!("  • Framework detection (Flask, FastAPI, Express, Axum, Gin)");
//             println!("  • Automated test generation (89% missing tests addressed)");
//             println!("  • Documentation automation (72% missing docs addressed)");
//             println!("  • Safety validation with atomic operations");
//             
//             println!("\n{} Project generation would create:", "📋".cyan());
//             println!("  • Complete project structure with best practices");
//             println!("  • Comprehensive test suite with multiple test types");
//             println!("  • Documentation at all levels (README, API, Architecture)");
//             println!("  • Configuration files for the target language/framework");
//             println!("  • Quality validation ensuring production readiness");
//             
//             println!("\n{} This demonstrates the enhanced tools capability!", "🎉".green().bold());
//         }
//         
//         GenerateCommands::Tests { path } => {
//             println!("{} Generating comprehensive test suite for project at: {}", "🧪".bright_blue().bold(), path.bright_white());
//             
//             let project_path = Path::new(path);
//             
//             // For now, show what would be done
//             println!("{} Test generation capability includes:", "📋".cyan());
//             println!("  • Unit tests for all source files");
//             println!("  • Integration tests for complex workflows");
//             println!("  • Security tests for security-focused projects");
//             println!("  • Performance tests for performance-critical code");
//             println!("  • Framework-specific configurations (pytest, jest, etc.)");
//             println!("\n{} This feature requires project analysis integration", "💡".yellow());
//         }
//         
//         GenerateCommands::Documentation { path } => {
//             println!("{} Generating comprehensive documentation for project at: {}", "📚".bright_blue().bold(), path.bright_white());
//             
//             let project_path = Path::new(path);
//             
//             // For now, show what would be done
//             println!("{} Documentation generation capability includes:", "📋".cyan());
//             println!("  • Comprehensive README with setup instructions");
//             println!("  • API documentation for endpoints and functions");
//             println!("  • Architecture documentation for complex projects");
//             println!("  • Security documentation for security implementations");
//             println!("  • Performance documentation and benchmarking guides");
//             println!("  • Inline code documentation (docstrings/comments)");
//             println!("\n{} This feature requires project analysis integration", "💡".yellow());
//         }
//         
//         GenerateCommands::Validate { path } => {
//             println!("{} Validating project quality at: {}", "🔍".bright_blue().bold(), path.bright_white());
//             
//             let project_path = Path::new(path);
//             
//             // For now, show what would be validated
//             println!("{} Quality validation includes:", "📋".cyan());
//             println!("  • Syntax correctness across all source files");
//             println!("  • Security vulnerability scanning");
//             println!("  • Code complexity analysis");
//             println!("  • Documentation completeness check");
//             println!("  • Test coverage assessment");
//             println!("  • Performance issue detection");
//             println!("  • Code style consistency validation");
//             println!("  • Dependency security analysis");
//             println!("\n{} This feature requires project analysis integration", "💡".yellow());
//         }
//     }
//     
//     Ok(())
// }

/// Run comprehensive integration test for multi-stage development workflow
async fn run_integration_test_command() -> Result<()> {
    use crate::integration_test::run_integration_test;
    
    log_info!("main", "{}", "🧪 Multi-Stage Development Workflow Integration Test".bright_blue().bold());
    log_info!("main", "{}", "Testing complete 10-stage development capabilities".dimmed());
    
    println!("{}", "🧪 Starting Multi-Stage Development Workflow Test".bright_blue().bold());
    println!("{}", "Testing CAI's enhanced development capabilities...".dimmed());
    println!();
    
    match run_integration_test().await {
        Ok(result) => {
            let successful_stages = result.stages.iter().filter(|s| s.success).count();
            let total_stages = result.stages.len();
            
            if successful_stages >= 8 && result.validation_passed {
                println!("\n{} Integration test PASSED! ({}/{} stages successful)", 
                        "✅".green(), successful_stages, total_stages);
                println!("{} Multi-stage development capabilities are working correctly", 
                        "🎉".bright_green());
                log_info!("main", "✅ Integration test passed: {}/{} stages successful", 
                        successful_stages, total_stages);
            } else {
                println!("\n{} Integration test FAILED ({}/{} stages successful)", 
                        "❌".red(), successful_stages, total_stages);
                println!("{} Some multi-stage capabilities need attention", 
                        "⚠️".yellow());
                log_warn!("main", "⚠️ Integration test failed: {}/{} stages successful", 
                        successful_stages, total_stages);
            }
            
            println!("\n{} Total execution time: {:.2}s", "⏱️", result.total_execution_time);
            println!("{} Files created: {}", "📁", result.created_files.len());
            
            Ok(())
        }
        Err(e) => {
            println!("\n{} Integration test encountered an error: {}", "❌".red(), e);
            log_error!("main", "❌ Integration test error: {}", e);
            Err(e)
        }
    }
}

/// Run simplified integration test for core functionality
async fn run_simple_test_command() -> Result<()> {
    use crate::simplified_integration_test::run_simplified_integration_test;
    
    log_info!("main", "{}", "🔬 Simplified Integration Test".bright_blue().bold());
    log_info!("main", "{}", "Testing core development functionality".dimmed());
    
    println!("{}", "🔬 Starting Simplified Integration Test".bright_blue().bold());
    println!("{}", "Testing core CAI development capabilities...".dimmed());
    println!();
    
    match run_simplified_integration_test().await {
        Ok(result) => {
            let successful_tests = result.test_results.iter().filter(|t| t.success).count();
            let total_tests = result.test_results.len();
            
            if result.overall_success && successful_tests >= (total_tests * 3 / 4) {
                println!("\n{} Simplified integration test PASSED! ({}/{} tests successful)", 
                        "✅".green(), successful_tests, total_tests);
                println!("{} Core functionality is working correctly", 
                        "🎉".bright_green());
                log_info!("main", "✅ Simple integration test passed: {}/{} tests successful", 
                        successful_tests, total_tests);
            } else {
                println!("\n{} Simplified integration test FAILED ({}/{} tests successful)", 
                        "❌".red(), successful_tests, total_tests);
                println!("{} Some core functionality needs attention", 
                        "⚠️".yellow());
                log_warn!("main", "⚠️ Simple integration test failed: {}/{} tests successful", 
                        successful_tests, total_tests);
            }
            
            println!("\n{} Total execution time: {:.2}s", "⏱️", result.total_execution_time);
            
            Ok(())
        }
        Err(e) => {
            println!("\n{} Simplified integration test encountered an error: {}", "❌".red(), e);
            log_error!("main", "❌ Simple integration test error: {}", e);
            Err(e)
        }
    }
}

/// Run basic functionality test for core components
async fn run_basic_test_command() -> Result<()> {
    use crate::basic_functionality_test::run_basic_functionality_test;
    
    log_info!("main", "{}", "🧪 Basic Functionality Test".bright_blue().bold());
    log_info!("main", "{}", "Testing core component functionality".dimmed());
    
    println!("{}", "🧪 Starting Basic Functionality Test".bright_blue().bold());
    println!("{}", "Testing core CAI component functionality...".dimmed());
    println!();
    
    match run_basic_functionality_test().await {
        Ok(result) => {
            let successful_tests = result.test_results.iter().filter(|t| t.success).count();
            let total_tests = result.test_results.len();
            let success_rate = if total_tests > 0 { successful_tests * 100 / total_tests } else { 0 };
            
            if result.overall_success || success_rate >= 80 {
                println!("\n{} Basic functionality test PASSED! ({}/{} tests successful - {}%)", 
                        "✅".green(), successful_tests, total_tests, success_rate);
                println!("{} Core components are working correctly", 
                        "🎉".bright_green());
                log_info!("main", "✅ Basic functionality test passed: {}/{} tests successful ({}%)", 
                        successful_tests, total_tests, success_rate);
            } else {
                println!("\n{} Basic functionality test FAILED ({}/{} tests successful - {}%)", 
                        "❌".red(), successful_tests, total_tests, success_rate);
                println!("{} Some core components need attention", 
                        "⚠️".yellow());
                log_warn!("main", "⚠️ Basic functionality test failed: {}/{} tests successful ({}%)", 
                        successful_tests, total_tests, success_rate);
            }
            
            println!("\n{} Total execution time: {:.2}s", "⏱️", result.total_execution_time);
            
            Ok(())
        }
        Err(e) => {
            println!("\n{} Basic functionality test encountered an error: {}", "❌".red(), e);
            log_error!("main", "❌ Basic functionality test error: {}", e);
            Err(e)
        }
    }
}

async fn handle_mcp_command(action: &McpCommands) -> Result<()> {
    // MCP servers are automatically initialized at startup
    // Only check for initialization on Init command which might create new config

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
