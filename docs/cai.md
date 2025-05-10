# Cai - OpenRouter Chat Application

## Features

*   **Chat with OpenRouter API:** Allows users to interact with various AI models through the OpenRouter API.
*   **Configuration File:** Loads settings from `~/cai.conf` on startup, including the selected model.
*   **Model Selection:**
    *   The `@set-model` command lists available models from OpenRouter.
    *   Users can filter models by providing a wildcard.
    *   The selected model is saved to the `~/cai.conf` configuration file.
*   **Prompt Saving:**
    *   The `@save-prompt <prompt_name>` command saves the last prompt to a file in the `prompts` directory.
*   **Prompt History:**
    *   Maintains a history of recent prompts.
    *   The `@save-history` command saves the prompt history to `prompt_history.txt`.
    *   Prompt history is also saved when the application exits.
*   **Ctrl-d Exit:**
    *   The application can be exited by pressing Ctrl-d or typing "exit".

## Functionality

The application loads its configuration from `~/cai.conf`, which includes the selected model. If the configuration file does not exist, it is created with a default model.

The main loop reads user input, stores it in the prompt history, and then either sends the prompt to the OpenRouter API or executes a command.

The `@set-model` command allows the user to select a new model from the list of available models. The selected model is then saved to the configuration file.

The `@save-prompt` command saves the last prompt to a file in the `prompts` directory.

The `@save-history` command saves the prompt history to a file named `prompt_history.txt`.

The application can be exited by pressing Ctrl-d or typing "exit".