# CAI — Conversational AI Interface

CAI is a powerful Rust CLI application that combines intelligent prompt management with LLM-driven agentic coding. It transforms natural language requests into executable tasks, integrates with external tools via Model Context Protocol (MCP), and maintains context across conversations for multi-step project development.

## 🚀 Key Features

- **📋 Smart Prompt Management**: YAML-based repository with search, similarity detection, and auto-curation
- **🤖 Agentic Chat Mode**: LLM plans and executes tasks automatically with context awareness
- **🔄 Workflow Orchestration**: Break down complex projects into manageable goals and sub-goals
- **🛠️ MCP Integration**: Connects to external tools (filesystem, web, etc.) via Model Context Protocol
- **💡 Context-Aware Planning**: Builds upon previous work rather than starting from scratch
- **⚙️ Flexible Execution Modes**: From suggestions to full automation
- **✅ Built-in Validation**: Optional code formatting, linting, and testing after execution
- **📂 Session Management**: Persistent workflow sessions that restore context across chat sessions

## 🎯 Getting Started

### Prerequisites

1. **Rust**: Install from [rustup.rs](https://rustup.rs/)
2. **OpenRouter API Key**: Get from [openrouter.ai](https://openrouter.ai/) for LLM features
3. **Docker** (Optional): For MCP server integration

### Quick Setup

1. **Clone and Build**:
```bash
git clone <repository-url>
cd cai
cargo build --release
```

2. **Set API Key**:
```bash
export OPENROUTER_API_KEY="your_api_key_here"
```

3. **Start Using CAI**:
```bash
# List available prompts
./run.sh list

# Start interactive chat mode
./run.sh chat

# Get help
./run.sh --help
```

### First Steps with Chat Mode

The most powerful way to use CAI is through its interactive chat mode:

```bash
./run.sh chat
```

Try these example requests:
- `"create a Python web API for task management"`
- `"add authentication to the existing API"`
- `"create a React frontend for the task management API"`

CAI will automatically:
- Plan the necessary tasks
- Create files and directories
- Build upon previous work in the same session
- Maintain context between requests

## Installation

### Quick Start (Recommended)

Use the provided run scripts for the easiest experience:

Linux/macOS:
```bash
./run.sh --help              # Show all options
./run.sh --build list        # Build and list prompts
./run.sh chat                # Start chat mode
```

Windows:
```cmd
run.bat --help               # Show all options
run.bat --build list         # Build and list prompts
run.bat chat                 # Start chat mode
```

The run scripts will automatically:
- Build the project if needed
- Create a sample prompts directory
- Handle environment setup
- Pass through all CAI commands

### Manual Installation

```bash
cargo build --release
```

## Usage

### Using the Run Scripts (Recommended)

List all prompts:
```bash
./run.sh list
```

Search prompts:
```bash
./run.sh search "performance"
./run.sh search "security"
```

Show a prompt file:
```bash
./run.sh show bug_fixing
```

Query a specific prompt:
```bash
./run.sh query bug_fixing "Performance Issues" "Performance bottleneck analysis"
```

Specify a custom prompts directory:
```bash
./run.sh --directory /path/to/prompts list
```

### Direct Binary Usage

```bash
./target/release/cai list
./target/release/cai search "performance"
./target/release/cai show bug_fixing
./target/release/cai query bug_fixing "Performance Issues" "Performance bottleneck analysis"
./target/release/cai --directory /path/to/prompts list
```

### Operation Modes (Autonomy)

Control how the agent executes planned tasks:

```bash
# Suggest (ask before executing planned tasks)
./run.sh --mode suggest chat

# Auto Edit (default): execute tasks automatically
./run.sh --mode auto-edit chat

# Full Auto (same execution behavior today; pair with validation below)
./run.sh --mode full-auto chat
```

`--mode` is also available when calling the binary directly.

## YAML File Structure

```yaml
name: "Bug Fixing"
description: "Prompts for debugging and fixing code issues"
subjects:
  - name: "General Debugging"
    prompts:
      - title: "Analyze error logs"
        content: "Analyze the following error logs and identify the root cause..."
      - title: "Code review for bugs"
        content: "Review this code for potential bugs, security vulnerabilities..."
  - name: "Concurrency Bugs"
    prompts:
      - title: "Deadlock analysis"
        content: "file://prompts/deadlock_analysis.md"
      - title: "Online prompt"
        content: "https://raw.githubusercontent.com/user/repo/main/prompt.md"
```

### URL References in Content

The `content` field supports URL references for external prompt files:

- Local files: `file://path/to/prompt.md` (relative to prompts dir)
- HTTP/HTTPS: `https://example.com/prompt.md` for online prompts
- Absolute paths: `file:///absolute/path/to/prompt.md`

URL references allow you to:
- Store large prompts in separate markdown files
- Share prompts across multiple YAML files
- Use online prompt repositories
- Keep YAML files clean and focused

## Sample Prompts Included

- Bug Fixing: Error analysis, performance issues, concurrency bugs
- Code Analysis: Architecture review, code quality, security analysis
- Task Creation: Project planning, documentation, testing strategy
- Refactoring: Clean code practices, performance optimization, modernization

## Commands

### Prompt Management
- `list`: Display all available prompt files and their contents
- `search <query>`: Search for prompts containing the query string
- `show <file_name>`: Display detailed view of a specific prompt file
- `query <file> <subject> <prompt>`: Retrieve a specific prompt
- `chat`: Start interactive chat mode for AI‑powered task planning and prompt management

### MCP (Model Context Protocol) Support
- `mcp status`: Show MCP server status overview (servers auto-start)
- `mcp list`: List configured MCP servers and their status  
- `mcp tools <server>`: List tools available from a server
- `mcp call <server> <tool> --args <json>`: Call a tool with arguments
- `mcp resources <server>`: List resources available from a server
- `mcp init`: Create default MCP configuration file

**Note**: MCP servers now auto-start when CAI launches, so manual start/stop commands are rarely needed.

### Workflow Orchestration (Agentic Coder)
- `workflow start "<description>"`: Create a workflow from a high‑level request
- `workflow status`: List active workflows
- `workflow show <WORKFLOW_ID>`: Show detailed goal hierarchy and recent actions
- `workflow continue <WORKFLOW_ID>`: Execute next executable goal(s)
- `workflow cleanup`: Remove completed workflows from memory and disk

## 🤖 Chat Mode Features

```bash
./run.sh chat
```

The chat mode is CAI's most powerful feature, providing an intelligent coding assistant that maintains context across conversations:

### Core Capabilities
- **🧠 Context-Aware Planning**: Remembers previous work and builds upon it
- **📝 Intelligent Task Breakdown**: Converts natural language into executable tasks
- **🔗 Session Continuity**: Persistent workflows that resume across chat sessions
- **🛠️ Tool Integration**: Automatically selects and uses appropriate tools
- **📋 Smart Prompt Management**: Auto-curates prompt repository based on usage
- **⚙️ Flexible Execution**: Choose from suggestion, auto-edit, or full-auto modes

### Special Commands in Chat Mode

- `@status` - Show current task queue status
- `@execute` - Execute all queued tasks  
- `@clear` - Clear completed tasks
- `@plan` - Create a validated execution plan
- `@improve` - Run iterative improvement on current work
- `@feedback` - Show feedback statistics and learning insights
- `@workflow` - Access workflow orchestration menu
- `@help` - Show all available commands
- `quit` - Exit chat mode

### Context-Aware Multi-Step Development

CAI excels at multi-step project development. For example:

**Step 1**: `"create a Python backend API for medical assessments"`
- Creates structured project with FastAPI
- Sets up models, routes, services
- Includes tests and documentation

**Step 2**: `"create a web interface for the medical assessment backend"`  
- **NEW**: References existing backend project
- Integrates with actual API endpoints
- Uses specific models from backend
- Creates complementary frontend structure

### How Context Awareness Works

1. **Session Management**: Each workflow session is persistent and restorable
2. **Historical Context**: Previous tasks and outputs inform new planning
3. **Path Resolution**: References existing files and directories correctly
4. **Integration Planning**: Builds upon existing functionality rather than duplicating
5. **Incremental Development**: Each prompt enhances the previous work

### Setup

Set your OpenRouter API key:
```bash
export OPENROUTER_API_KEY="your_api_key_here"
```

Get your API key from: https://openrouter.ai/

Optional validation after execution:
```bash
export CAI_VALIDATE=after_all
```

Set execution mode per‑invocation with `--mode`, or globally:
```bash
export CAI_MODE=auto-edit   # suggest | auto-edit | full-auto
```

## Prompt Scoring System

Prompts include a `score` that tracks usage and effectiveness:
- Score increases when the prompt is matched in chat interactions
- High‑scoring prompts indicate proven usefulness
- Scores are displayed as ⭐ in listings

## Search Capabilities

Search considers:
- File names
- File descriptions
- Subject names
- Prompt titles
- Prompt content (including URL‑referenced content)

Results show the match type and context for easy navigation.

## 🛠️ MCP (Model Context Protocol) Integration

CAI automatically connects to MCP servers on startup to extend its capabilities with external tools (filesystem operations, web access, databases, etc.). MCP integration is optional but highly recommended.

### Automatic Server Management

**🚀 New**: MCP servers automatically start when CAI launches - no manual start commands needed!

- All configured servers start automatically
- Health checks ensure servers are ready
- Graceful shutdown when CAI exits
- Optimized for Docker environments

### MCP Configuration

Create a default configuration:

```bash
./run.sh mcp init
```

Example `mcp-config.json` (filesystem server using Docker):

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-v", "/path/to/your/project:/workspace",
        "mcp/filesystem",
        "/workspace"
      ],
      "env": {},
      "cwd": null
    }
  }
}
```

### MCP Usage Examples

```bash
# Check server status (servers auto-start)
./run.sh mcp status

