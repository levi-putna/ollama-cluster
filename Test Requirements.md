# Ollama Cluster: Test Requirements

## 1. Purpose

This document defines the test requirements for Ollama Cluster (`ocluster`).

It is derived from:

- [Functional Requirements.md](./Functional%20Requirements.md): functional behaviour and user-facing capabilities;
- [Technical Requirements.md](./Technical%20Requirements.md): implementation constraints, architecture, and quality attributes.

The purpose is to provide a single, traceable basis for:

- test planning and prioritisation;
- acceptance criteria for releases;
- verification that functional and technical requirements are met;
- regression coverage as the system evolves.

Each test requirement is identified as **TXR-NNN** and maps to one or more source requirements (**FR-***, **TR-***).

---

## 2. Scope

### 2.1 In scope for initial release

Testing must cover the minimum viable release described in FR §20 and TR §25, including:

1. Cluster initialisation (`ocluster init`).
2. Static node registration and lifecycle (add, remove, enable, disable, drain).
3. Automatic model discovery and discover / allow / static model modes.
4. Cluster model-to-node registry and synchronisation commands.
5. Model-aware routing with least-active-request policy and loaded-model preference.
6. Passive failure detection, circuit-breaker ejection, and recovery with exponential backoff.
7. Streaming proxy behaviour with retry-before-streaming and client cancellation.
8. Request monitoring and cluster visibility commands.
9. TOML configuration, validation, reload, and precedence.
10. Structured logging and Prometheus-compatible metrics.
11. SQLite persistence and controller restart recovery.
12. `ocluster` CLI with human-readable and JSON output, exit codes, and non-interactive operation.
13. Systemd service packaging and graceful shutdown.
14. Unit, integration, end-to-end, failure, and baseline load testing via a mock Ollama server.

### 2.2 Out of scope for initial release

The following may be covered by placeholder or smoke tests only until implemented:

- Node agent deployment and remote Ollama process control (FR-016, FR-049, TR-150–TR-156).
- Interactive Ratatui dashboard (FR-100–FR-104, TR-140–TR-144).
- Remote management with TLS and authentication (FR-142–FR-143, TR-032, TR-170–TR-173).
- Model pull and deletion through the cluster (FR-052–FR-053, TR-122).
- Controller high availability and leader election (FR-152).
- User management and role-based access control (FR-143, TR-173).
- GPU telemetry and advanced token-aware scheduling.
- Web-based administration.

Deferred features must not block initial release acceptance unless explicitly promoted into scope.

---

## 3. Test Principles

### TXR-001: Requirement traceability

Every test case must reference at least one FR or TR identifier.

Coverage reports should demonstrate that all in-scope FR and TR identifiers have associated tests before release.

### TXR-002: Test pyramid

Testing must follow a layered approach:

| Layer | Purpose | Typical tooling |
| ----- | ------- | --------------- |
| Unit | Pure logic, fast feedback | Rust `#[test]`, property tests where appropriate |
| Integration | Component interaction with mock dependencies | In-process controller + mock Ollama |
| End-to-end | Full CLI → API → controller → proxy → backend | Process-spawned binaries, HTTP clients |
| Failure / chaos | Resilience under adverse conditions | Controlled fault injection |
| Load / performance | Throughput, latency, resource use | Benchmark harness, load generator |

### TXR-003: Deterministic tests

Tests must be repeatable in CI without external network dependencies.

Non-deterministic timing must use configurable timeouts, polling with bounded retries, or injected clocks where practical.

### TXR-004: Isolation

Each integration and end-to-end test must use isolated temporary directories for configuration and SQLite databases.

Tests must not depend on a pre-existing cluster installation on the host.

### TXR-005: Fail-fast diagnostics

Failed tests must emit sufficient context to diagnose the failure without re-running interactively, including:

- command invoked;
- HTTP status and structured error body where applicable;
- relevant controller log excerpts;
- node and model state at failure time.

### TXR-006: Security-sensitive assertions

Tests must verify that prompts, completions, and secrets are not written to logs or metrics by default (FR-083, TR-113, TR-177).

---

## 4. Test Infrastructure

### TXR-010: Mock Ollama server

The test suite must include a configurable mock Ollama server (TR-212) capable of simulating:

