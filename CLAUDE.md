# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CAI (Conversational AI Interface) is a Rust CLI application for managing prompt collections and executing tasks through Model Context Protocol (MCP) servers. The application combines prompt management, LLM-powered task planning, and intelligent tool selection for automated task execution.

## Essential Commands

### Development Commands
```bash
# Build and run (recommended approach)
./run.sh --build list                    # Build then list prompts
./run.sh --test --build chat            # Test, build, then start chat
./run.sh --release show bug_fixing      # Release build then show file

# Direct cargo commands
cargo build --release                   # Production build
cargo test                             # Run all tests
cargo run -- list                      # Run with list command
cargo run -- task-demo                 # Test MCP tool integration

# Environment setup
export OPENROUTER_API_KEY="your_key"   # Required for LLM features
export CAI_LOG_LEVEL=DEBUG             # Enable debug logging
```

### Core Application Commands
```bash
# Prompt management
cai list                               # List all prompts
cai search "performance"               # Search prompt content
cai show bug_fixing                    # Show specific prompt file
cai query file subject "prompt title" # Get specific prompt

# Interactive chat mode (LLM-powered)
cai chat                              # Start chat with task planning

# MCP server management
cai mcp list                          # List configured servers
cai mcp start filesystem              # Start MCP server
cai mcp tools filesystem              # List available tools
cai mcp call filesystem read_file --args '{"path":"/file.txt"}'

# Testing and demos
cai task-demo                         # Demo MCP tool integration
```

## Architecture Overview

### Core Components

**Main Application Flow**: `main.rs` → Command parsing → Module delegation → Result presentation

**Key Modules**:
- **`prompt_loader`**: YAML prompt file management, search, and similarity analysis
- **`chat_interface`**: Interactive chat mode with LLM task planning and prompt management
- **`task_executor`**: LLM-powered task analysis and MCP tool orchestration
- **`openrouter_client`**: LLM API client with tool analysis capabilities
- **`mcp_client`**: MCP protocol client using rmcp crate
- **`mcp_manager`**: Global MCP server lifecycle management
- **`logger`**: Structured logging with performance metrics

### Data Flow Architecture

1. **Command Input** → CLI parsing (clap) → Module routing
2. **Chat Mode**: User input → LLM task planning → Task queue → MCP tool execution
3. **Prompt Management**: YAML files → In-memory structures → Search/similarity analysis
4. **MCP Integration**: Task analysis → Tool selection (LLM) → Tool execution → Result aggregation

### LLM-Powered Intelligence

The application uses OpenRouter API for intelligent task processing:
- **Task Planning**: Converts user requests into structured task lists
- **Tool Selection**: LLM analyzes tasks and selects appropriate MCP tools
- **Prompt Management**: Automatic similarity detection, updating, and categorization

### MCP (Model Context Protocol) Integration

- **Global Manager**: Singleton pattern for server lifecycle management
- **Docker-based Servers**: Configured via `mcp-config.json`
- **Tool Discovery**: Dynamic tool enumeration from active servers
- **Intelligent Dispatch**: LLM selects tools based on task requirements

## Configuration

### MCP Configuration (`mcp-config.json`)
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "-v", "/path:/project", "mcp/filesystem", "/project"],
      "env": {},
      "cwd": null
    }
  }
}
```

### Environment Variables
- `OPENROUTER_API_KEY`: Required for LLM features (chat mode, intelligent tool selection)
- `CAI_PROMPTS_DIR`: Custom prompts directory (default: `./prompts`)
- `CAI_LOG_LEVEL`: Logging level (TRACE, DEBUG, INFO, WARN, ERROR)

## Critical Implementation Details

### Task Executor Architecture
- **Dual Mode**: LLM-based analysis with heuristic fallback
- **Tool Metadata**: Structured tool descriptions for LLM context
- **Async Execution**: Concurrent MCP tool calls with timeout handling
- **State Management**: Task queue with status tracking (Waiting, Running, Done, Failed)

### Prompt System Organization
- **Hierarchical Structure**: File → Subject → Prompt
- **Similarity Analysis**: Uses strsim crate for duplicate detection
- **Auto-categorization**: LLM-powered prompt classification
- **Score-based Ranking**: Usage-based prompt effectiveness tracking

### MCP Client Implementation
- **Real Protocol**: Uses rmcp crate for JSON-RPC 2.0 communication
- **Service Management**: `RunningService<RoleClient, ()>` pattern
- **Error Handling**: Comprehensive timeout and retry logic
- **Tool Discovery**: Dynamic enumeration with metadata collection

### LLM Integration Patterns
- **JSON Response Parsing**: Robust extraction from markdown-wrapped responses
- **Fallback Mechanisms**: Graceful degradation when LLM unavailable
- **Context Management**: Efficient prompt engineering for tool selection
- **Performance Optimization**: Caching and timeout management

## Key Dependencies

- **rmcp**: MCP protocol implementation
- **reqwest**: HTTP client for OpenRouter API
- **tokio**: Async runtime for concurrent operations
- **serde_yaml**: YAML prompt file processing
- **clap**: CLI argument parsing
- **anyhow**: Error handling throughout application
- **strsim**: Text similarity for prompt management

## Testing and Debugging

### Test MCP Integration
```bash
cargo run -- task-demo    # Full MCP workflow test
RUST_LOG=debug cargo run -- mcp tools filesystem  # Debug MCP calls
```

### Enable Comprehensive Logging
```bash
CAI_LOG_LEVEL=TRACE ./run.sh chat  # Full execution tracing
```

### Verify LLM Integration
```bash
export OPENROUTER_API_KEY="your_key"
cargo run -- chat  # Test LLM-powered features
```