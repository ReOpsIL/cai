# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust source modules and entry points.
- `tests/`: Integration tests run with `cargo test`.
- `prompts/`: Prompt templates and assets used by the app.
- `docs/`: Additional documentation and notes.
- `target/`: Build artifacts (ignored by Git).
- `run.sh` / `run.bat`: Convenience scripts for local runs.
- `mcp-config.json` (+ `mcp-config.example.json`): Local provider/config settings.

## Build, Test, and Development Commands
- `cargo build`: Compile in debug mode.
- `cargo run -- <args>`: Run the binary locally (passes args to the app).
- `cargo test`: Run unit/integration tests.
- `cargo fmt --all`: Format code with `rustfmt`.
- `cargo clippy -- -D warnings`: Lint; treat warnings as errors.
- `./run.sh` (or `run.bat` on Windows): Project launcher with sensible defaults.

## Coding Style & Naming Conventions
- Formatting: Use `rustfmt` (4-space indent, stable style). Run before committing.
- Linting: Keep `clippy` clean; prefer explicit types and avoid `unwrap()` in app code.
- Naming: modules/files `snake_case`; types/enums `CamelCase`; functions `snake_case`; constants `SCREAMING_SNAKE_CASE`.
- Structure: Prefer small modules; keep feature logic in `src/`, tests colocated or under `tests/`.

## Testing Guidelines
- Framework: Native Rust tests via `cargo test`.
- Organization: Unit tests in-module with `#[cfg(test)]`; integration tests in `tests/`.
- Naming: Use descriptive test names (e.g., `handles_empty_input`, `parses_config_file`).
- Expectations: Add tests for new behavior and regressions; keep tests deterministic.

## Commit & Pull Request Guidelines
- Commits: Imperative mood (“Add …”, “Fix …”); keep them focused. Optionally prefix with `feat:`, `fix:`, `refactor:`.
- References: Link issues with `#123` and summarize rationale in the body.
- PRs: Clear description, steps to reproduce, before/after output or screenshots (for UX/CLI), tests updated, and docs touched when relevant.

## Security & Configuration Tips
- Do not commit secrets. Prefer environment variables (e.g., `OPENAI_API_KEY`).
- To customize providers, copy `mcp-config.example.json` to `mcp-config.json` and edit locally.
- Validate config changes by running `cargo run -- --help` or `./run.sh` to ensure the app boots without errors.