# List available tools  
./run.sh mcp tools filesystem

# Call a tool directly
./run.sh mcp call filesystem list_directory --args '{"path":"/workspace"}'

# View all configured servers
./run.sh mcp list
```

**💡 Tip**: In chat mode, CAI automatically selects and uses appropriate MCP tools based on your requests - no need to call them manually!

## Workflow Orchestration Details

The orchestrator:
- Analyzes your request into a root goal and success criteria
- Plans sub‑goals and per‑goal task lists (LLM)
- Executes tasks via the `TaskExecutor` and MCP tools
- Optionally validates the repo (`CAI_VALIDATE=after_all`)
- Refines goals based on results and tracks progress to completion

Project context (counts, cargo package name, etc.) is summarized and provided to the planner to improve breakdowns.

## Development

- `cargo build`: Compile in debug mode
- `cargo run -- <args>`: Run the binary locally (passes args to the app)
- `cargo test`: Run unit/integration tests
- `cargo fmt --all`: Format code with rustfmt
- `cargo clippy -- -D warnings`: Lint; treat warnings as errors
- `./run.sh` (or `run.bat`): Project launcher with sensible defaults

### Environment Variables
- `OPENROUTER_API_KEY`: **Required** for LLM features (planning, refinement, analysis)
- `CAI_MODE`: Execution mode for chat/workflows (`suggest` | `auto-edit` | `full-auto`)
- `CAI_VALIDATE`: Set to `after_all` to run validators after task execution
- `CAI_PROMPTS_DIR`: Set automatically from `--directory`; used for safe file URL resolution
- `CAI_LOG_LEVEL`: Logging level (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`)

