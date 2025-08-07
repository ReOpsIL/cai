# Autonomous Looping Workflow Feature

## Overview

The Autonomous Looping Workflow feature enables CAI to execute complex, multi-step tasks through an iterative plan -> execute -> verify -> continue loop system.

## Key Concepts

- **Workflow Plan**: A high-level goal broken down into executable steps
- **Steps**: Individual actions that can be executed using CAI commands
- **Verification**: Automated success checking using different strategies
- **Iterations**: Repeated execution cycles until completion or max limit

## Available Commands

### Starting a Workflow
```
@start-loop(goal, max_iterations, verification_strategy)
```

**Parameters:**
- `goal`: Description of what you want to achieve
- `max_iterations`: Maximum number of loop cycles (1-50)
- `verification_strategy`: How to verify success
  - `file_exists`: Check if expected files exist
  - `command_success`: Check if commands execute successfully
  - `llm_validation`: Use LLM to evaluate success
  - `combined`: Use multiple verification methods
  - `pattern`: Custom regex pattern to match in outputs

**Example:**
```
@start-loop(Set up a new Rust project with error handling, 10, file_exists)
```

### Continuing Workflow Execution
```
@continue-loop(plan_id)
```

Execute the next iteration of the workflow loop. Repeat this command until the workflow completes.

### Workflow Management

#### Check Status
```
@workflow-status(plan_id)
```
Shows current progress, steps completed, and iteration count.

#### List All Workflows
```
@list-workflows()
```
Shows all active workflows with their status.

#### Pause/Resume/Stop
```
@pause-workflow(plan_id)    # Pause execution
@resume-workflow(plan_id)   # Resume paused workflow
@stop-workflow(plan_id)     # Stop and mark as failed
```

### Advanced Commands

#### Manual Step Execution
```
@execute-step(plan_id, step_id)
```
Execute a specific step manually for debugging.

#### Verification Check
```
@verify-workflow(plan_id)
```
Run verification check on current workflow state.

## Workflow Example

1. **Start the workflow:**
```
@start-loop(Create a Python web scraper for news articles, 8, llm_validation)
```

2. **CAI automatically creates a plan:**
```
Plan ID: abc123
Steps:
1. Create project directory
2. Set up virtual environment  
3. Install required packages (requests, beautifulsoup4)
4. Write basic scraper script
5. Test the scraper
6. Add error handling
```

3. **Continue execution:**
```
@continue-loop(abc123)
```

4. **Monitor progress:**
```
@workflow-status(abc123)
```

5. **Repeat continue command until completion**

## Verification Strategies

### FileExists
- Checks if expected files are created during execution
- Good for: Project setup, file generation tasks

### CommandSuccess  
- Verifies commands execute without errors
- Good for: Installation tasks, build processes

### LLMValidation
- Uses LLM to evaluate if the goal was achieved
- Good for: Complex tasks requiring semantic understanding

### Combined
- Uses multiple verification methods for higher confidence
- Good for: Critical tasks requiring thorough validation

### OutputPattern
- Matches command outputs against regex patterns
- Good for: Tasks with predictable output formats

## Integration with Existing Features

- **Memory System**: Workflow plans are stored in CAI's memory with UUID-based IDs
- **Command System**: Works with all existing CAI commands (@read-file, @bash-cmd, etc.)
- **Export**: Workflow logs can be exported using `@export(~, filename.md)`
- **File Operations**: Automatically integrates with CAI's file reading/writing capabilities

## Best Practices

1. **Clear Goals**: Write specific, actionable goals
   - Good: "Create a Rust CLI tool with clap for argument parsing"
   - Bad: "Make a tool"

2. **Reasonable Iterations**: Start with 5-10 iterations for most tasks

3. **Appropriate Verification**: Choose verification strategy based on task type
   - Code projects: `combined` or `llm_validation`
   - File operations: `file_exists`
   - System tasks: `command_success`

4. **Monitor Progress**: Use `@workflow-status` to track execution

5. **Handle Failures**: Use `@pause-workflow` to debug issues, then `@resume-workflow`

## Troubleshooting

### Workflow Not Progressing
- Check `@workflow-status(plan_id)` for failed steps
- Use `@execute-step(plan_id, step_id)` to manually run problematic steps
- Consider using `@verify-workflow(plan_id)` to understand verification issues

### Planning Issues
- Make goals more specific and actionable
- Ensure LLM API is working with `@get-memory` commands
- Check that required files/dependencies are available

### Verification Failures
- Switch to `llm_validation` for complex tasks
- Use `combined` strategy for better accuracy
- Manually verify expected outcomes and adjust approach