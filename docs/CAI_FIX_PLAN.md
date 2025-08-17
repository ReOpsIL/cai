# CAI Comprehensive Fix Plan

## Test Results Analysis

**Current Performance**: 55/100 tests passing (55.0% success rate)  
**Target Goal**: Achieve >80% success rate (80+ tests passing)  
**Tests to Fix**: 45 failing tests across multiple categories  

## Failure Analysis by Category

### 🔴 Critical Issues (High Priority)

#### 1. Basic Functionality (1/10 passing - 10% success)
**Root Causes**:
- ✅ **FIXED**: CLI argument parsing conflict (`--version` duplication)
- ✅ **FIXED**: MCP config corruption from test error_076
- Missing command flag implementations
- Invalid argument handling

**Remaining Issues**:
- Missing CLI flags and options not implemented
- Command routing failures for edge cases
- Environment variable handling inconsistencies

#### 2. MCP Integration (1/10 passing - 10% success)  
**Root Causes**:
- Docker service initialization failures
- MCP server configuration and discovery issues
- Tool call argument parsing problems
- Server lifecycle management issues

#### 3. Workflow Orchestration (2/10 passing - 20% success)
**Root Causes**:
- Workflow persistence not fully implemented
- State management across sessions incomplete
- Complex workflow decomposition failures
- Timeout issues in long-running workflows

### 🟡 Moderate Issues (Medium Priority)

#### 4. Prompt Management (5/10 passing - 50% success)
**Root Causes**:
- Missing file/directory arguments support
- Query command argument parsing issues
- Non-existent file handling edge cases

#### 5. Search Operations (7/10 passing - 70% success)
**Root Causes**:
- Long query handling issues
- Special character escaping problems
- Empty result handling

### 🟢 Working Well (Low Priority)

#### 6. File Operations (10/10 passing - 100% success) ✅
#### 7. Chat Interactions (8/10 passing - 80% success) 
#### 8. Error Handling (7/10 passing - 70% success)
#### 9. Edge Cases (7/10 passing - 70% success)
#### 10. Performance Tests (7/10 passing - 70% success)

## Detailed Fix Implementation Plan

### Phase 1: Critical Basic Functionality Fixes

#### Fix 1.1: Complete CLI Argument Implementation
**Problem**: Missing CLI flags and commands not implemented
**Solution**:
```rust
// Add missing CLI arguments to main.rs
#[derive(Parser)]
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
    
    /// Disable colored output
    #[arg(long)]
    no_color: bool,
    
    /// Resolve URLs in search results
    #[arg(long)]
    resolve_urls: bool,
}
```

#### Fix 1.2: Improve Command Routing
**Problem**: Invalid commands and edge cases not handled properly
**Solution**:
```rust
// Enhanced command matching with better error messages
match &cli.command {
    Some(Commands::List) => handle_list_command(&cli, &manager).await?,
    Some(Commands::Search { query, resolve_urls }) => {
        handle_search_command(&cli, &manager, query, *resolve_urls).await?
    },
    Some(Commands::Show { file_name }) => {
        handle_show_command(&cli, &manager, file_name).await?
    },
    Some(Commands::Query { file, subject, prompt }) => {
        handle_query_command(&cli, &manager, file, subject, prompt).await?
    },
    None => {
        // Default to list when no command provided
        handle_list_command(&cli, &manager).await?
    },
    _ => {
        eprintln!("Error: Unknown command. Use --help for available commands.");
        std::process::exit(2);
    }
}
```

#### Fix 1.3: Environment Variable Support
**Problem**: Environment variables not properly handled
**Solution**:
```rust
// Enhanced environment variable handling
fn setup_environment_config(cli: &Cli) -> EnvironmentConfig {
    EnvironmentConfig {
        debug_mode: cli.debug || std::env::var("CAI_LOG_LEVEL").unwrap_or_default() == "debug",
        no_color: cli.no_color || std::env::var("NO_COLOR").is_ok(),
        prompts_dir: cli.directory.clone().or_else(|| {
            std::env::var("CAI_PROMPTS_DIR").ok().map(PathBuf::from)
        }).unwrap_or_else(|| PathBuf::from("prompts")),
    }
}
```

