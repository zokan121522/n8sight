# Contributing to n8sight

Thanks for your interest in contributing! Here's how to get started.

## Development setup

1. Install Rust 1.75+ via [rustup](https://rustup.rs)
2. Clone the repo
3. Run with mock data — no n8n instance needed:

```bash
cargo run -- --mock
```

## Workflow

1. Fork and clone
2. Create a branch: `git checkout -b feat/my-feature`
3. Make changes
4. Ensure quality:

```bash
cargo fmt          # format code
cargo clippy       # lint
cargo test         # run tests
cargo build        # check it compiles
```

5. Open a pull request

## Project structure

```
src/
  main.rs           — Entry point + effect processing loop
  app.rs            — TUI state machine (update returns Vec<Effect>)
  action.rs         — Action enum (user inputs) + Effect enum (side effects)
  event.rs          — Terminal keyboard events → Action::Key
  scroll_state.rs   — Generic scrollable list with TableState
  cli_worker.rs     — Async API call serializer (mpsc channels)
  cli.rs            — Clap CLI argument definitions
  config.rs         — Layered config (CLI > env > file > defaults)
  logging.rs        — Tracing setup (file in TUI mode, stderr in CLI)
  tui.rs            — Terminal raw mode wrapper
  client/
    mod.rs          — N8nClient trait + filter types
    http.rs         — Production reqwest client
    mock.rs         — Mock client for development
  commands/
    workflow.rs     — `n8s workflow list/get/activate/deactivate`
    execution.rs    — `n8s execution list/get/retry`
    insights.rs     — `n8s insights`
    audit.rs        — `n8s audit`
    health.rs       — `n8s health`
  domain/
    workflow.rs     — WorkflowSummary, WorkflowDetail, WorkflowNode
    execution.rs    — ExecutionSummary, ExecutionDetail, NodeRunResult
    insights.rs     — InsightFinding, InsightsResult
    insights_compute.rs — Finding algorithms (failure rate, stuck, retry storms, etc.)
  widgets/
    mod.rs          — Main render dispatcher
    workflow_list.rs, workflow_detail.rs
    execution_list.rs, execution_detail.rs
    node_detail.rs, insights.rs
    tabs.rs, status_bar.rs, help.rs
  output/
    json.rs         — JSON formatter for CLI
    table.rs        — comfy-table formatter for CLI
```

## Key architectural decisions

- **TEA-inspired**: `App::update()` returns `Vec<Effect>` — side effects are processed externally in `main.rs`, keeping update testable.
- **Mode-aware key handling**: `EventHandler` sends raw `Action::Key(KeyEvent)`. `App::handle_key()` routes based on `InputMode` (Normal vs Filter) and current `View`.
- **ScrollState**: Generic wrapper around ratatui's `TableState` that handles selection bounds, page navigation, and scroll offset in one place.
- **Cached filtering**: `filtered_workflows()` / `filtered_executions()` cache results and only recompute when data or filters change.
- **No Utc::now() in render paths**: `app.now` is captured once per frame via `tick_frame()`.

## Adding a new view

1. Add the view variant to `View` enum in `app.rs`
2. Add navigation logic in `handle_back()` and `handle_select()`
3. Create a widget file in `widgets/`
4. Wire it up in `widgets/mod.rs` render dispatcher
5. Add any new key bindings in `handle_view_key()`

## Adding a new insight algorithm

1. Add a new function in `domain/insights_compute.rs`
2. Call it from `compute_insights()`
3. Add any new `InsightCategory` variants to `domain/insights.rs`

## Tests

Tests live alongside the code they test (`#[cfg(test)] mod tests`). Focus areas:

- `scroll_state.rs` — Navigation, bounds, pagination
- `domain/execution.rs` — Duration formatting, relative time, status counts
- `domain/insights_compute.rs` — Algorithm correctness (add more here!)

## Code style

- Run `cargo fmt` before committing
- No warnings from `cargo clippy`
- Prefer returning `Vec<Effect>` from state mutations over inline side effects
- Use `tracing::warn!` for recoverable errors, not silent `let _ =`
