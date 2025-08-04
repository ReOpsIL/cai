# Prompt Manager CLI

A Rust CLI application for managing and searching prompt collections stored in YAML files.

## Features

- **YAML-based prompt storage**: Organize prompts in structured YAML files
- **Hierarchical organization**: File → Subject → Prompt structure
- **Powerful search**: Search across file names, subjects, and prompt content
- **CLI interface**: Easy-to-use command-line interface with colored output
- **Flexible querying**: Query specific prompts or browse collections

## Installation

### Quick Start (Recommended)

Use the provided run scripts for the easiest experience:

**Linux/macOS:**
```bash
./run.sh --help              # Show all options
./run.sh --build list        # Build and list prompts
./run.sh chat                # Start chat mode
```

**Windows:**
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

**List all prompts:**
```bash
# Linux/macOS
./run.sh list

# Windows  
run.bat list
```

**Search prompts:**
```bash
# Linux/macOS
./run.sh search "performance"
./run.sh search "security"

# Windows
run.bat search "performance"
run.bat search "security"
```

**Show specific prompt file:**
```bash
# Linux/macOS
./run.sh show bug_fixing

# Windows
run.bat show bug_fixing
```

**Query specific prompt:**
```bash
# Linux/macOS
./run.sh query bug_fixing "Performance Issues" "Performance bottleneck analysis"

# Windows
run.bat query bug_fixing "Performance Issues" "Performance bottleneck analysis"
```

**Specify custom prompts directory:**
```bash
# Linux/macOS
./run.sh --directory /path/to/prompts list

# Windows
run.bat --directory C:\path\to\prompts list
```

### Direct Binary Usage

If you prefer to use the compiled binary directly:

```bash
./target/release/cai list
./target/release/cai search "performance"
./target/release/cai show bug_fixing
./target/release/cai query bug_fixing "Performance Issues" "Performance bottleneck analysis"
./target/release/cai --directory /path/to/prompts list
```

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

- **Local files**: `file://path/to/prompt.md` (relative to current directory)
- **HTTP/HTTPS**: `https://example.com/prompt.md` for online prompts
- **Absolute paths**: `file:///absolute/path/to/prompt.md`

URL references allow you to:
- Store large prompts in separate markdown files
- Share prompts across multiple YAML files
- Use online prompt repositories
- Keep YAML files clean and focused

## Sample Prompts Included

- **Bug Fixing**: Error analysis, performance issues, concurrency bugs
- **Code Analysis**: Architecture review, code quality, security analysis
- **Task Creation**: Project planning, documentation, testing strategy
- **Refactoring**: Clean code practices, performance optimization, modernization

## Commands

### Prompt Management
- `list`: Display all available prompt files and their contents
- `search <query>`: Search for prompts containing the query string
- `show <file_name>`: Display detailed view of a specific prompt file
- `query <file> <subject> <prompt>`: Retrieve a specific prompt
- `chat`: Start interactive chat mode for AI-powered task planning and prompt management

### MCP (Model Context Protocol) Support
- `mcp list`: List configured MCP servers and their status
- `mcp start <server>`: Start an MCP server
- `mcp stop <server>`: Stop an MCP server
- `mcp tools <server>`: List tools available from a server
- `mcp call <server> <tool> --args <json>`: Call a tool with arguments
- `mcp resources <server>`: List resources available from a server
- `mcp status`: Show MCP server status overview

## Chat Mode Features

### Interactive Task Planning
```bash
# Linux/macOS
./run.sh chat

# Windows
run.bat chat

# Or with direct binary
./target/release/cai chat
```

The chat mode provides:
- **AI-powered task planning**: Input any request and get a structured task breakdown
- **Smart prompt management**: Automatically adds, updates, or scores existing prompts
- **Similarity detection**: Prevents duplicate prompts and improves existing ones
- **Automatic categorization**: Tasks are intelligently sorted into appropriate subjects

### How Chat Mode Works

1. **Task Planning**: Enter a request and the AI generates actionable tasks
2. **Similarity Analysis**: Each task is compared against existing prompts
3. **Smart Repository Management**:
   - **New prompts**: Added if no similar prompts exist
   - **Prompt updates**: Similar prompts are improved and merged
   - **Score increment**: Exact matches get higher relevance scores
4. **Self-curating repository**: High-quality prompts emerge through usage patterns

### Setup

Set your OpenRouter API key:
```bash
export OPENROUTER_API_KEY="your_api_key_here"
```

Get your API key from: https://openrouter.ai/

## Prompt Scoring System

Each prompt includes a score field that tracks usage and effectiveness:
- Prompts start with score 0
- Score increases when the prompt is matched in chat interactions
- High-scoring prompts indicate proven usefulness
- Scores are displayed as ⭐ icons in listings

## Search Capabilities

The search function looks for matches in:
- File names
- File descriptions  
- Subject names
- Prompt titles
- Prompt content (including URL-referenced content)

Results show the match type and context for easy navigation.

## MCP (Model Context Protocol) Integration

CAI now supports MCP servers, enabling integration with external tools and services through Docker containers. This allows you to extend the application with capabilities like file system access, database operations, web browsing, and more.

### MCP Configuration

MCP servers are configured via a JSON file. On first use, CAI creates a default `mcp-config.json`:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-v", "/local-directory:/local-directory",
        "mcp/filesystem",
        "/local-directory"
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

# Start the filesystem server
./run.sh mcp start filesystem

# List tools from the server
./run.sh mcp tools filesystem

# Call a tool with arguments
./run.sh mcp call filesystem read_file --args '{"path": "/tmp/example.txt"}'

# Check server status
./run.sh mcp status
```

### Supported MCP Server Types

The configuration supports various Docker-based MCP servers:

- **Filesystem**: File operations and directory management
- **Database**: SQL queries and data operations  
- **Web**: Browser automation and web scraping
- **APIs**: REST API interactions and integrations
- **Custom**: Any Docker container implementing MCP protocol

### Configuration Locations

CAI searches for MCP configuration in:
1. `./mcp-config.json` (current directory)
2. `./.mcp-config.json` (hidden file)
3. `~/.config/cai/mcp-config.json` (user config)

### Technical Implementation

- Uses official Rust MCP SDK (`rmcp` crate)
- Supports Docker-based server management
- Async/await architecture for non-blocking operations
- JSON-RPC communication over stdin/stdout
- Configuration-driven server lifecycle management