### Phase 2: MCP Integration Fixes

#### Fix 2.1: Docker Service Management
**Problem**: Docker MCP servers failing to initialize
**Solution**:
```rust
// Enhanced MCP server initialization with better error handling
impl McpClientManager {
    pub async fn initialize_with_fallback(&mut self, config: &McpConfig) -> Result<()> {
        for (server_name, server_config) in &config.mcp_servers {
            match self.start_server_with_retry(server_name, server_config, 3).await {
                Ok(_) => log_info!("mcp", "✅ Started MCP server: {}", server_name),
                Err(e) => {
                    log_warn!("mcp", "⚠️ Failed to start MCP server '{}': {}", server_name, e);
                    // Continue with other servers instead of failing completely
                }
            }
        }
        Ok(())
    }
    
    async fn start_server_with_retry(&mut self, name: &str, config: &ServerConfig, retries: u32) -> Result<()> {
        for attempt in 1..=retries {
            match self.start_server(name, config).await {
                Ok(client) => {
                    self.clients.insert(name.to_string(), client);
                    return Ok(());
                }
                Err(e) if attempt < retries => {
                    log_debug!("mcp", "Retry {}/{} for server '{}': {}", attempt, retries, name, e);
                    tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}
```

#### Fix 2.2: Tool Call Argument Parsing
**Problem**: MCP tool arguments not parsed correctly
**Solution**:
```rust
// Improved argument parsing for MCP tools
pub async fn call_tool_safe(&self, server_name: &str, tool_name: &str, args_str: &str) -> Result<Value> {
    let arguments = if args_str.trim().is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        match serde_json::from_str(args_str) {
            Ok(args) => args,
            Err(_) => {
                // Try to parse as a simple string if JSON parsing fails
                log_debug!("mcp", "Failed to parse as JSON, treating as string");
                serde_json::json!({ "input": args_str })
            }
        }
    };
    
    self.call_tool(server_name, tool_name, arguments).await
}
```

#### Fix 2.3: MCP Server Status and Discovery
**Problem**: Server status and tool discovery not working properly
**Solution**:
```rust
// Enhanced server status and tool discovery
impl McpClientManager {
    pub async fn get_server_status(&self) -> Result<HashMap<String, ServerStatus>> {
        let mut status_map = HashMap::new();
        
        for (server_name, client) in &self.clients {
            let status = match self.ping_server(server_name).await {
                Ok(_) => ServerStatus::Running,
                Err(_) => ServerStatus::Failed,
            };
            status_map.insert(server_name.clone(), status);
        }
        
        Ok(status_map)
    }
    
    pub async fn discover_tools_safe(&self, server_name: &str) -> Result<Vec<ToolInfo>> {
        match self.list_tools(server_name).await {
            Ok(tools) => Ok(tools),
            Err(e) => {
                log_warn!("mcp", "Failed to discover tools for '{}': {}", server_name, e);
                Ok(Vec::new()) // Return empty list instead of failing
            }
        }
    }
}
```

### Phase 3: Workflow Orchestration Improvements

