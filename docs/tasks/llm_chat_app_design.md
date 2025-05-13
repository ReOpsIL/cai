
# 📄 Design and Specification Document

**Project**: Prompt-Embedded LLM Chat App in Rust

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

```
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
| short-uuid tracking    | Every prompt/response has a short-uuid and timestamp  |
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
    short-uuid: short-uuid,
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
    entries: HashMap<short-uuid, PromptEntry>,
    last_prompt: Option<short-uuid>,
    last_result: Option<short-uuid>,
}
```

---

## 📜 Commands Specification

### 🔹 Syntax

All commands are prefixed by `@`, may take parameters in parentheses `()`:

```
@set-memory(_, "Project Init")
@chain(prompt-short-uuid -> summarizer-short-uuid)
```

### 🔹 Core Commands

| Command             | Signature                      | Description                |
| ------------------- | ------------------------------ | -------------------------- |
| `@set-memory`       | `@set-memory(short-uuid, "headline")`| Save prompt or result      |
| `@get-memory`       | `@get-memory(short-uuid)`       | Retrieve memory            |
| `@compose`          | `@compose(id1 , id2 , ...)`    | Merge memory entries       |
| `@chain`            | `@chain(id1 -> id2)`           | Run id1, use result in id2 |
| `@export-memory-md` | `@export-memory-md(short-uuid)` | Export as Markdown         |
| `@enter`            | `@enter()`                     | Run current prompt         |
| `@list-memory`      | `@list-memory()`               | List saved prompts         |
| `@tag`              | `@tag(short-uuid, tag1, tag2)`  | Add tags                   |
| `@filter-by-tag`    | `@filter-by-tag(tag)`          | Show matching entries      |


🧠 Memory & Prompt Manipulation
@list-memory()
Show all saved memory entries (UUID + headline + type: prompt/result + timestamp).

@delete-memory(memory-id)
Remove a stored memory item.

@rename-memory(memory-id, new-headline)
Rename an existing memory item.

@search-memory(keywords)
Search saved memory for matching headlines or content.

@compare-memory(memory-id1, memory-id2)
Show side-by-side diff between two prompts or responses.

🛠️ Prompt Composition & Processing
@compose(memory-id1 + memory-id2 + ...)
Combine multiple prompts/responses into one prompt.

@chain(memory-id1 -> memory-id2)
Feed result of memory-id1 as input to memory-id2 (pipeline-style).

@replace(memory-id, placeholder, new-value)
Replace a placeholder in a prompt from memory (e.g., {{idea}}).

@summarize(memory-id)
Generate a summary of the prompt or response stored.

🔧 Input Augmentation / Preprocessing
@template(prompt-text, var1=..., var2=...)
Dynamically fill a template prompt stored in memory.

@extract(key, text)
Use OpenRouter to extract structured data like emails, tasks, code.

@translate(language)
Translate the last result (_) to a target language.

🔁 Execution Flow & Control
@delay(seconds)
Wait X seconds before next command.

@if(condition, then=@command, else=@command)
Basic conditional logic using prompt results or stored flags.

@loop(memory-id, times)
Run a memory prompt multiple times (e.g., for variation or retries).

@schedule(time, @command)
Run a command at a future time (basic scheduling).

🗂️ Tags, Labels & Organization
@tag(memory-id, tag1, tag2)
Add tags for filtering/grouping prompts.

@list-tags()
Show all used tags.

@filter-by-tag(tag)
Show all memory items tagged with tag.

📤 Export & Sharing
@share(memory-id)
Generate a shareable link (if integrated with backend or cloud).

@export-all(format=md|pdf|json)
Export all memory items in one go.

🔍 Prompt Testing / A/B Comparison
@test-prompt(memory-id, input-text)
Test a saved prompt against a new input.

@ab-test(memory-id1 vs memory-id2, input)
Run both prompts with the same input and compare results.


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

```
User Input:
    "Generate an app idea. @set-memory(_, 'app idea')"

Parsed Command Execution:
1. Run LLM on input → "How about a voice-controlled to-do app?"
2. Save result to memory with short-uuid, headline "app idea"
```

---

## 📤 Exports

```rust
fn export_to_markdown(entry: &PromptEntry) -> String
fn export_to_pdf(entry: &PromptEntry) -> Result<PathBuf, ExportError>
```

---

## 📁 Storage Options

* In-memory (default): `HashMap<short-uuid, PromptEntry>`
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
short-uuid = "1"
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