- successful non-streaming generation;
- streaming JSON/NDJSON responses with configurable chunk timing;
- slow responses and delayed first token;
- connection refusal and abrupt disconnect;
- HTTP 4xx and 5xx errors;
- malformed or partial responses;
- model inventory listing and changes (`GET /api/tags`);
- loaded model state (`GET /api/ps`);
- mid-stream upstream disconnect;
- configurable Ollama version string.

The mock must support running multiple independent instances on ephemeral ports to represent a multi-node cluster.

### TXR-011: Test fixtures

The repository must provide reusable fixtures for:

- valid minimal cluster configuration (TOML);
- multi-node cluster configuration with mixed model modes;
- invalid configuration samples for validation tests;
- expected JSON schema snapshots for CLI `--output json` commands.

Fixtures must live under `tests/fixtures/` or an equivalent documented location.

### TXR-012: Test harness utilities

Shared test utilities must provide helpers to:

- start and stop a controller process with temporary config and database paths;
- register mock Ollama nodes programmatically;
- send Ollama-compatible inference requests through the proxy;
- poll until a node reaches an expected state or a timeout elapses;
- capture and parse structured logs and Prometheus metrics.

### TXR-013: CI environment

Continuous integration must run the full automated test suite on every pull request (TR-220):

```text
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
dependency audit
licence checks
```

CI must complete without warnings treated as errors and without flaky test failures above an agreed threshold (zero for blocking tests).

---

## 5. Test Levels and Coverage

### TXR-020: Unit tests

Unit tests must cover (TR-210):

| Area | Source requirements | Minimum scenarios |
| ---- | ------------------- | ----------------- |
| Routing score calculation | FR-063, TR-062 | Weighted score with varied active/queued counts, cold-model penalty, failure penalty, priority bonus; lower score wins |
| Candidate filtering | FR-060, TR-060 | Exclude disabled, draining, unavailable, over-limit, and model-missing nodes |
| State transitions | FR-020, FR-021 | Valid transitions (e.g. `ready → suspect → unavailable → recovering → ready`); reject invalid transitions |
| Retry classification | FR-074–FR-076, TR-053 | Retryable vs non-retryable errors; respect retry limit and alternate-node selection |
| Model mode calculation | FR-041, TR-085 | Discover, allow, and static effective model sets |
| Configuration validation | FR-003, TR-114 | Duplicate names, invalid URLs, invalid timeouts, conflicting values; error includes path, field, reason |
| Inventory fingerprinting | FR-048, TR-083 | Stable fingerprint for identical inventories; change detection on digest or size change |
| Circuit-breaker behaviour | FR-031–FR-032, TR-092 | Transitions among closed, open, and half-open states |

Unit tests must not require network I/O or SQLite unless testing the persistence layer in isolation with an in-memory database.

### TXR-021: Integration tests

Integration tests must cover (TR-211):

| Area | Source requirements | Minimum scenarios |
| ---- | ------------------- | ----------------- |
| Controller-to-Ollama communication | FR-010, TR-082 | Successful connect, version retrieval, model discovery |
| Multiple mock nodes | FR-044, TR-080 | Registry reflects models across nodes with correct readiness and loaded state |
| Node failure and recovery | FR-030–FR-034, TR-090–TR-094 | Ejection after threshold; recovery with backoff; return to routing after success threshold |
| Model discovery | FR-040–FR-048 | Discovery on startup, node add, manual sync, background interval |
| Request retry | FR-074, TR-053 | Pre-stream failure retries on alternate node; no retry after bytes sent |
| Streaming forwarding | FR-071, TR-050 | Incremental delivery without full-response buffering |
| Client cancellation | FR-073, TR-052 | Upstream cancelled; concurrency decremented; not counted as node failure |
| Configuration reload | FR-115, TR-115 | Valid reload applies atomically; invalid reload retains previous config |
| Management API | TR-120–TR-125 | CRUD operations for nodes, models, requests, events, config |

### TXR-022: End-to-end tests

End-to-end tests must exercise the full path (TR-213):

```text
CLI → management API → controller → proxy → mock Ollama
```

Minimum end-to-end scenarios:

1. `ocluster init` (non-interactive) → `ocluster serve` → register nodes → `ocluster status` reports ready cluster.
2. Inference via proxy `POST /api/chat` routed to correct node based on model availability.
3. `ocluster node disable` → subsequent requests avoid disabled node → `ocluster node enable` → node returns to pool.
4. `ocluster node drain` → no new assignments → drain completes when active requests finish.
5. `ocluster models sync` → registry updated → `ocluster model inspect` reflects changes.
6. `ocluster explain <model>` → lists eligible and rejected nodes with reasons.
7. `ocluster requests` and `ocluster request cancel` during active inference.
8. `ocluster config validate` and `ocluster config reload` via CLI.
9. Machine-readable output: `ocluster status --output json` parses against documented schema.

### TXR-023: Failure tests

Failure tests must cover (TR-214):

| Scenario | Expected behaviour |
| -------- | ------------------ |
| Controller restart | Persisted state restored; disabled nodes stay disabled; model index rebuilt (FR-022, FR-151, TR-192) |
| Node restart | Node marked unavailable; recovery after mock returns; models re-discovered |
| Network partition | Timeouts classified; circuit opens; no indefinite hang |
| Node timeout | Ejection after configured threshold |
| Database lock / corruption | Controller fails safely; does not serve inference with invalid schema (TR-194) |
| Model removed during operation | Routing index updated; in-flight requests handled per policy |
| All nodes for model unavailable | Queue or service-unavailable response per configuration (FR-065) |
| Client disconnect during generation | Upstream cancelled; node not penalised (TR-091) |

### TXR-024: Load and performance tests

Load tests should measure (TR-215, TR-180–TR-183):

- concurrent streaming requests across multiple nodes;
- median proxy-added latency (target: < 5 ms on local network, excluding Ollama processing);
- time-to-first-token overhead vs direct mock access;
- memory growth under sustained load;
- connection pool reuse (FR-077, TR-054);
- queue behaviour at configured limits (FR-066, TR-065);
- failure recovery under load;
- controller CPU utilisation at declared concurrency targets.

Performance tests may run outside the default PR pipeline but must run on a scheduled CI job or before release tagging.

---

## 6. Functional Test Requirements

### 6.1 Cluster initialisation

#### TXR-100: Initialise cluster (FR-001, FR-002, FR-003)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-100-01 | Non-interactive `ocluster init` with valid flags | Config created; endpoints configured; connectivity tested; service files created where applicable |
| TXR-100-02 | Interactive wizard (manual / automated pty) | Same outcomes as non-interactive path |
| TXR-100-03 | Init with config file input | Config merged and validated |
| TXR-100-04 | Duplicate node name in config | Validation failure with clear error |
| TXR-100-05 | Invalid node URL | Validation failure |
| TXR-100-06 | Unreachable node at init | Reported; init completes or fails per documented policy |
| TXR-100-07 | Invalid routing policy or timeout | Validation failure |

### 6.2 Node registration and management

#### TXR-110: Node lifecycle (FR-010–FR-017, FR-011)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-110-01 | `ocluster node add` with reachable mock Ollama | Node registered; version stored; models discovered; eligible for routing when ready |
| TXR-110-02 | Add node with invalid URL | Rejected with validation error |
| TXR-110-03 | Add duplicate node name | Rejected |
| TXR-110-04 | `ocluster node remove` with no active requests | Node removed from registry and routing index; audit event recorded |
| TXR-110-05 | Remove node with active requests | Warning; forced removal when `--force` supplied |
| TXR-110-06 | `ocluster node enable` after disable | Connectivity and readiness verified; model sync; routing eligible |
| TXR-110-07 | `ocluster node disable` | No new requests assigned; registration preserved |
| TXR-110-08 | `ocluster node drain` | No new assignments; active complete; state becomes drained |
| TXR-110-09 | Drain timeout with `--force` | Forced termination after timeout |
| TXR-110-10 | `ocluster node inspect` | Output includes name, address, version, state, models, concurrency, latency fields |
| TXR-110-11 | SSRF protection on node URL (TR-176) | Blocked loopback/metadata URLs when restrictions enabled |

#### TXR-111: Node state persistence (FR-020–FR-022, TR-072–TR-073)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-111-01 | Disabled node survives controller restart | Node remains disabled; not routed |
| TXR-111-02 | Draining node survives restart | Drain intent preserved or safely reconciled per design |
| TXR-111-03 | Runtime recovery does not override admin disable | Disabled node stays disabled despite mock recovery (TR-072) |