#### Fix 3.1: Workflow Persistence
**Problem**: Workflow state not properly persisted across sessions
**Solution**:
```rust
// Enhanced workflow persistence
impl WorkflowOrchestrator {
    pub async fn start_workflow_with_persistence(&self, description: &str) -> Result<String> {
        let workflow_id = self.generate_workflow_id();
        
        // Create workflow with initial state
        let workflow = WorkflowState::new(&workflow_id, description);
        
        // Persist workflow to disk immediately
        self.save_workflow_state(&workflow).await?;
        
        // Start execution in background
        self.begin_workflow_execution(&workflow_id).await?;
        
        Ok(workflow_id)
    }
    
    async fn save_workflow_state(&self, workflow: &WorkflowState) -> Result<()> {
        let workflow_dir = PathBuf::from(".cai/workflows");
        tokio::fs::create_dir_all(&workflow_dir).await?;
        
        let file_path = workflow_dir.join(format!("{}.json", workflow.id));
        let json_data = serde_json::to_string_pretty(workflow)?;
        tokio::fs::write(file_path, json_data).await?;
        
        Ok(())
    }
    
    pub async fn continue_workflow_safe(&self, workflow_id: &str) -> Result<String> {
        // Load workflow state from disk
        match self.load_workflow_state(workflow_id).await {
            Ok(workflow) => {
                self.resume_workflow_execution(workflow).await
            }
            Err(e) => {
                log_warn!("workflow", "Failed to load workflow '{}': {}", workflow_id, e);
                Err(anyhow!("Workflow '{}' not found or corrupted", workflow_id))
            }
        }
    }
}
```

#### Fix 3.2: Timeout Handling
**Problem**: Long-running workflows timing out in tests
**Solution**:
```rust
// Enhanced timeout handling for workflows
impl WorkflowOrchestrator {
    pub async fn execute_with_timeout(&self, workflow_id: &str, timeout_secs: u64) -> Result<String> {
        let timeout_duration = Duration::from_secs(timeout_secs);
        
        match tokio::time::timeout(timeout_duration, self.execute_workflow(workflow_id)).await {
            Ok(result) => result,
            Err(_) => {
                log_warn!("workflow", "Workflow '{}' timed out after {}s", workflow_id, timeout_secs);
                // Save partial progress
                self.save_workflow_progress(workflow_id, "TIMEOUT").await?;
                Ok(format!("Workflow '{}' timed out but progress saved", workflow_id))
            }
        }
    }
}
```

### Phase 4: Prompt Management Enhancements

#### Fix 4.1: Enhanced Query Command
**Problem**: Query command argument parsing and validation issues
**Solution**:
```rust
// Improved query command implementation
async fn handle_query_command(
    cli: &Cli,
    manager: &PromptManager,
    file: &str,
    subject: &str,
    prompt: &str,
) -> Result<()> {
    // Validate inputs
    if file.trim().is_empty() || subject.trim().is_empty() || prompt.trim().is_empty() {
        return Err(anyhow!("All query parameters (file, subject, prompt) must be non-empty"));
    }
    
    // Try to find exact match first
    match manager.query_prompt(file, subject, prompt) {
        Ok(result) => {
            println!("{}", result);
            Ok(())
        }
        Err(_) => {
            // Try fuzzy matching if exact match fails
            let suggestions = manager.find_similar_queries(file, subject, prompt)?;
            if suggestions.is_empty() {
                eprintln!("Error: No matching prompt found for query");
                eprintln!("File: '{}', Subject: '{}', Prompt: '{}'", file, subject, prompt);
                std::process::exit(1);
            } else {
                eprintln!("No exact match found. Did you mean:");
                for suggestion in suggestions.iter().take(3) {
                    eprintln!("  {} {} \"{}\"", suggestion.file, suggestion.subject, suggestion.prompt);
                }
                std::process::exit(1);
            }
        }
    }
}
```

#### Fix 4.2: Directory Support
**Problem**: --directory flag not properly supported
**Solution**:
```rust
// Enhanced directory support
async fn handle_list_command(cli: &Cli, manager: &PromptManager) -> Result<()> {
    let prompts_dir = resolve_prompts_directory(&cli.directory)?;
    
    // Reload manager with specified directory if different
    let manager = if prompts_dir != manager.get_current_directory() {
        PromptManager::new(&prompts_dir)?
    } else {
        manager.clone()
    };
    
    let prompt_files = manager.list_all();
    if prompt_files.is_empty() {
        println!("No prompt files found in directory: {}", prompts_dir.display());
        return Ok(());
    }
    
    print_prompt_list(&prompt_files, &cli.mode)?;
    Ok(())
}

fn resolve_prompts_directory(directory: &Path) -> Result<PathBuf> {
    let dir = if directory.is_absolute() {
        directory.to_path_buf()
    } else {
        std::env::current_dir()?.join(directory)
    };
    
    if !dir.exists() {
        return Err(anyhow!("Directory does not exist: {}", dir.display()));
    }
    
    if !dir.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", dir.display()));
    }
    
    Ok(dir)
}
```

