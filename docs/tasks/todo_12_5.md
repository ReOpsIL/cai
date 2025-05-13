# prompt handling
- Users stories:

* add automatic **uuid** to each prompt and each response - print uuid when as headline befor user input block.
* The user can copy store the prompt / result as memory using uuid
* on @set-memory command `_` represent last results chat uuid  and `?` represent last prompt question chat uuid
* User wants to save a prompt or response in memory by providing chat uuid and *description*
  @set-memory(chat-uuid, headline)

* User saves the last prompt results using last chat-id uuid
  @set-memory(_, headline)

* User saves the last prompt question using last chat-id uuid
  @set-memory(?, headline) and press enter ..

* User wants to run a prompt allready saved in memory without storing its output.
  @get-memory(memory-id) and press enter ..
  will print mempry content.

* User wants to run a prompt allready saved in memory and store result in memory (two operations ).
  @get-memory(memory-id)
  **advanced** @enter() # `(eg. same as pressing Enter)`. Will call openrouter - send the prompt and wait for results, print results (from openrouter).
  @set-memory(_,headline)

* User wants to run a prompt allready saved in memory on input.
  example: Use the folowing prompt (named prompt-id) to process the poc idea: "a small application for reading news".
   @get-memory(memory-id) and press enter ..

* user wants to export prompt in markdown or pdf
  @export-memory-pdf(memory-id)
  @export-memory-md(memory-id)

# DONT forget - check that a LLM response with embedded commands acctualy work (save files , run prompt on embeded content)


Here is a **Design and Specification Document** for your LLM-embedded command-based chat app, written in a way that an LLM (or human developer) can implement the system in Rust. It is modular, includes key responsibilities, examples, and Rust-specific considerations:

---

# 📄 Design and Specification Document

**Project**: Prompt-Embedded LLM Chat App in Rust
**Author**: \[Your Name]
**Version**: 1.0
**Date**: 2025-05-12

---

## 🧩 Overview

This project is a lightweight chat interface that embeds inline commands (prefixed with `@`) into user prompts. Commands enable memory management, prompt chaining, exporting, and conditional logic.

The backend is implemented in **Rust**, calling LLM APIs (e.g. via `OpenRouter`) and storing state (e.g. in-memory or optionally persisted to disk). Users interact using natural language augmented with commands like `@set-memory`, `@get-memory`, `@compose(...)`, etc.

---

## 📐 High-Level Architecture

### Components:

1. **Command Parser**
2. **Prompt Engine (LLM Runner)**
3. **Memory Store**
4. **Command Dispatcher**
5. **Export & Output Renderer**
6. **CLI / Web TUI (optional)**

```plaintext
User Input
   ↓
[Command Parser] → [Command Dispatcher] → [Memory Store / Prompt Engine / Exporter]
   ↓                          ↑
Final Response ←─────────────┘
```

---

## 🎯 Features & Use Cases

| Feature          | Description                                     |
| ---------------- | ----------------------------------------------- |
| UUID tracking    | Every prompt/response has a UUID and timestamp  |
| Command handling | Commands like `@set-memory(_)` processed inline |
| Memory storage   | Save/retrieve/reuse prompts/responses           |
| Export           | Markdown/PDF export of saved content            |
| Pipelines        | Use prompts as functions via chaining           |
| Simple logic     | Commands like `@loop`, `@if`, `@compose`        |

---

## 🧱 Data Structures (Rust)

### PromptEntry

```rust
struct PromptEntry {
    uuid: Uuid,
    content: String,
    is_prompt: bool,
    timestamp: DateTime<Utc>,
    headline: Option<String>,
    tags: Vec<String>,
}
```

### MemoryStore

```rust
struct MemoryStore {
    entries: HashMap<Uuid, PromptEntry>,
    last_prompt: Option<Uuid>,
    last_result: Option<Uuid>,
}
```

---

## 📜 Commands Specification

### 🔹 Syntax

All commands are prefixed by `@`, may take parameters in parentheses `()`:

```plaintext
@set-memory(_, "Project Init")
@chain(prompt-uuid -> summarizer-uuid)
```

### 🔹 Core Commands

| Command             | Signature                      | Description                |                  |                       |
| ------------------- | ------------------------------ | -------------------------- | ---------------- | --------------------- |
| `@set-memory`       | \`@set-memory(uuid             | \_                         | ?, "headline")\` | Save prompt or result |
| `@get-memory`       | `@get-memory(memory-id)`       | Retrieve memory            |                  |                       |
| `@compose`          | `@compose(id1 + id2 + ...)`    | Merge memory entries       |                  |                       |
| `@chain`            | `@chain(id1 -> id2)`           | Run id1, use result in id2 |                  |                       |
| `@export-memory-md` | `@export-memory-md(memory-id)` | Export as Markdown         |                  |                       |
| `@enter`            | `@enter()`                     | Run current prompt         |                  |                       |
| `@list-memory`      | `@list-memory()`               | List saved prompts         |                  |                       |
| `@tag`              | `@tag(memory-id, tag1, tag2)`  | Add tags                   |                  |                       |
| `@filter-by-tag`    | `@filter-by-tag(tag)`          | Show matching entries      |                  |                       |

---

## ⚙️ LLM Execution Flow

```rust
fn execute_prompt(prompt: &str) -> Result<String, LlmError> {
    // Send to OpenRouter or API client and return response
}
```

Command sequence (e.g., `@get-memory -> @enter -> @set-memory`) can be modeled as a **pipeline of operations** with intermediate context.

---

## 🧪 Example Flow

```plaintext
User Input:
    "Generate an app idea. @set-memory(_, 'app idea')"

Parsed Command Execution:
1. Run LLM on input → "How about a voice-controlled to-do app?"
2. Save result to memory with UUID, headline "app idea"
```

---

## 📤 Exports

```rust
fn export_to_markdown(entry: &PromptEntry) -> String
fn export_to_pdf(entry: &PromptEntry) -> Result<PathBuf, ExportError>
```

---

## 📁 Storage Options

* In-memory (default): `HashMap<Uuid, PromptEntry>`
* Optional: serialize to disk (JSON, SQLite)
* Optional: load/export full session

---

## 🖥️ CLI Interface (Optional)

Use `clap` or `tui-rs`:

```bash
> @list-memory
> @set-memory(_, "My Summary")
> @chain(prompt1 -> summarizer)
```

---

## 📦 Dependencies (Cargo.toml)

```toml
[dependencies]
uuid = "1"
chrono = "0.4"
serde = { version = "1.0", features = ["derive"] }
reqwest = { version = "0.11", features = ["json"] }
regex = "1"
pdf-writer = "0.8"
```

---

## 📌 LLM API Abstraction

```rust
trait LlmClient {
    fn run_prompt(&self, prompt: &str) -> Result<String, LlmError>;
}
```

* Implement this for `OpenRouter`, `OpenAI`, etc.
* Use `reqwest` for HTTP calls.

---

## ✅ To Implement

* [ ] Command parser: `@command(arg1, ...)` using `regex`
* [ ] Command dispatcher engine
* [ ] Prompt execution engine
* [ ] Memory manager
* [ ] Export system
* [ ] CLI or Web frontend (optional)

---

Would you like this saved as a downloadable `.md` or `.pdf` file as well?