### 6.3 Health monitoring and recovery

#### TXR-120: Health and recovery (FR-030–FR-036, TR-090–TR-095)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-120-01 | Connection refused on inference | Immediate or threshold-based ejection per config |
| TXR-120-02 | Repeated HTTP 503 responses | Ejection after configured count |
| TXR-120-03 | Ejected node excluded from routing | No assignments until recovery |
| TXR-120-04 | Recovery with exponential backoff and jitter | Probes at increasing intervals; capped maximum |
| TXR-120-05 | Recovery success threshold | Node returns only after N consecutive successes |
| TXR-120-06 | `ocluster node probe` | Immediate health result returned |
| TXR-120-07 | Client cancellation not a node failure | Cancellation does not increment failure counter (TR-091) |
| TXR-120-08 | Application error (invalid model on node) | Does not mark entire node unhealthy (TR-091) |
| TXR-120-09 | Idle healthy node periodic check | Silent failure detected when configured (FR-035) |

### 6.4 Model discovery and registry

#### TXR-130: Discovery and registry (FR-040–FR-049, TR-080–TR-086)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-130-01 | Discovery on controller startup | All registered nodes scanned |
| TXR-130-02 | Discovery on node add | New node inventory indexed |
| TXR-130-03 | Discover mode | All discovered models routable |
| TXR-130-04 | Allow mode | Only discovered ∩ configured models routable |
| TXR-130-05 | Static mode | Only configured models routable; readiness validated |
| TXR-130-06 | Default mode for new node is discover | Unless explicitly overridden (FR-042) |
| TXR-130-07 | `ocluster models sync` (all nodes) | Registry updated on all nodes |
| TXR-130-08 | `ocluster node models sync <node>` | Single-node sync |
| TXR-130-09 | `--dry-run` sync | Reports added, removed, digest changes without applying |
| TXR-130-10 | Background discovery interval | Periodic refresh at configured interval |
| TXR-130-11 | Fingerprint unchanged | No unnecessary registry update or event (FR-048) |
| TXR-130-12 | Discovery failure | Last known state preserved (TR-082) |
| TXR-130-13 | Configuration drift detection | Visible in health, inspect, and events (TR-086) |
| TXR-130-14 | Cluster model index | `ocluster models` and `ocluster model inspect` show per-node readiness and loaded state |

#### TXR-131: Model aliases and restrictions (FR-054–FR-055, TR-061)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-131-01 | Request with alias | Resolved to canonical model; routed correctly |
| TXR-131-02 | Globally denied model | Rejected before proxying |
| TXR-131-03 | Per-node denied model | Node excluded from candidates |

### 6.5 Request routing

#### TXR-140: Routing engine (FR-060–FR-067, TR-060–TR-066)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-140-01 | Route to node with available, permitted model | Request succeeds |
| TXR-140-02 | Least-active-request policy | Request goes to node with fewest active requests |
| TXR-140-03 | Loaded-model preference | Prefer node with model loaded unless another is significantly less busy |
| TXR-140-04 | Per-node concurrency limit | Excess requests queued or rejected per config |
| TXR-140-05 | Per-node queue limit | Bounded queue enforced |
| TXR-140-06 | No eligible node: queue mode | Request queued up to depth and wait time |
| TXR-140-07 | No eligible node: reject mode | Clear service-unavailable response |
| TXR-140-08 | `ocluster explain <model>` | Eligible nodes, rejections with reasons, scores, preferred node |
| TXR-140-09 | Deterministic tie-breaking | Equal-score nodes handled fairly across repeated requests (TR-063) |
| TXR-140-10 | Routing snapshot consistency | Decision not based on partially updated registry (TR-064) |
| TXR-140-11 | Weighted and priority policies | Configurable policy selects expected node (when implemented beyond MVP default) |
| TXR-140-12 | Cluster shutting down | New requests rejected (TR-066) |

### 6.6 Proxy and streaming

