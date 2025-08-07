# CAI Session Management Feature

## Overview

The Session Management feature adds the ability to create, manage, and persist multiple named conversation sessions in CAI. Each session maintains its own memory context, allowing users to organize different conversation topics and maintain context across application restarts.

## Features

### Core Functionality
- **Named Sessions**: Create and manage multiple conversation sessions by name
- **Memory Isolation**: Each session has its own isolated memory context
- **Persistent Storage**: Sessions are automatically saved to disk and restored on startup
- **Auto-Save**: Current session is automatically saved when switching sessions
- **Session Export**: Export complete session history to markdown files

### Commands

#### @session-create(name)
Creates a new conversation session with the specified name.

```
@session-create(project-work)
@session-create(research-notes)
@session-create(daily-tasks)
```

**Requirements:**
- Session name must not be empty
- Session name cannot contain path separators (`/` or `\`)
- Session names must be unique

#### @session-switch(name)
Switches to an existing session, saving the current session's memory first.

```
@session-switch(project-work)
```

**Behavior:**
- Current session memory is auto-saved before switching
- Target session memory is loaded into active memory
- Session's last_accessed timestamp is updated

#### @session-list()
Lists all available sessions with creation and last access times.

```
@session-list()
```

**Output Example:**
```
Available sessions:
- daily-tasks (current)
  Created: 2025-01-15 09:30, Last accessed: 2025-01-15 14:22
- project-work
  Created: 2025-01-14 16:45, Last accessed: 2025-01-15 11:15
- research-notes
  Created: 2025-01-13 08:20, Last accessed: 2025-01-14 17:30
```

#### @session-current()
Shows detailed information about the currently active session.

```
@session-current()
```

**Output Example:**
```
Current session: 'project-work'
Created: 2025-01-14 16:45:30
Last accessed: 2025-01-15 14:22:15
Memory items: 12
```

#### @session-save()
Manually saves the current session (normally done automatically).

```
@session-save()
```

#### @session-delete(name)
Permanently deletes a session and its associated data.

```
@session-delete(old-project)
```

**Warning:** This operation cannot be undone.

#### @session-export(name, file-path)
Exports a complete session to a markdown file.

```
@session-export(project-work, ./exports/project-summary.md)
@session-export(research-notes, ~/Documents/research.md)
```

**Export Format:**
```markdown
# Session: project-work
Created: 2025-01-14 16:45:30
Last Accessed: 2025-01-15 14:22:15

## abc123ef (2025-01-14 16:45:30)
Type: Question

How do I implement error handling in Rust?

---

## def456gh (2025-01-14 16:46:15)
Type: Answer

In Rust, error handling is primarily done using the Result<T, E> type...

---
```

## Storage and Persistence

### File Location
Sessions are stored in JSON format in the user's data directory:
- **Linux/macOS**: `~/.local/share/cai/sessions/`
- **Windows**: `%APPDATA%/cai/sessions/`
- **Fallback**: `.cai/sessions/` in current directory

### File Format
Each session is stored as `{session-name}.json`:

```json
{
  "name": "project-work",
  "created": "2025-01-14T16:45:30Z",
  "last_accessed": "2025-01-15T14:22:15Z",
  "memory": {
    "abc123ef": {
      "id": "abc123ef",
      "date": "2025-01-14T16:45:30Z",
      "value": "How do I implement error handling in Rust?",
      "ptype": "Question"
    }
  },
  "config_overrides": null
}
```

### Auto-Save Behavior
- Sessions are automatically saved when switching between sessions
- Current session memory is preserved when creating new sessions
- Sessions are saved with updated `last_accessed` timestamps

## Integration with Existing Features

### Memory System
- Sessions use the existing `Prompt` and `PromptType` structures
- Memory isolation is achieved by swapping the global `MEMORY` HashMap contents
- All existing memory commands (`@get-memory`, `@export`, etc.) work within session context

### Command System
- Session commands integrate seamlessly with the existing command registry
- Session commands are classified as `CommandType::NotLLM` (not sent to AI)
- Commands follow the same pattern-matching system as other CAI commands

### Export Compatibility
- Session exports are compatible with the existing `@export` command format
- Session-exported files can be re-imported using `@read-file`

## Usage Examples

### Basic Workflow
```
# Create a new session for a coding project
@session-create(rust-web-server)

# Work on the project, ask questions, get responses...
How do I set up a basic HTTP server in Rust?

# Switch to another session for research
@session-create(web-frameworks-research)
@session-switch(web-frameworks-research)

# Research questions and answers are now in separate context
What are the most popular Rust web frameworks?

# Go back to coding project - all previous context is restored
@session-switch(rust-web-server)

# Export progress for documentation
@session-export(rust-web-server, ./project-notes.md)
```

### Session Management
```
# List all sessions
@session-list()

# Check current session
@session-current()

# Clean up old sessions
@session-delete(outdated-project)

# Manual save (if needed)
@session-save()
```

## Error Handling

The session system includes comprehensive error handling:

- **Invalid session names**: Names cannot be empty or contain path separators
- **Missing sessions**: Clear error messages when trying to switch to non-existent sessions
- **File system errors**: Graceful handling of permission issues or disk space problems
- **Serialization errors**: Proper error reporting for corrupted session files

## Future Enhancements

Potential future improvements to the session system:

1. **Session Configuration**: Per-session model settings, temperature, max tokens
2. **Session Templates**: Create sessions from predefined templates
3. **Session Search**: Search across all sessions for specific content
4. **Session Backup/Restore**: Automated backup and restore functionality
5. **Session Sharing**: Export/import sessions between CAI instances
6. **Session Statistics**: Usage analytics and memory usage statistics

## Technical Implementation

- **Core Module**: `src/session.rs` - SessionManager and core logic
- **Commands**: `src/commands/session_cmd.rs` - Command implementations  
- **Integration**: Commands registered in `src/commands/mod.rs`
- **Dependencies**: Uses `serde` for JSON serialization, `directories` for cross-platform paths
- **Thread Safety**: Uses `Arc<Mutex<>>` for safe concurrent access

The implementation maintains full backward compatibility with existing CAI functionality while adding powerful session management capabilities.