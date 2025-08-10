# CAI — Agentic Coder CLI

CAI is a Rust CLI that blends prompt repository management with an LLM‑driven agentic coder. It plans tasks from natural language, orchestrates tools via MCP, executes in loops, and can optionally validate results (fmt, clippy, tests).

## Features

- Prompt management: YAML repository; list/search/query prompts
- Agentic chat: LLM plans tasks, executes with MCP tools, and curates prompts
- Workflow orchestration: LLM creates goals/sub‑goals and executes iteratively
- MCP integration: Start/stop servers; list tools; call tools and resources
- Operation modes: `suggest`, `auto-edit` (default), `full-auto`
- Optional validation: Run `cargo fmt`, `clippy`, and `tests` after execution

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
- `mcp list`: List configured MCP servers and their status
- `mcp start <server>`: Start an MCP server
- `mcp stop <server>`: Stop an MCP server
- `mcp tools <server>`: List tools available from a server
- `mcp call <server> <tool> --args <json>`: Call a tool with arguments
- `mcp resources <server>`: List resources available from a server
- `mcp status`: Show MCP server status overview

### Workflow Orchestration (Agentic Coder)
- `workflow start "<description>"`: Create a workflow from a high‑level request
- `workflow status`: List active workflows
- `workflow show <WORKFLOW_ID>`: Show detailed goal hierarchy and recent actions
- `workflow continue <WORKFLOW_ID>`: Execute next executable goal(s)
- `workflow cleanup`: Remove completed workflows from memory and disk

## Chat Mode Features

```bash
./run.sh chat
```

The chat mode provides:
- AI‑powered task planning: Input any request and get a structured task breakdown
- Smart prompt management: Automatically adds, updates, or scores existing prompts
- Similarity detection: Prevents duplicate prompts and improves existing ones
- Automatic categorization: Tasks are intelligently sorted into appropriate subjects
- Operation modes: Choose `suggest`, `auto-edit`, or `full-auto`
- Optional validation: Enable `CAI_VALIDATE=after_all` to run fmt/clippy/tests after tasks

### How Chat Mode Works

1. Task Planning: Enter a request and the AI generates actionable tasks
2. Similarity Analysis: Each task is compared against existing prompts
3. Smart Repository Management:
   - New prompts: Added if no similar prompts exist
   - Prompt updates: Similar prompts are improved and merged
   - Score increment: Exact matches get higher relevance scores
4. Self‑curating repository: High‑quality prompts emerge through usage

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

## MCP (Model Context Protocol) Integration

CAI can connect to MCP servers to extend capabilities (filesystem, etc.). MCP is optional.

### MCP Configuration

Create a default `mcp-config.json`:

```bash
./run.sh mcp init
```

Example (filesystem server using Docker and mounting the project directory):

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-v", "/path/to/your/project:/project",
        "mcp/filesystem",
        "/project"
      ],
      "env": {},
      "cwd": null
    }
  }
}
```

### MCP Usage Examples

```bash
# List available MCP servers
./run.sh mcp list
./run.sh mcp start filesystem
./run.sh mcp tools filesystem
./run.sh mcp call filesystem list_directory --args '{"path":"/project"}'
./run.sh mcp status
```

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
- `OPENROUTER_API_KEY`: Required for LLM features (planning, refinement, analysis)
- `CAI_MODE`: Execution mode for chat/workflows (`suggest` | `auto-edit` | `full-auto`)
- `CAI_VALIDATE`: Set to `after_all` to run validators after task execution
- `CAI_PROMPTS_DIR`: Set automatically from `--directory`; used for safe file URL resolution

### Notes
- MCP and Docker are optional; without MCP, tool execution may be limited
- Validation runs external commands; enable only when appropriate for your environment