#### TXR-150: Proxy behaviour (FR-070–FR-077, TR-040–TR-056)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-150-01 | `POST /api/generate` non-streaming | Compatible request/response; correct status codes |
| TXR-150-02 | `POST /api/chat` streaming | Chunks forwarded incrementally; no full buffering |
| TXR-150-03 | `POST /api/embed` and `POST /api/embeddings` | Routed correctly |
| TXR-150-04 | Cluster headers present | `X-OCluster-Node`, `X-OCluster-Request-ID` documented and non-sensitive (TR-043) |
| TXR-150-05 | Pre-stream upstream failure | Retry on alternate eligible node within limits |
| TXR-150-06 | Mid-stream upstream failure | No transparent retry; client receives appropriate error |
| TXR-150-07 | Client disconnect | Upstream cancelled; tracking cleaned up |
| TXR-150-08 | Backpressure | Slow client does not unboundedly buffer upstream (FR-072, TR-051) |
| TXR-150-09 | Large request body retry | Retries disabled or body buffered within explicit limit (TR-055) |
| TXR-150-10 | Connection reuse | Multiple sequential requests reuse upstream connections where possible |
| TXR-150-11 | Mixed Ollama versions | Version recorded; warnings for incompatible versions (TR-044) |
| TXR-150-12 | `GET /api/tags` cluster aggregation | Documented aggregate behaviour (TR-041) |

### 6.7 Request monitoring

#### TXR-160: Request visibility (FR-080–FR-083, TR-162)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-160-01 | `ocluster requests` during active inference | Lists ID, model, node, duration, streaming state |
| TXR-160-02 | `ocluster requests watch` | Live updates as requests start and complete |
| TXR-160-03 | `ocluster request cancel <id>` | Request terminated; node counters updated |
| TXR-160-04 | Request history | Metadata recorded; prompt/content not stored by default |
| TXR-160-05 | Request ID in logs and response header | Correlation across retries (TR-162) |

### 6.8 Cluster status and visibility

#### TXR-170: Status commands (FR-090–FR-093)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-170-01 | `ocluster status` | Summarises cluster state, nodes, models, requests, recent failures |
| TXR-170-02 | `ocluster nodes` | Lists nodes with state, address, load, latency, last contact |
| TXR-170-03 | `ocluster health` | Highlights unavailable, suspect, drift, models with no nodes |
| TXR-170-04 | `ocluster events` | Records node, model, config, drain, failure events |
| TXR-170-05 | Event retention limits | Old events pruned per configuration (TR-105) |

### 6.9 Configuration management

#### TXR-180: Configuration (FR-110–FR-116, TR-110–TR-116)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-180-01 | `ocluster config show` | Effective configuration displayed; secrets redacted |
| TXR-180-02 | `ocluster config validate` | Valid config exits 0; invalid exits non-zero with field-level errors |
| TXR-180-03 | Configuration precedence | CLI overrides env overrides file overrides defaults (FR-114, TR-111) |
| TXR-180-04 | `OCLUSTER_*` environment variables | Applied correctly (TR-112) |
| TXR-180-05 | Runtime apply without restart | Supported settings applied where practical (FR-113) |
| TXR-180-06 | Invalid reload | Previous config remains active (TR-115) |
| TXR-180-07 | Configuration rollback | Previous valid config restorable (FR-116) |
| TXR-180-08 | Schema version in config file | Parsed and validated (TR-116) |

### 6.10 Logging and metrics

#### TXR-190: Observability (FR-120–FR-124, TR-160–TR-165)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-190-01 | Structured logs for routing, health, discovery | Required fields present (TR-160) |
| TXR-190-02 | JSON log format option | Valid JSON log lines |
| TXR-190-03 | `ocluster logs` and `--follow` | Controller logs retrieved |
| TXR-190-04 | Prometheus metrics endpoint | Exposes minimum metric set (TR-163) |
| TXR-190-05 | Metric label cardinality | No request IDs or prompts as labels (TR-164) |
| TXR-190-06 | `/health/live` and `/health/ready` | Liveness vs readiness semantics correct (TR-165) |
| TXR-190-07 | Metrics reflect routing and failures | Counters increment on request, retry, node failure |

### 6.11 Service management

