# Ollama Cluster (`ocluster`) 0.1.0

Ollama Cluster is a lightweight cluster management and intelligent routing platform for multiple Ollama instances. It provides model-aware request routing, automatic failover, node health monitoring, and a unified Ollama-compatible inference endpoint.

## Features (0.1.0)

- Cluster initialisation and TOML configuration
- Static node registration with automatic model discovery
- Model-aware routing (least-active-request + loaded-model preference)
- Streaming inference proxy (`/api/generate`, `/api/chat`, `/api/embed`, `/api/embeddings`)
- Passive failure detection and circuit-breaker ejection
- Pre-stream retry on alternate nodes
- Management REST API and CLI
- **Interactive terminal dashboard (Ratatui TUI)**
- **Web admin panel** (browser dashboard on port 11602)
- Prometheus metrics endpoint
- SQLite persistence and controller restart recovery

## Prerequisites

- **Rust 1.88+** (managed via `rust-toolchain.toml`)
- macOS or Linux for development (Linux systemd unit included for deployment)

### Install Rust tooling

If you do not have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The repository pins the toolchain automatically. From the project root:

```bash
rustup show   # installs 1.88 via rust-toolchain.toml
rustup component add rustfmt clippy
cargo install cargo-audit
```

## Build

```bash
cargo build --release
```

Binaries are written to:

```text
target/release/ocluster
target/release/ocluster-agent   # stub in 0.1.0
```

Install locally (optional):

```bash
cargo install --path crates/ocluster
```

## Quick start (macOS user mode)

### 1. Start mock or real Ollama nodes

With real Ollama on port 11434, skip this step. For testing, the included mock server works:

```bash
# Or use real Ollama at http://127.0.0.1:11434
```

### 2. Initialise the cluster

```bash
cargo run -p ocluster -- init \
  --inference-listen 127.0.0.1:11434 \
  --management-listen 127.0.0.1:11600 \
  --database-path ~/.local/share/ocluster/ocluster.db
```

Or register a node during init (use `@` between name and URL):

```bash
cargo run -p ocluster -- init \
  --node gpu-01@http://127.0.0.1:11434
```

Configuration is written to `~/.config/ocluster/ocluster.toml`.

### 3. Start the controller

```bash
cargo run -p ocluster -- serve
```

This starts:

| Service | Default address |
| ------- | --------------- |
| Inference proxy | `127.0.0.1:11434` |
| Management API | `127.0.0.1:11600` |
| Prometheus metrics | `127.0.0.1:11601` |

### 4. Add nodes (if not done at init)

```bash
cargo run -p ocluster -- node add gpu-01 --url http://127.0.0.1:11435
```

### 5. Check status

```bash
cargo run -p ocluster -- status
cargo run -p ocluster -- nodes
cargo run -p ocluster -- models
cargo run -p ocluster -- status --output json
```

### 6. Send inference through the cluster

Point any Ollama client at the proxy port:

```bash
curl http://127.0.0.1:11434/api/generate -d '{
  "model": "llama3.2:latest",
  "prompt": "Hello",
  "stream": false
}'
```

Response headers include:

- `X-OCluster-Node` — backend node name
- `X-OCluster-Request-ID` — correlation ID
- `X-OCluster-Retry-Count` — retry attempts

## Interactive TUI (Ratatui)

The cluster includes a terminal dashboard for live monitoring and node operations. It reads from the management API only (the controller must be running).

### Launch the dashboard

Running `ocluster` with no subcommand opens the TUI (same as `ocluster dashboard`):

```bash
# Controller must already be running (see Quick start step 3)
cargo run -p ocluster -- serve   # in one terminal

cargo run -p ocluster --          # TUI in another terminal
# or explicitly:
cargo run -p ocluster -- dashboard
```

After `cargo install --path crates/ocluster`:

```bash
ocluster              # default: launch dashboard
ocluster dashboard    # same
```

Point at a remote controller if needed:

```bash
OCLUSTER_MANAGEMENT_URL=http://192.168.1.10:11600 ocluster dashboard
ocluster --endpoint http://192.168.1.10:11600 dashboard
```

### Views

| Tab | Shows |
| --- | ----- |
| Overview | Cluster health, node counts, active requests |
| Nodes | Registered nodes and runtime state |
| Node | Detail for the selected node (Enter from Nodes) |
| Models | Discovered models across the cluster |
| Requests | In-flight inference requests |
| Events | Recent cluster events |
| Config | Loaded configuration summary |
| Help | Keybinding reference |

Data refreshes automatically every 2 seconds. Press `r` to refresh immediately.

### Keybindings

| Key | Action |
| --- | ------ |
| `Tab` / `Shift+Tab` | Switch views |
| `j` / `k` or `↑` / `↓` | Move list selection |
| `Enter` | Open node detail (Nodes view) |
| `r` | Refresh now |
| `?` or `F1` | Jump to Help |
| `q` or `Ctrl+c` | Quit |