## 🔧 Troubleshooting

### Common Issues

**1. "OpenRouter API key is not set"**
```bash
export OPENROUTER_API_KEY="your_api_key_here"
# Verify it's set
echo $OPENROUTER_API_KEY
```

**2. MCP servers not starting**
```bash
# Check Docker is running
docker ps

# Initialize MCP configuration
./run.sh mcp init

# Check MCP status
./run.sh mcp status
```

**3. Context not preserved between prompts**
- Ensure you're using the same chat session
- Check that workflow session is active: look for session ID in chat startup
- Historical context is gathered automatically - no action needed

**4. Files created but remain empty**
- This was a known issue that has been fixed
- Upgrade to the latest version
- Files should now contain proper content after LLM generation

**5. Tasks fail with "read-before-edit violation"**
- This issue has been resolved with automatic file reading
- Safety system now auto-handles read requirements

### Debug Mode

Enable detailed logging:
```bash
CAI_LOG_LEVEL=DEBUG ./run.sh chat
```

Enable trace-level logging for maximum detail:
```bash  
CAI_LOG_LEVEL=TRACE ./run.sh chat
```

### Getting Help

- Use `./run.sh --help` for command options
- Use `@help` in chat mode for special commands  
- Check logs for detailed error information
- Ensure Docker is running if using MCP servers

## Notes
- MCP and Docker are optional; without MCP, tool execution may be limited
- Validation runs external commands; enable only when appropriate for your environment
- Context awareness works best within the same workflow session
- Session files are stored in `~/.config/cai/` or local directory if home is unavailable