#### TXR-200: Service lifecycle (FR-130–FR-132, TR-190, TR-200)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-200-01 | `ocluster serve` as foreground process | Inference and management listeners start |
| TXR-200-02 | systemd unit start/stop/restart | Service behaves per packaging docs (TR-200) |
| TXR-200-03 | Graceful shutdown | Stops accepting new requests; drains optionally; persists state; clean connection close |
| TXR-200-04 | Panic in worker task | Does not crash entire controller (TR-191) |
| TXR-200-05 | Service runs as unprivileged user | Per packaging requirements (TR-201) |

### 6.12 Security and access control

#### TXR-210: Security (FR-140–FR-144, TR-170–TR-178)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-210-01 | Separate inference and management listeners | Distinct bind addresses configurable (FR-140, TR-030) |
| TXR-210-02 | Management defaults to localhost / Unix socket | Not exposed on all interfaces by default (TR-170) |
| TXR-210-03 | Unix socket permissions | Restricted to authorised group (TR-031) |
| TXR-210-04 | Administrative actions create audit records | Timestamp, action, target, outcome (FR-144, TR-174) |
| TXR-210-05 | Input validation on all external inputs | Invalid model names, paths, URLs rejected (TR-175) |
| TXR-210-06 | No prompt/response content in default logs | Verified under load (TR-177) |
| TXR-210-07 | Remote management (when enabled) | TLS and authentication required (deferred smoke test) |

### 6.13 Persistence and recovery

#### TXR-220: Persistence (FR-150–FR-151, TR-100–TR-104)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-220-01 | Registered nodes persist across restart | Node config and admin state restored |
| TXR-220-02 | Model configuration persists | Modes and permitted lists restored |
| TXR-220-03 | Database migrations on startup | Pending migrations applied; failure prevents unsafe start (TR-102) |
| TXR-220-04 | Transactional node removal | Node and model index updated atomically (TR-103) |
| TXR-220-05 | Hot path does not sync-query SQLite | Inference routing uses in-memory cache (TR-104) |
| TXR-220-06 | Post-restart recovery sequence | Reconnect, health refresh, model sync, index rebuild (FR-151) |

### 6.14 CLI output and automation

#### TXR-230: CLI behaviour (FR-160–FR-163, TR-130–TR-136)

| ID | Scenario | Expected result |
| -- | -------- | --------------- |
| TXR-230-01 | Default human-readable output | Clear terminal formatting |
| TXR-230-02 | `--output json` on read commands | Valid, schema-stable JSON (TR-132) |
| TXR-230-03 | `--output yaml` on read commands | Valid YAML |
| TXR-230-04 | Exit code 0 on success | All successful operations |
| TXR-230-05 | Exit codes 2–8 per scenario | Invalid args, config error, unavailable, not found, rejected, partial, auth (TR-133) |
| TXR-230-06 | Non-interactive destructive commands | Require `--yes`, `--force`, or `--non-interactive` (TR-135) |
| TXR-230-07 | Shell completion generation | Scripts generated for bash, zsh, fish, PowerShell (TR-136) |
| TXR-230-08 | CLI/API version compatibility check | Mismatch reported clearly (TR-125) |

---

## 7. Non-Functional Test Requirements

### TXR-300: Performance

| ID | Requirement | Acceptance criteria |
| -- | ----------- | ------------------- |
| TXR-300-01 | Proxy latency overhead (TR-180) | Median added latency < 5 ms vs direct mock on loopback, excluding Ollama processing |
| TXR-300-02 | Streaming first-byte latency (TR-181) | No measurable wait for full upstream response before first client byte |
| TXR-300-03 | Declared concurrency (TR-182) | Controller handles configured concurrent streams without deadlock or unbounded memory |
| TXR-300-04 | Memory limits (TR-183) | No unbounded growth during 30-minute sustained load test |

### TXR-310: Reliability

| ID | Requirement | Acceptance criteria |
| -- | ----------- | ------------------- |
| TXR-310-01 | Partial availability (TR-193) | Cluster continues serving models available on healthy nodes when others fail |
| TXR-310-02 | Database failure (TR-194) | Controller enters safe degraded state; does not corrupt data |
| TXR-310-03 | Failure isolation (TR-024) | Fault in one internal component does not terminate unrelated subsystems |

### TXR-320: Compatibility

