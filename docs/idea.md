# CAI - Command-Augmented Interface

## Project Overview

CAI is a command-line interface tool designed to enhance interactions with large language models (LLMs) through a command-based approach. It allows users to seamlessly integrate local system operations with LLM conversations, creating a powerful environment for development, content creation, and information retrieval.

## Core Functionality

### Command System

The heart of CAI is its command system, which enables users to:

- Execute specialized commands within the chat interface
- Embed commands directly within prompts sent to LLMs
- Manage file operations from within the conversation
- Control model selection and configuration

Commands follow a simple syntax: `@command-name(parameter1, parameter2)` and can be used both as standalone inputs or embedded within larger prompts.

### Memory Management

CAI implements an intelligent memory system that:

- Tracks conversation history with unique IDs for each prompt and response
- Uses caret UUIDs for quick reference to previous messages
- Allows exporting conversation content to files
- Provides command access to previous conversation elements

### File Operations

The tool integrates deeply with the local file system:

- List files and folders matching patterns
- Read file and folder contents
- Export conversation segments to files
- Process multiple files in batches using wildcards

### Model Selection

CAI supports dynamic model selection through OpenRouter:

- List available LLM models
- Filter models by capabilities or provider
- Set preferred models for different tasks
- Save model preferences in configuration

## Technical Architecture

The project is built with Rust and organized around several key components:

- **Command Registry**: Central system for registering, parsing, and executing commands
- **Chat Loop**: Core interaction loop managing user input and model responses
- **OpenRouter Integration**: API connections to access multiple LLM providers
- **Configuration Management**: User settings persistence
- **File System Interface**: Abstraction layer for file operations

## Use Cases

CAI is particularly valuable for:

- Developers working on code projects who need contextual assistance
- Content creators managing multiple documents and references
- Researchers exploring and analyzing data sets
- Anyone who wants a more powerful, persistent interface to LLMs

## Future Directions

Potential enhancements include:

- Expanded command set for specialized tasks
- Support for additional LLM providers and APIs
- Improved conversation history visualization
- Enhanced memory management capabilities
- Custom command creation and scripting

## Getting Started

To use CAI, you'll need:

1. An OpenRouter API key (set as the OPENROUTER_API_KEY environment variable)
2. Basic familiarity with command-line interfaces
3. Rust installed on your system (for building from source)

Run the application with `cargo run` and start exploring the available commands with `@help()`.
