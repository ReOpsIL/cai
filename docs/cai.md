# Cai - OpenRouter Chat Application

## Features

*   **Chat with OpenRouter API:** Allows users to interact with various AI models through the OpenRouter API.
*   **Configuration File:** Loads settings from `~/cai.conf` on startup, including the selected model.
*   **Model Selection:**
    *   The `@set-model` command lists available models from OpenRouter.
    *   Users can filter models by providing a wildcard.
    *   The selected model is saved to the `~/cai.conf` configuration file.
*   **Memory Management:**
    *   All prompts and responses are assigned unique caret UUIDs for reference.
    *   UUIDs are displayed after each interaction for easy reference.
    *   The `@get-memory(ID)` command allows retrieving previous content by ID.
    *   The `@reset-memory()` command clears the stored conversation memory.
*   **Prompt History:**
    *   Maintains a history of recent prompts.
    *   The `@save-history` command saves the prompt history to `prompt_history.txt`.
    *   Prompt history is also saved when the application exits.
*   **Content Export:**
    *   The `@export(ID, filename)` command exports memory content to a file.
    *   Special ID flags allow exporting specific content types (? for questions, _ for answers, @ for all).
*   **Ctrl-d Exit:**
    *   The application can be exited by pressing Ctrl-d or typing "exit".

## Functionality

The application loads its configuration from `~/cai.conf`, which includes the selected model. If the configuration file does not exist, it is created with a default model.

The main loop reads user input, stores it in the prompt history, and then either sends the prompt to the OpenRouter API or executes a command.

### Memory and UUID System

Each prompt (user input) and response (AI output) is assigned a unique caret UUID, which is displayed after each interaction. This serves as a reference identifier for later retrieval or export operations. UUIDs are derived from standard UUIDs but shortened to just the first segment for easier reference.

The memory system maintains a timestamped record of all conversation elements, enabling:
- Retrieval of specific prompts or responses with `@get-memory(ID)`
- Exporting selected content to external files with `@export(ID, filename)`
- Clearing the conversation history with `@reset-memory()`

### Command System

The application supports both standalone commands and embedded commands within prompts. Commands follow the syntax `@command-name(parameters)` and include:

- `@set-model(filter)` - Select and configure the AI model
- `@get-memory(ID)` - Retrieve content from memory by its ID
- `@export(ID, filename)` - Export memory content to a file
- `@reset-memory()` - Clear the stored conversation memory
- `@help()` - Display available commands and usage information
- Various file management commands (read-file, list-files, etc.)

The application can be exited by pressing Ctrl-d or typing "exit".
