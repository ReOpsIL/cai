# Autocomplete Implementation Plan

## Overview

This document outlines the plan for implementing autocomplete for commands and prompt history in the Cai application. Due to the constrained environment, a simplified approach will be used where suggestions are displayed when the user presses a specific key (e.g., Tab).

## Limitations

*   Limited access to system resources.
*   Cannot directly manipulate the terminal or VS Code interface.
*   Reliance on standard input/output for user interaction.

## Approach

Implement a simplified autocomplete feature where the application displays a list of suggestions when the user presses a specific key (e.g., Tab). The user can then select a suggestion by typing its number and pressing Enter.

## Implementation Details

### 1. Command Autocomplete

*   When the user types "@" and presses Tab, display a list of available commands.
*   The commands will be hardcoded in the `command_handler` module.
*   The user can select a command by typing its number and pressing Enter.
*   The selected command will be inserted into the input buffer.

### 2. Prompt History Suggestions

*   When the user presses Tab without typing "@", display a list of recent prompt snippets.
*   The prompt snippets will be extracted from the prompt history.
*   The user can select a prompt snippet by typing its number and pressing Enter.
*   The selected prompt snippet will be inserted into the input buffer.

### 3. Input Handling Module

*   Create a new module called `input_handler` to handle the keyboard input and suggestion selection logic.

### 4. Main Loop Modification

*   Modify the main loop in `chat.rs` to use the `input_handler` module to read user input.

## Mermaid Diagram

```mermaid
graph LR
    A[Chat Loop] --> B{Input?};
    B -- "@" + Tab --> C[Display Command Suggestions];
    B -- Tab --> D[Display Prompt History Suggestions];
    C --> E{Select Command?};
    D --> F{Select Prompt?};
    E -- Yes --> G[Insert Command];
    E -- No --> A;
    F -- Yes --> H[Insert Prompt];
    F -- No --> A;
    G --> A;
    H --> A;
    B -- Other Input --> I[Send to OpenRouter];
    I --> A;