**Node actions** (on Nodes or Node detail views):

| Key | Action |
| --- | ------ |
| `E` | Enable selected node |
| `d` | Disable selected node (confirm with `y`) |
| `D` | Drain selected node (confirm with `y`) |
| `p` | Probe selected node |
| `s` | Sync models across cluster |

Destructive actions show a confirmation prompt — press `y` to confirm or `n` / `Esc` to cancel.

## Web admin panel

The cluster includes a browser-based admin panel for monitoring cluster heartbeat, managing nodes, and viewing models. It runs on a separate port and proxies API calls to the management API.

### Launch the admin panel

```bash
# Terminal 1 — controller
cargo run -p ocluster -- serve

# Terminal 2 — admin panel (default http://127.0.0.1:11602)
cargo run -p ocluster -- admin
```

After install:

```bash
ocluster admin
ocluster admin --listen 0.0.0.0:11602   # bind all interfaces
```

Open **http://127.0.0.1:11602** in your browser.

Point at a remote controller:

```bash
OCLUSTER_MANAGEMENT_URL=http://192.168.1.10:11600 ocluster admin
ocluster --endpoint http://192.168.1.10:11600 admin --listen 127.0.0.1:11602
```

### Ports

| Service | Default address |
| ------- | --------------- |
| Inference proxy | `127.0.0.1:11434` |
| Management API | `127.0.0.1:11600` |
| Prometheus metrics | `127.0.0.1:11601` |
| **Web admin panel** | **`127.0.0.1:11602`** |

### Features

| Page | Description |
| ---- | ----------- |
| Dashboard | Cluster stats, heartbeat indicator, node ring visualisation, recent events |
| Nodes | List, add, edit, and remove nodes; click a row for detail |
| Node detail | Runtime state, enable/disable/drain/probe actions, per-node models |
| Models | Global model inventory with sync; click a model for per-node breakdown |
| Events | Full cluster event log |

**Polling:** the sidebar lets you set refresh interval (1s–30s, default 2s). The heartbeat indicator shows live connection status. Use **Refresh** for an immediate update.

**Node CRUD:** add nodes via the **Add node** button; edit URL and concurrency limits; remove with confirmation.

**Model management:** **Sync all models** on the Models page runs cluster-wide discovery; node detail pages show discovered models for that node and support probe/sync actions.

## CLI reference

```bash
ocluster dashboard                     # Terminal dashboard (Ratatui)
ocluster admin [--listen HOST:PORT]    # Web admin panel (default 127.0.0.1:11602)
ocluster init                          # Create configuration
ocluster serve                         # Run controller
ocluster status [--output json|yaml]
ocluster nodes
ocluster health
ocluster events
ocluster node add <name> --url <url>
ocluster node remove <name> --yes
ocluster node enable|disable|drain <name>
ocluster node inspect|probe <name>
ocluster models
ocluster model inspect <name>
ocluster explain <model>
ocluster requests
ocluster request cancel <id>
ocluster config show|validate|reload
```

Environment variables (prefix `OCLUSTER_`):

- `OCLUSTER_INFERENCE_LISTEN`
- `OCLUSTER_MANAGEMENT_LISTEN`
- `OCLUSTER_DATABASE_PATH`
- `OCLUSTER_LOG_LEVEL`
- `OCLUSTER_MANAGEMENT_URL` (CLI default endpoint)

## Configuration paths

| Platform | Config | Database |
| -------- | ------ | -------- |
| macOS (user) | `~/.config/ocluster/ocluster.toml` | `~/.local/share/ocluster/ocluster.db` |
| Linux (system) | `/etc/ocluster/ocluster.toml` | `/var/lib/ocluster/ocluster.db` |

See [config/ocluster.toml.example](config/ocluster.toml.example) for all options.

## Running tests

```bash
# Unit and integration tests
cargo test --workspace

# With clippy (matches CI)
cargo clippy --workspace -- -D warnings

# End-to-end tests (spawn controller + mock Ollama)
cargo test -p ocluster --test e2e
```

Test fixtures live in [tests/fixtures/](tests/fixtures/).

## Linux deployment

Copy the systemd unit from [packaging/systemd/ocluster.service](packaging/systemd/ocluster.service) and install the release binary to `/usr/local/bin/ocluster`.

## Architecture

```text
Client → ocluster proxy (:11434)
              ↓
         Routing engine → Ollama nodes
              ↓
         Management API (:11600) ← CLI / TUI / admin panel (:11602)
              ↓
         SQLite persistence
```

## Known limitations (0.1.0)

- `ocluster-agent` is a stub (no remote process control)
- No TLS, authentication, or RBAC on management API
- No model pull/delete via cluster
- Weighted/priority routing policies beyond default
- No controller high availability

## License

MIT
