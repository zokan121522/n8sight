# n8sight

A real-time terminal dashboard for [n8n](https://n8n.io) — monitor workflows, track executions, catch failures, and visualize your automation fleet from the command line.

```
┌─ n8sight ──────────────────────────────────────────────────────────────────┐
│  ⚙ Workflows  │  ▶ Executions  │  ⚡ Insights                             │
├────────────────────────────────────────────────────────────────────────────┤
│ ┌ Quick Stats ───────────────────────────────────────────────────────────┐ │
│ │ TOTAL  20   ACTIVE  14 (70%) ██████████████  INACTIVE  6 (30%) ██████ │ │
│ └────────────────────────────────────────────────────────────────────────┘ │
│ ┌ Active Rate: 70% (14/20) ─────────────────────────────────────────────┐ │
│ │ ███████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │
│ └────────────────────────────────────────────────────────────────────────┘ │
│ ┌ Workflows ─────────────────────────────────────────────────────────────┐ │
│ │ ID     Name                     Active       Tags           Updated   │ │
│ │ 2XEs   PhoneValidator           ● active     production     284d ago  │ │
│ │ 0sKe   Shabaigs Email Listen…   ○ inactive                  90d ago   │ │
│ │ 1MsD   Follow Up Context        ● active                    284d ago  │ │
│ │ 07QO   service edit             ● active                    192d ago  │ │
│ └────────────────────────────────────────────────────────────────────────┘ │
│  j/k:nav  Enter:detail  a/i/0:filter  n/s/u:sort  /:search       ⟳ 12s  │
└────────────────────────────────────────────────────────────────────────────┘
```

## Why

n8n's web UI is great for building workflows. It's not great for monitoring a fleet of them. When you have 20+ active workflows firing thousands of executions, you need:

- A dashboard that auto-refreshes and shows what's happening **right now**
- Failure rates, stuck executions, and retry storms surfaced **immediately**
- The ability to drill into any execution and see per-node timing at a glance
- Something that works over SSH, in tmux, without a browser

n8sight is that.

## Install

```bash
# From source
git clone https://github.com/flancast90/n8sight.git
cd n8sight
cargo build --release
# Binary at ./target/release/n8s
```

Requires Rust 1.75+.

## Setup

### 1. Get your API key

1. Log in to your n8n instance
2. Go to **Settings → n8n API**
3. Click **Create an API key**
4. Copy the key

### 2. Configure

Create the config file at the OS-native config directory:

**macOS**: `~/Library/Application Support/n8sight/config.toml`
**Linux**: `~/.config/n8sight/config.toml`

```toml
api_url = "https://your-instance.example.com"
api_key = "your-api-key-here"
```

Or use environment variables:

```bash
export N8N_API_URL=https://your-instance.example.com
export N8N_API_KEY=your-api-key-here
```

### 3. Run

```bash
n8s          # launch the dashboard
n8s --mock   # demo mode with fake data (no n8n needed)
```

That's it. One command.

## Features

### Real-time dashboard

Auto-refreshes every 15 seconds. Press `p` to pause, `p` to resume. The countdown lives in the bottom-right corner.

### Workflows tab

Stats bar with active/inactive counts and percentages. Active rate gauge. Sortable, filterable table. Press Enter to see the workflow graph.

### Executions tab

Live success/error/running counts with percentages. Success rate gauge that goes green/yellow/red. Full-width sparkline showing execution frequency over time (one column per minute, fills the entire terminal width). Filterable by status.

```
┌ Quick Stats ─────────────────────────────────────────────────────────────────┐
│ TOTAL  487   ✓  421 (86%) █████████████████  ✗  31 (6%) █  ⟳  12  AVG 2.1s │
└──────────────────────────────────────────────────────────────────────────────┘
┌ Success Rate: 86% (421/487)  ·  Error Rate: 6% (31/487) ────────────────────┐
│ ████████████████████████████████████████████████████████████████████░░░░░░░░ │
└──────────────────────────────────────────────────────────────────────────────┘
┌ Execution Frequency (last 139min · peak: 41/min) ────────────────────────────┐
│ ▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▃▂▁▁▂▃▅▆▇██▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇ │
│ ▃▄▅▆▇██▇▆▅▄▃▂▁▁▂▃▅▆▇██▇▆▅▃▂▁▁▂▃▅▆▇███▇▆▅▄▃▂▁▁▂▃▄▅▆▇██▇▆▅▄▃▂▁▁▂▃▄▅▆▇██▇ │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Execution detail

Node-level waterfall timeline showing where time was spent. Each node rendered as a proportional bar with duration and percentage of total. Quick stats on node success rate, slowest node, total items processed.

```
┌ ⏱ Node Waterfall ────────────────────────────────────────────────────────────┐
│         Webhook █ 1ms (0%)                                                   │
│      Set Fields █ 3ms (0%)                                                   │
│              IF █ 1ms (0%)                                                   │
│      Send Email ██████████████████████████████████████████████ 450ms (96%)    │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Interactive workflow graph

Press Enter on any workflow to see its node graph rendered in ASCII with box-drawing characters. Pan with arrow keys or `h`/`j`/`k`/`l`. Tab cycles through nodes. Trigger nodes glow yellow, selected node highlighted in cyan.

```
┌──────────────────┐         ┌──────────────────┐         ┌──────────────────┐
│ Webhook          │────────▶│ Set Fields        │────────▶│ IF               │
│ webhook          │         │ set               │         │ if               │
└──────────────────┘         └──────────────────┘         └───────┬──────────┘
                                                                  │
                                                         ┌───────┘
                                                         │
                                                ┌────────▼─────────┐
                                                │ Send Email        │
                                                │ emailSend         │
                                                └──────────────────┘
```

### Fleet insights

Scans your instance and surfaces problems automatically:

| Finding | What it catches |
|---|---|
| **High Failure Rate** | Workflows with >50% error rate (critical) or >20% (warning) |
| **Stuck Execution** | Executions running/waiting for >30min (warning) or >2h (critical) |
| **Retry Storm** | Workflows with 3+ (warning) or 5+ (critical) retries |
| **Long Running** | Executions >3x the average for their workflow |
| **Abandoned Workflow** | Active workflows with no executions in 30+ days |
| **Inactive Critical** | Workflows tagged `production`/`critical` that are deactivated |

## Keyboard reference

### Navigation

| Key | Action |
|---|---|
| `j`/`k` or `↑`/`↓` | Move up/down |
| `g`/`G` | Jump to top/bottom |
| `Ctrl+D`/`Ctrl+U` | Half page down/up |
| `Enter` | Drill into detail |
| `Esc` | Go back |
| `Tab` | Cycle tabs (or cycle nodes in graph view) |
| `Alt+1`/`2`/`3` | Jump to Workflows/Executions/Insights |

### Filtering & sorting

| Key | Context | Action |
|---|---|---|
| `/` | Any list | Text search |
| `a` | Workflows | Show active only |
| `i` | Workflows | Show inactive only |
| `0` | Workflows | Clear filter |
| `1`–`5` | Executions | Filter: error/running/success/waiting/canceled |
| `0` | Executions | Clear status filter |
| `n` | Any list | Sort by name |
| `s` | Any list | Sort by status |
| `u` | Workflows | Sort by updated |
| `d` | Executions | Sort by duration |

### Actions

| Key | Action |
|---|---|
| `r` | Manual refresh |
| `p` | Pause/resume auto-refresh |
| `A` | Activate/deactivate workflow |
| `R` | Retry failed execution |
| `x` | Copy URL to clipboard |
| `o` | Open in browser |
| `?` | Help overlay |
| `q` (×2) | Quit |

### Graph view

| Key | Action |
|---|---|
| `h`/`j`/`k`/`l` or arrows | Pan the graph |
| `Tab` | Select next node |
| `0` | Reset pan to origin |
| `Esc` | Back to workflow list |

## Architecture

Built in Rust with [ratatui](https://ratatui.rs). Single binary, no runtime dependencies.

```
src/
  main.rs            One entry point. TUI only. Effect processing loop.
  app.rs             State machine — update(action) → effects. Zero side effects.
  action.rs          Action enum (inputs) + Effect enum (outputs)
  event.rs           Terminal events → raw Key actions
  scroll_state.rs    Generic scrollable list with ratatui TableState
  cli_worker.rs      Async API call serializer via mpsc channels
  config.rs          Layered config: CLI flags > env vars > config file
  client/            N8nClient trait + HTTP (reqwest) + mock implementations
  domain/            Business logic: workflow/execution models, insight algorithms
  widgets/           TUI rendering: graphs, charts, gauges, tables, sparklines
```

### Key design decisions

- **TEA-inspired**: `App::update()` returns `Vec<Effect>`. Side effects processed externally. Update is testable.
- **Mode-aware keys**: Event handler sends raw `KeyEvent`. App dispatches based on current view + input mode. No key bleed across views.
- **Cached filtering**: Filtered lists computed once, invalidated on change. Not recomputed every frame.
- **No `Utc::now()` in render**: `app.now` captured once per frame via `tick_frame()`.
- **Node runs parsed once**: Execution JSON parsed on load, cached in `cached_node_runs`. Zero re-parsing per frame.

## n8n API coverage

| Endpoint | Used for |
|---|---|
| `GET /workflows` | Workflow list with filtering |
| `GET /workflows/{id}` | Full detail: nodes, connections, settings |
| `POST /workflows/{id}/activate` | Activate from TUI |
| `POST /workflows/{id}/deactivate` | Deactivate from TUI |
| `GET /executions` | Execution list with status/workflow filtering |
| `GET /executions/{id}?includeData=true` | Per-node execution data |
| `POST /executions/{id}/retry` | Retry from TUI |

## Contributing

```bash
cargo run -- --mock   # TUI with fake data
cargo test            # 9 tests
cargo clippy          # Zero warnings
cargo fmt             # Format
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for project structure and architecture guide.

## License

MIT — [Finn Lancaster](https://github.com/flancast90)