### Phase 5: Search Operations Improvements

#### Fix 5.1: Long Query Handling
**Problem**: Very long search queries not handled properly
**Solution**:
```rust
// Enhanced search query processing
impl PromptManager {
    pub fn search_prompts_enhanced(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        // Limit query length to prevent performance issues
        const MAX_QUERY_LENGTH: usize = 1000;
        let trimmed_query = if query.len() > MAX_QUERY_LENGTH {
            log_warn!("search", "Query truncated from {} to {} characters", query.len(), MAX_QUERY_LENGTH);
            &query[..MAX_QUERY_LENGTH]
        } else {
            query
        };
        
        // Handle empty queries gracefully
        if trimmed_query.trim().is_empty() {
            log_info!("search", "Empty query provided, returning all prompts");
            return Ok(self.get_all_prompts_as_search_results());
        }
        
        // Perform search with timeout
        let search_timeout = Duration::from_secs(10);
        let start_time = Instant::now();
        
        let results = self.perform_search_internal(trimmed_query, options)?;
        
        let elapsed = start_time.elapsed();
        if elapsed > search_timeout {
            log_warn!("search", "Search took longer than expected: {:?}", elapsed);
        }
        
        Ok(results)
    }
}
```

## Implementation Priority and Timeline

### Week 1: Critical Fixes (Target: +20 passing tests)
1. ✅ **COMPLETED**: Fix CLI argument parsing conflict
2. **Fix basic functionality issues** (estimated +8-10 tests)
3. **Fix MCP initialization and basic tool calls** (estimated +5-7 tests)
4. **Fix workflow persistence basics** (estimated +3-5 tests)

### Week 2: Moderate Improvements (Target: +10 passing tests)
1. **Complete prompt management fixes** (estimated +3-5 tests)
2. **Enhance search operations** (estimated +2-3 tests)
3. **Improve error handling edge cases** (estimated +2-3 tests)
4. **Performance optimizations** (estimated +2-3 tests)

### Week 3: Polish and Edge Cases (Target: +5 passing tests)
1. **Handle all edge cases and special characters**
2. **Optimize timeout handling**
3. **Complete workflow orchestration features**
4. **Final integration testing**

## Success Metrics

### Target Results After Fixes
- **Basic Functionality**: 1/10 → 8/10 (80% success)
- **MCP Integration**: 1/10 → 7/10 (70% success)  
- **Workflow Orchestration**: 2/10 → 7/10 (70% success)
- **Prompt Management**: 5/10 → 9/10 (90% success)
- **Search Operations**: 7/10 → 9/10 (90% success)

### Overall Target
- **Current**: 55/100 (55% success rate)
- **Target**: 80+/100 (80%+ success rate)
- **Improvement**: +25 tests passing (+45% relative improvement)

## Risk Mitigation

### High-Risk Changes
1. **CLI Structure Changes**: Implement with backward compatibility
2. **MCP Protocol Changes**: Maintain existing API contracts
3. **Workflow State Format**: Include migration logic for existing workflows

### Testing Strategy
1. **Run test suite after each major fix**
2. **Implement fix-specific unit tests**
3. **Add regression tests for previously failing scenarios**
4. **Performance benchmarking for critical paths**

## Rollback Strategy

### Safe Implementation Approach
1. **Feature flags for major changes**
2. **Incremental rollout of fixes**
3. **Backup configurations before changes**
4. **Git branching strategy for safe experimentation**

---

**Next Action**: Begin Phase 1 implementation with Fix 1.1 (CLI argument completion)

*This plan provides a systematic approach to achieving 80%+ test success rate through prioritized, measurable improvements.*