| ID | Requirement | Acceptance criteria |
| -- | ----------- | ------------------- |
| TXR-320-01 | Supported platforms (TR-204) | Tests pass on declared Linux targets; macOS for development where supported |
| TXR-320-02 | Static binary smoke test (TR-203) | Release binary starts and serves health endpoint |
| TXR-320-03 | Container image smoke test (TR-202) | Image starts controller with sample config |

### TXR-330: Packaging

| ID | Requirement | Acceptance criteria |
| -- | ----------- | ------------------- |
| TXR-330-01 | Release artefacts (TR-222) | Binaries, checksums, systemd unit, sample config, completions present |
| TXR-330-02 | Semantic versioning (TR-223) | Version strings consistent across CLI, API, config schema, DB schema |

---

## 8. Deferred Feature Test Stubs

When deferred features are implemented, the following test groups must be added before they are considered production-ready:

| Feature area | Future test group | Source requirements |
| ------------ | ----------------- | ------------------- |
| Node agent | TXR-400 | FR-016, FR-049, TR-150–TR-156 |
| Interactive TUI | TXR-410 | FR-100–FR-104, TR-140–TR-144 |
| Remote management auth | TXR-420 | FR-142–FR-143, TR-032, TR-170–TR-173 |
| Model pull/delete via cluster | TXR-430 | FR-052–FR-053 |
| Controller HA | TXR-440 | FR-152 |

---

## 9. Acceptance Criteria for Initial Release

The initial release is acceptable for tagging when all of the following are true:

1. **Coverage**: Every in-scope FR and TR identifier in §2.1 has at least one mapped test case.
2. **Automation**: All TXR-020 through TXR-024 blocking tests pass in CI on every merge to the release branch.
3. **No critical defects**: No open defects classified as critical or high against in-scope requirements.
4. **MVP scenarios**: All TXR-100 through TXR-230 scenarios marked as MVP pass.
5. **Performance baseline**: TXR-300-01 and TXR-300-02 meet targets on reference hardware documented in the test report.
6. **Security baseline**: TXR-210-01 through TXR-210-06 pass.
7. **Recovery**: TXR-220-06 and TXR-023 controller-restart scenario pass.
8. **Documentation**: Test report includes environment, seed versions, and known limitations.

---

## 10. Traceability Matrix

A traceability matrix must be maintained mapping:

```text
FR/TR requirement → TXR test requirement → test case ID → automated (yes/no) → status
```

The matrix should live in `docs/traceability.md` or be generated from test annotations in source code (for example, `/// Covers: FR-060, TR-060` on test functions).

Minimum coverage targets for initial release:

| Requirement type | Target |
| ---------------- | ------ |
| In-scope FR identifiers | 100% mapped |
| In-scope TR identifiers | 100% mapped |
| TXR scenarios marked MVP | 100% automated |
| TXR performance scenarios | Benchmark job; manual review before release |
| Deferred features | 0% required until in scope |

---

## 11. Test Reporting

### TXR-500: Test execution report

Each release candidate must produce a test report containing:

- commit hash and version;
- CI run URL;
- pass/fail counts by test level;
- skipped tests with justification;
- performance benchmark results vs targets;
- known failures and waivers;
- coverage summary against traceability matrix.

### TXR-501: Defect severity definitions

| Severity | Definition |
| -------- | ---------- |
| Critical | Data loss, security breach, incorrect routing causing wrong model/node, cluster unavailable |
| High | Major feature broken with no workaround; persistent state corruption |
| Medium | Feature impaired with workaround; incorrect non-critical output |
| Low | Cosmetic, documentation, or minor CLI formatting issues |

---

## 12. Recommended Test Execution Order

For local and CI efficiency, run tests in this order:

1. Unit tests (`cargo test --lib`).
2. Integration tests (`cargo test --test '*'`).
3. End-to-end tests (may require `--ignored` or feature flag).
4. Failure tests (often longer; may run in parallel job).
5. Load/performance benchmarks (scheduled or pre-release only).

Flaky tests must be quarantined within one sprint: either fixed or explicitly marked ignored with a linked issue; ignored tests do not count toward §9 acceptance.

---

## 13. Document History

| Version | Date | Description |
| ------- | ---- | ----------- |
| 1.0 | 2026-07-10 | Initial test requirements derived from Functional and Technical Requirements |
