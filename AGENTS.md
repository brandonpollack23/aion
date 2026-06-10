# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project Overview

Aion is a Rust terminal calendar client with vim-style keybindings. It uses `ratatui`/`crossterm` for the TUI, `tokio` for async work, `rusqlite` for local storage, and native Google Calendar and CalDAV integrations.

The README still contains some older Bun/React packaging references. Treat `Cargo.toml`, `CLAUDE.md`, and the Rust source tree as the source of truth for development commands and architecture.

## Common Commands

- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- CI-equivalent quick check: `cargo check && cargo test`

CI currently runs:

- `cargo check`
- `cargo build --release`
- `cargo test`

## Repository Layout

- `src/main.rs` sets up panic handling, file logging, and starts the app.
- `src/app.rs` owns the terminal event loop, app lifecycle, background sync startup, and top-level `AppEvent` handling.
- `src/state/` contains mutable application and dialog state.
- `src/ui/` contains rendering code for the main layout and reusable views.
- `src/ui/overlays/` contains modal/dialog rendering.
- `src/keybinds/` contains command definitions, key registration, and key handling.
- `src/domain/` contains calendar/event/time/free-slot domain logic and natural date parsing.
- `src/api/` contains Google Calendar, CalDAV, iCal, and contacts integrations.
- `src/auth/` contains OAuth, token/account storage, and password-command helpers.
- `src/sync/` contains background sync logic.
- `src/db/` contains SQLite connection setup, migrations, event persistence, and sync tokens.
- `src/config/` contains config schema and loading/saving for `~/.config/aion/config.toml`.

## Development Notes

- Prefer small, idiomatic Rust changes that match the surrounding module style.
- Keep terminal cleanup behavior intact when touching `src/main.rs` or `src/app.rs`; raw mode and alternate screen must be restored on exit and panic paths.
- Avoid blocking the async runtime in UI or sync paths. If a change performs IO or network work, use existing async patterns or isolate blocking work carefully.
- Preserve local-first behavior: the app should continue using SQLite cache/offline data when API calls or configuration are unavailable.
- Be careful with time zones and all-day events. Reuse helpers in `src/domain/time.rs`, `src/domain/event.rs`, and `src/domain/natural_date.rs` rather than duplicating conversion logic.
- For Google/CalDAV changes, do not log tokens, passwords, authorization codes, or full credential payloads.
- For config changes, keep serde defaults/backward compatibility in `src/config/schema.rs` so existing config files keep loading.
- For UI changes, render through `ratatui` widgets and existing theme helpers in `src/ui/theme.rs`; keep layouts resilient to narrow terminal widths.
- For keybinding changes, update both the registry behavior and help/command visibility where appropriate.
- Use `anyhow::Result` consistently where the surrounding code already does.

## Testing Guidance

- Run `cargo test` for domain, parsing, API serialization, and state logic changes.
- Run `cargo check` after broader edits or signature changes.
- Run `cargo clippy` before handing off lint-sensitive changes.
- Run `cargo fmt` after editing Rust files.
- For TUI behavior, also run `cargo run` and manually verify the changed workflow where practical.

## Data And Local Files

Runtime files are outside the repository:

- Config: `~/.config/aion/config.toml`
- Contacts: `~/.config/aion/contacts.json`
- SQLite DB: `~/.local/share/aion/aion.db`
- Logs: `~/.local/share/aion/logs/`

Do not modify or delete a user's runtime config, database, logs, or credentials unless explicitly asked.

## Git Hygiene

- The working tree may contain user edits. Inspect `git status --short` before changing files and do not revert unrelated changes.
- Keep edits scoped to the requested behavior.
- Do not update `Cargo.lock` unless dependency changes require it.
- Do not make network-dependent dependency changes unless they are necessary for the task.
