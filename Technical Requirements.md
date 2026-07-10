# Ollama Cluster — Technical Requirements

## 1. Purpose

This document defines the technical requirements for implementing Ollama Cluster.

Ollama Cluster will provide:

- model-aware request routing across multiple Ollama instances;
- node health monitoring and automatic failover;
- model discovery and registry management;
- cluster administration through the `ocluster` command;
- streaming-compatible proxy behaviour;
- persistent configuration and operational state;
- an optional node agent for remote service and model management.

The initial implementation should prioritise:

- low proxy overhead;
- predictable behaviour;
- operational simplicity;
- compatibility with the Ollama API;
- a single-binary deployment model where practical;
- clear separation between inference traffic and cluster management.

---

# 2. Technology Stack

## TR-001 — Programming language

The primary implementation must use Rust.

Rust should be used for:

- the cluster controller;
- the inference proxy;
- the management API;
- the CLI;
- the interactive terminal interface;
- the optional node agent;
- shared protocol and domain libraries.

## TR-002 — Rust toolchain

The project must:

- use the stable Rust toolchain;
- specify a minimum supported Rust version;
- use Cargo workspaces;
- commit `Cargo.lock`;
- compile without warnings in CI;
- enforce formatting with `rustfmt`;
- enforce linting with Clippy.

## TR-003 — Recommended libraries

The initial implementation should use:

| Requirement                    | Recommended library             |
| ------------------------------ | ------------------------------- |
| Async runtime                  | Tokio                           |
| HTTP server and management API | Axum                            |
| HTTP client                    | Reqwest or Hyper                |
| Proxy transport                | Hyper or Pingora                |
| CLI parsing                    | Clap                            |
| Serialisation                  | Serde                           |
| TOML configuration             | toml                            |
| Terminal UI                    | Ratatui                         |
| Structured logging             | tracing                         |
| Metrics                        | metrics or prometheus           |
| TLS                            | rustls                          |
| Date and time                  | chrono or time                  |
| Error handling                 | thiserror and anyhow            |
| Database                       | SQLite through SQLx or rusqlite |
| File watching                  | notify                          |
| Unique identifiers             | uuid                            |

Alternative libraries may be used where there is a clear technical justification.

---

# 3. Project Structure

## TR-010 — Cargo workspace

The source repository should use a Cargo workspace.

Recommended structure:

```text
ollama-cluster/
├── Cargo.toml
├── crates/
│   ├── ocluster/
│   ├── ocluster-controller/
│   ├── ocluster-proxy/
│   ├── ocluster-agent/
│   ├── ocluster-core/
│   ├── ocluster-protocol/
│   ├── ocluster-client/
│   ├── ocluster-config/
│   ├── ocluster-storage/
│   └── ocluster-tui/
├── config/
├── packaging/
│   ├── systemd/
│   └── containers/
├── tests/
└── docs/
```

## TR-011 — Binary layout

The initial release should produce the following binaries:

```text
ocluster
ocluster-agent
```

The `ocluster` binary must support both:

- service/controller execution;
- CLI administration.

Example:

```bash
ocluster serve
ocluster status
ocluster nodes
```

The node agent should remain a separate binary because it operates with local host privileges.

## TR-012 — Shared domain layer

Cluster concepts must be defined in a shared core crate.

This should include:

- cluster state;
- node state;
- model state;
- request state;
- routing policy;
- health state;
- configuration types;
- management commands;
- event types.

Shared domain types must not depend directly on:

- Axum;
- Ratatui;
- SQLite;
- systemd;
- a specific proxy implementation.

---

# 4. Controller Architecture

## TR-020 — Controller responsibilities

The controller must be the authoritative owner of:

- registered nodes;
- node administrative state;
- discovered model inventories;
- effective model routing eligibility;
- active request counters;
- routing decisions;
- health state;
- recovery schedules;
- runtime configuration;
- cluster events.

## TR-021 — Internal services

The controller should be divided into internal components:

```text
Controller
├── Node Registry
├── Model Registry
├── Routing Engine
├── Health Manager
├── Request Tracker
├── Configuration Manager
├── Persistence Layer
├── Event Store
├── Metrics Service
└── Management API
```

Each component must expose explicit interfaces rather than directly mutating shared state.

## TR-022 — Concurrency model

The controller must use Tokio for asynchronous execution.

Shared mutable state should be handled through one or more of:

- actor-style tasks;
- channels;
- `Arc<RwLock<T>>`;
- concurrent maps;
- immutable snapshots.

Long-running network, storage or filesystem operations must not block the asynchronous runtime.

Blocking operations must run through:

```rust
tokio::task::spawn_blocking
```

or an equivalent dedicated thread pool.

## TR-023 — Internal messaging

Internal controller operations should use typed channels.

Recommended channel types:

- `mpsc` for commands and work queues;
- `broadcast` for cluster events;
- `watch` for current state snapshots;
- `oneshot` for command responses.

## TR-024 — Failure isolation

Failure in one background task must not terminate the entire controller process unless the failure makes safe operation impossible.

Critical background tasks must be supervised and restarted or trigger an orderly controller shutdown.

---

# 5. Network Interfaces

## TR-030 — Separate listeners

The controller must support separate listeners for:

- inference proxy traffic;
- management API traffic;
- metrics traffic.

Example:

```toml
[inference]
listen = "0.0.0.0:11434"

[management]
listen = "127.0.0.1:11600"

[metrics]
listen = "127.0.0.1:11601"
```

## TR-031 — Unix domain socket

The management interface should support a Unix domain socket.

Default path:

```text
/run/ocluster/ocluster.sock
```

The socket must support filesystem permissions that restrict access to authorised users or groups.

## TR-032 — Remote management

Where remote management is enabled, the management API must support:

- TLS;
- certificate verification;
- configurable authentication;
- request timeouts;
- request size limits;
- audit logging.

## TR-033 — Protocol support

The inference proxy must support:

- HTTP/1.1;
- persistent connections;
- chunked streaming responses;
- client cancellation;
- upstream timeouts.

HTTP/2 may be supported for client-facing and management interfaces, but Ollama compatibility must take priority.

---

# 6. Ollama API Compatibility

## TR-040 — Supported endpoints

The initial inference proxy must support routing for at least:

```text
POST /api/generate
POST /api/chat
POST /api/embed
POST /api/embeddings
```

Additional Ollama inference endpoints should be supported where they can be routed safely.

## TR-041 — Cluster-managed endpoints

The controller may provide cluster-level behaviour for:

```text
GET  /api/tags
GET  /api/ps
POST /api/pull
DELETE /api/delete
```

Cluster-level responses must be clearly documented because they may aggregate information from multiple nodes rather than mirror a single Ollama instance.

## TR-042 — Request compatibility

The proxy must preserve:

- HTTP method;
- headers where safe;
- request body;
- streaming preference;
- Ollama model identifiers;
- response status codes where appropriate.

## TR-043 — Response compatibility

The proxy must avoid modifying Ollama inference responses unless required for compatibility or observability.

Any additional cluster-specific response headers must use a documented namespace, for example:

```text
X-OCluster-Node
X-OCluster-Request-ID
X-OCluster-Retry-Count
```

Cluster headers must not include sensitive internal information unless explicitly enabled.

## TR-044 — Version compatibility

The controller must record the Ollama version for each node.

The system should support:

- minimum supported Ollama version checks;
- warnings for incompatible versions;
- mixed-version clusters;
- capability detection where endpoint support varies.

---

# 7. Streaming Proxy Requirements

## TR-050 — Zero full-response buffering

Streaming inference responses must be forwarded incrementally.

The proxy must not load the entire Ollama response into memory before sending it to the client.

## TR-051 — Backpressure

The proxy must propagate backpressure between:

- upstream Ollama response;
- proxy transport;
- downstream client.

## TR-052 — Cancellation

When a client disconnects, times out or cancels:

- the upstream request must be cancelled where possible;
- the request must be removed from active tracking;
- the node concurrency counter must be decremented;
- the cancellation must not count as a node failure unless the upstream independently failed.

## TR-053 — Retry boundary

The proxy must track whether downstream response bytes have been sent.

A request may be retried only when:

- no downstream response body has been sent;
- the error is classified as retryable;
- the retry limit has not been exceeded;
- another eligible node is available.

## TR-054 — Connection pooling

The proxy must maintain reusable upstream connection pools per node.

Pools must support configurable:

- maximum idle connections;
- idle timeout;
- connection timeout;
- maximum total connections;
- keep-alive behaviour.

## TR-055 — Request body handling

Request bodies may need to be replayed during a retry.

The proxy must therefore either:

- buffer request bodies up to a configured maximum size; or
- disable retries for requests that cannot be replayed safely.

Large request bodies must not be buffered without an explicit size limit.

## TR-056 — Streaming detection

The proxy must correctly handle both:

- streaming Ollama responses;
- non-streaming Ollama responses.

---

# 8. Request Routing Engine

## TR-060 — Candidate filtering

Before scoring nodes, the routing engine must exclude nodes that are:

- disabled;
- draining;
- drained;
- unavailable;
- recovering;
- over concurrency limits;
- missing the requested model;
- prohibited from serving the requested model.

## TR-061 — Model resolution

The routing engine must resolve:

- exact model names;
- optional aliases;
- tags;
- configured canonical model names.

Example:

```text
production-chat
    ↓
llama3.3:70b
```

## TR-062 — Routing score

The routing engine must support a configurable weighted score.

Possible inputs include:

```text
active request count
queued request count
model loaded state
node priority
recent failures
recent latency
node capacity
request token estimate
node warm-up state
```

Example conceptual score:

```text
score =
  active_requests × active_weight
  + queued_requests × queue_weight
  + cold_model_penalty
  + failure_penalty
  - node_priority_bonus
```

Lower scores should represent more desirable nodes.

## TR-063 — Deterministic tie-breaking

Where nodes have equal scores, the router should use deterministic or fair tie-breaking, such as:

- rotating selection;
- stable hash;
- least recently selected.

## TR-064 — Routing snapshots

Routing decisions should use an immutable or internally consistent snapshot of:

- node state;
- model eligibility;
- load state;
- routing configuration.

A request must not observe a partially updated registry.

## TR-065 — Queueing

If cluster-level queueing is enabled, queued requests must be managed through bounded queues.

The queue implementation must enforce:

- global maximum depth;
- per-model maximum depth;
- queue timeout;
- client cancellation;
- fairness between models;
- rejection when full.

## TR-066 — Admission control

The router must reject requests before proxying where:

- no node can service the model;
- the queue is full;
- the request exceeds configured limits;
- the cluster is shutting down;
- authentication or policy checks fail.

---

# 9. Node Registry

## TR-070 — Node identity

Each node must have:

- a unique internal identifier;
- a unique configured name;
- one Ollama endpoint;
- optional agent endpoint;
- labels;
- priority;
- routing weight;
- administrative state;
- discovered runtime state.

## TR-071 — Node labels

Nodes should support arbitrary key-value labels.

Example:

```toml
[nodes.labels]
location = "brisbane"
gpu = "rtx-4090"
environment = "production"
team = "ai"
```

Labels may later be used for:

- routing rules;
- placement;
- filtering;
- CLI selection;
- policy enforcement.

## TR-072 — Administrative and runtime state

The system must distinguish between:

```text
Administrative state
- enabled
- disabled
- draining

Runtime state
- ready
- suspect
- unavailable
- recovering
- warming
```

A runtime recovery must not override a manual administrative disablement.

## TR-073 — Node state persistence

Administrative node state must be persisted.

Transient runtime values may be rebuilt after restart.

## TR-074 — Node capability record

Each node must maintain capabilities including:

- Ollama version;
- available models;
- loaded models;
- model metadata;
- optional hardware metadata;
- optional GPU metadata;
- maximum configured concurrency.

---

# 10. Model Registry

## TR-080 — Registry data model

The model registry must record model-node relationships.

Required fields should include:

```text
node_id
model_name
model_digest
model_size
model_family
parameter_size
quantisation
modified_at
discovered
configured
permitted
available
loaded
last_seen_at
```

## TR-081 — Model source states

The registry must keep separate values for:

- discovered models;
- configured models;
- effective models;
- loaded models.

## TR-082 — Discovery endpoint

Model discovery must use the Ollama model listing endpoint.

The discovery client must:

- use a configurable timeout;
- validate response format;
- handle partial or invalid responses;
- update the registry atomically;
- preserve the last known state if discovery fails.

## TR-083 — Inventory fingerprint

Each node inventory must have a calculated fingerprint.

The fingerprint should be based on a stable sorted representation of:

```text
model name
digest
size
modified time
```

If the fingerprint has not changed, the controller should avoid unnecessary registry updates and events.

## TR-084 — Discovery scheduling

The discovery scheduler must support:

- discovery at startup;
- discovery after node registration;
- discovery after reconnection;
- discovery after model operations;
- manual discovery;
- periodic background discovery;
- jittered scheduling.

## TR-085 — Discovery modes

The configuration system must support:

```text
discover
allow
static
```

The effective model set must be calculated as follows.

### Discover

```text
effective = discovered
```

### Allow

```text
effective = discovered ∩ configured
```

### Static

```text
effective = configured, subject to runtime readiness validation
```

## TR-086 — Configuration drift

For static and allow modes, the controller must detect:

- configured model missing from node;
- unexpected model discovered;
- digest changed;
- model removed;
- model metadata changed.

Drift must be visible through:

- events;
- node inspection;
- health output;
- machine-readable management APIs.

---

# 11. Health Monitoring

## TR-090 — Passive health tracking

The proxy must report request outcomes to the health manager.

The health manager must classify failures, including:

- connection refused;
- connection timeout;
- DNS failure;
- network unreachable;
- HTTP server error;
- malformed response;
- upstream disconnect before response;
- upstream disconnect during streaming;
- client cancellation.

## TR-091 — Failure classification

Client cancellations must not be treated as node failures.

Application-level errors such as an invalid model request must not automatically mark the entire node unhealthy.

## TR-092 — Circuit breaker

Each node must have a circuit-breaker state.

Recommended states:

```text
closed
open
half-open
```

Behaviour:

- `closed`: normal routing;
- `open`: node excluded from routing;
- `half-open`: limited test traffic or recovery probes allowed.

## TR-093 — Recovery scheduler

Recovery checks must support:

- initial retry delay;
- exponential backoff;
- maximum delay;
- random jitter;
- maximum consecutive attempts before reduced-frequency monitoring;
- configurable success threshold.

## TR-094 — Recovery readiness

A successful health connection alone must not always return a node to routing.

Recovery may require:

- Ollama API responds;
- version endpoint succeeds;
- model discovery succeeds;
- requested configured models are available;
- node agent reports ready;
- configured warm-up checks pass.

## TR-095 — Healthy-node checks

Healthy nodes should be checked infrequently to detect silent failure while idle.

The interval must be configurable and should include jitter.

---

# 12. Persistence

## TR-100 — Persistent store

The initial release should use SQLite for local controller persistence.

SQLite should store:

- registered nodes;
- node configuration;
- administrative node state;
- model configuration;
- cluster configuration;
- model registry cache;
- recent cluster events;
- schema version;
- audit records.

## TR-101 — Storage location

Default storage location:

```text
/var/lib/ocluster/ocluster.db
```

User-mode installations may use:

```text
~/.local/share/ocluster/ocluster.db
```

## TR-102 — Database migrations

Database schema changes must use versioned migrations.

The controller must:

- detect pending migrations;
- apply supported migrations at startup;
- fail safely if migration cannot complete;
- avoid starting inference traffic with an invalid schema.

## TR-103 — Transactional updates

Updates involving multiple related records must use database transactions.

Examples include:

- node removal and associated model index removal;
- configuration apply;
- model inventory replacement;
- administrative state changes.

## TR-104 — Runtime cache

The controller may maintain an in-memory registry for routing performance.

Persistent storage must not be queried synchronously for every inference request.

## TR-105 — Event retention

Event retention must be configurable by:

- maximum age;
- maximum count;
- maximum database size.

---

# 13. Configuration

## TR-110 — Configuration format

The primary configuration format must be TOML.

Default system configuration location:

```text
/etc/ocluster/ocluster.toml
```

Default user configuration location:

```text
~/.config/ocluster/ocluster.toml
```

## TR-111 — Configuration layers

Configuration precedence must be:

```text
built-in defaults
→ configuration file
→ environment variables
→ command-line arguments
→ authorised runtime overrides
```

## TR-112 — Environment variables

Environment variables must use the prefix:

```text
OCLUSTER_
```

Example:

```text
OCLUSTER_MANAGEMENT_LISTEN
OCLUSTER_LOG_LEVEL
OCLUSTER_DATABASE_PATH
```

## TR-113 — Secrets

Secrets must not be stored in plaintext configuration where avoidable.

The system should support:

- environment variables;
- protected files;
- system credential stores;
- secret references.

CLI output must redact secrets by default.

## TR-114 — Configuration validation

Validation must occur:

- during startup;
- during `ocluster config validate`;
- before runtime configuration apply;
- before configuration reload.

Validation errors must include:

- configuration path;
- invalid field;
- reason;
- expected value or range.

## TR-115 — Atomic reload

Runtime configuration changes must be applied atomically.

If validation or application fails, the previous valid configuration must remain active.

## TR-116 — Configuration schema version

The configuration file should include a schema version.

Example:

```toml
version = 1
```

---

# 14. Management API

## TR-120 — API style

The management API should use JSON over HTTP.

It should expose versioned endpoints.

Example:

```text
/api/v1/cluster
/api/v1/nodes
/api/v1/models
/api/v1/requests
/api/v1/events
/api/v1/config
```

## TR-121 — Management operations

The API must support operations for:

- reading cluster status;
- listing nodes;
- inspecting nodes;
- adding and removing nodes;
- enabling and disabling nodes;
- draining nodes;
- probing nodes;
- synchronising models;
- listing and inspecting models;
- listing and cancelling requests;
- reading events;
- validating and applying configuration.

## TR-122 — Long-running operations

Long-running operations such as:

- model pull;
- node drain;
- node restart;
- model synchronisation across all nodes;

should return an operation identifier.

The API must support checking operation status.

Example:

```text
POST /api/v1/nodes/gpu-01/drain
GET  /api/v1/operations/{id}
```

## TR-123 — Idempotency

Administrative operations should be idempotent where practical.

Examples:

- enabling an enabled node should succeed without duplicate effects;
- disabling a disabled node should succeed;
- requesting the same model sync repeatedly should not corrupt state.

## TR-124 — API errors

Management API errors must use a structured format.

Example:

```json
{
    "error": {
        "code": "NODE_NOT_FOUND",
        "message": "Node 'gpu-01' does not exist",
        "details": {}
    }
}
```

## TR-125 — API version compatibility

The CLI must verify management API compatibility.

The controller must expose:

- API version;
- application version;
- supported features.

---

# 15. CLI Requirements

## TR-130 — CLI framework

The CLI must use structured subcommands.

Example:

```text
ocluster status
ocluster nodes
ocluster node inspect
ocluster node drain
ocluster models
ocluster model inspect
ocluster requests
ocluster config validate
```

## TR-131 — Output formats

Read commands must support:

```text
table
json
yaml
```

Example:

```bash
ocluster nodes --output json
```

## TR-132 — Stable machine output

JSON output must use a versioned and stable schema.

Human-readable output may change between releases, but machine-readable output should maintain backward compatibility within a major version.

## TR-133 — Exit codes

The CLI must use documented exit codes.

Recommended codes:

```text
0  success
1  general failure
2  invalid arguments
3  configuration error
4  controller unavailable
5  resource not found
6  operation rejected
7  partial success
8  authentication failure
```

## TR-134 — Remote contexts

The CLI should support named controller contexts.

Example:

```toml
[current]
context = "production"

[contexts.production]
endpoint = "https://ocluster.internal.example"

[contexts.local]
socket = "/run/ocluster/ocluster.sock"
```

Commands:

```bash
ocluster context list
ocluster context use production
```

## TR-135 — Command confirmation

Destructive commands must require confirmation unless one of the following is used:

```text
--yes
--force
--non-interactive
```

## TR-136 — Shell completion

The CLI should generate completion scripts for:

- Bash;
- Zsh;
- Fish;
- PowerShell.

---

# 16. Interactive Terminal Interface

## TR-140 — Terminal framework

The interactive terminal interface should use Ratatui.

## TR-141 — Data source

The TUI must use the management API or shared client library.

It must not directly access internal controller state or the SQLite database.

## TR-142 — Update transport

The initial TUI may use periodic management API refreshes.

A later version should support live updates through:

- server-sent events;
- WebSocket;
- streaming HTTP.

## TR-143 — Minimum views

The TUI should include:

- cluster overview;
- node list;
- node detail;
- model list;
- model detail;
- active requests;
- cluster events;
- metrics;
- logs;
- configuration summary.

## TR-144 — Actions

TUI actions must call the same management operations as the CLI.

No operation should exist only in the TUI.

---

# 17. Node Agent

## TR-150 — Optional deployment

The node agent must be optional.

Core inference routing and model discovery must work without an agent.

## TR-151 — Agent responsibilities

The agent may provide:

- Ollama service status;
- Ollama start;
- Ollama stop;
- Ollama restart;
- local model operations;
- local hardware information;
- GPU metrics;
- local log access;
- model change notifications.

## TR-152 — Restricted action model

The agent must expose only predefined operations.

It must not provide a generic remote command or shell execution endpoint.

## TR-153 — Local service integration

On Linux, the agent should support systemd service control.

Default Ollama service:

```text
ollama.service
```

The service name must be configurable.

## TR-154 — Agent authentication

Communication between controller and agent must be authenticated and encrypted when crossing a network.

Recommended options:

- mutual TLS;
- signed short-lived tokens;
- pre-shared credentials for small deployments.

## TR-155 — Agent registration

Agents must register or be configured with:

- node identity;
- controller identity;
- certificate or credential;
- capability list;
- supported agent version.

## TR-156 — Agent capability negotiation

The controller must not assume all agents support all operations.

The agent must report capabilities such as:

```text
service_control
model_pull
model_delete
log_streaming
gpu_metrics
filesystem_watch
```

---

# 18. Observability

## TR-160 — Structured logging

All services must produce structured logs.

Logs should include:

- timestamp;
- severity;
- component;
- request identifier;
- node identifier where relevant;
- model name where relevant;
- operation identifier where relevant;
- error classification.

## TR-161 — Log formats

The system must support:

- human-readable console logs;
- JSON logs.

## TR-162 — Request correlation

Every proxied request must receive an internal request identifier.

The identifier should be:

- included in logs;
- visible in request monitoring;
- returned in a response header;
- propagated to retries.

## TR-163 — Metrics

The controller must expose Prometheus-compatible metrics.

Minimum metrics:

```text
ocluster_requests_total
ocluster_requests_active
ocluster_requests_queued
ocluster_request_duration_seconds
ocluster_time_to_first_token_seconds
ocluster_upstream_failures_total
ocluster_retries_total
ocluster_node_ready
ocluster_node_active_requests
ocluster_node_recovery_attempts_total
ocluster_model_available
ocluster_model_loaded
ocluster_registry_sync_total
ocluster_registry_sync_failures_total
```

## TR-164 — Metric labels

Metrics labels must be controlled to avoid unbounded cardinality.

Request identifiers, user identifiers and raw prompts must not be metric labels.

## TR-165 — Health endpoints

The controller should expose separate endpoints for:

```text
/health/live
/health/ready
```

Liveness should indicate that the process is running.

Readiness should indicate that the controller can safely accept inference or management traffic.

---

# 19. Security

## TR-170 — Default network security

The management API must bind to localhost or a Unix socket by default.

Remote management must require explicit configuration.

## TR-171 — TLS

Remote management and agent communication must use TLS.

Rustls should be preferred over native OpenSSL dependencies where practical.

## TR-172 — Authentication

The management API must support at least one authentication mechanism before remote access is considered production ready.

Possible initial mechanisms:

- bearer token;
- mutual TLS;
- Unix socket permissions.

## TR-173 — Authorisation

The design must support future role-based access control.

Suggested roles:

```text
viewer
operator
model-admin
cluster-admin
```

## TR-174 — Audit logging

Administrative actions must create audit records containing:

- actor;
- action;
- target;
- timestamp;
- outcome;
- source address where relevant.

## TR-175 — Input validation

All external inputs must be validated, including:

- node URLs;
- model names;
- headers;
- configuration values;
- file paths;
- timeout values;
- CLI arguments.

## TR-176 — URL restrictions

The controller must protect against server-side request forgery when adding nodes.

It should support restrictions such as:

- approved CIDR ranges;
- approved hostname patterns;
- blocked loopback or metadata addresses where relevant;
- explicit allowlists.

## TR-177 — Sensitive request handling

Prompt and response content must not be logged by default.

Optional content logging must require explicit configuration and display a warning.

## TR-178 — Privilege separation

The controller should run as an unprivileged service user.

The node agent may require additional permissions, but these must be narrowly scoped.

---

# 20. Performance

## TR-180 — Proxy overhead objective

The proxy should add minimal latency relative to direct Ollama access.

A target for local-network deployments should be:

```text
less than 5 ms median added latency
```

This target excludes:

- model queueing;
- Ollama processing;
- model loading;
- network latency outside the proxy.

## TR-181 — Streaming latency

The proxy must forward the first upstream response bytes without waiting for the full response.

Time-to-first-token overhead should be measured separately from total request duration.

## TR-182 — Concurrency

The initial controller should support at least:

```text
1,000 concurrent streaming connections
```

on suitable commodity hardware, subject to operating-system limits and benchmark validation.

## TR-183 — Memory limits

Memory use must be bounded by:

- request-body buffer limits;
- queue depth limits;
- event retention;
- connection pool limits;
- log buffering limits.

## TR-184 — No synchronous hot-path storage

Inference routing must not require synchronous database writes before selecting a node.

Operational storage may be updated asynchronously where safe.

## TR-185 — Benchmark suite

The project should include benchmarks for:

- routing decision time;
- streaming proxy throughput;
- connection concurrency;
- request cancellation;
- model registry lookup;
- node state updates;
- JSON serialisation overhead.

---

# 21. Reliability

## TR-190 — Graceful shutdown

On shutdown, the controller must:

- stop accepting new management mutations;
- stop accepting new inference requests;
- optionally drain active requests;
- cancel background tasks;
- persist required state;
- close listeners cleanly.

## TR-191 — Panic handling

Recoverable input or upstream failures must not cause panics.

Panics in background tasks must be logged and surfaced through health status.

## TR-192 — State reconstruction

After restart, the controller must be able to rebuild transient runtime state by:

- loading persisted node configuration;
- restoring administrative states;
- probing nodes;
- rediscovering models;
- rebuilding the model routing index.

## TR-193 — Partial availability

The cluster must continue serving models where eligible nodes remain available, even if other nodes or models are unavailable.

## TR-194 — Database failure behaviour

If the persistent store becomes unavailable:

- existing inference routing may continue using in-memory state where safe;
- administrative mutations requiring persistence must fail clearly;
- degraded status must be exposed;
- the controller must avoid silently losing durable changes.

---

# 22. Deployment

## TR-200 — Systemd packaging

The project must provide systemd units for:

```text
ocluster.service
ocluster-agent.service
```

## TR-201 — Service user

The controller should run under a dedicated account:

```text
ocluster
```

Recommended directories:

```text
/etc/ocluster
/var/lib/ocluster
/var/log/ocluster
/run/ocluster
```

## TR-202 — Container image

The project should provide a container image for the controller.

The image should:

- run as a non-root user;
- expose configurable inference and management ports;
- support mounted configuration and storage;
- include health checks;
- use a minimal runtime base image.

## TR-203 — Static binaries

Where practical, release builds should provide self-contained Linux binaries.

## TR-204 — Supported platforms

The initial supported controller platform should be:

```text
Linux x86_64
```

Future support may include:

```text
Linux ARM64
macOS ARM64
macOS x86_64
Windows
```

The node agent should initially target Linux hosts running Ollama as a service.

---

# 23. Testing

## TR-210 — Unit testing

Unit tests must cover:

- routing score calculation;
- candidate filtering;
- state transitions;
- retry classification;
- model mode calculation;
- configuration validation;
- inventory fingerprinting;
- circuit-breaker behaviour.

## TR-211 — Integration testing

Integration tests must cover:

- controller-to-Ollama communication;
- multiple mock Ollama nodes;
- node failure and recovery;
- model discovery;
- request retry;
- streaming response forwarding;
- client cancellation;
- configuration reload;
- management API operations.

## TR-212 — Mock Ollama server

The test suite should include a configurable mock Ollama server capable of simulating:

- successful generation;
- streaming responses;
- slow responses;
- connection failures;
- HTTP errors;
- malformed responses;
- model inventory changes;
- mid-stream disconnects.

## TR-213 — End-to-end testing

End-to-end tests should run:

```text
CLI
→ management API
→ controller
→ proxy
→ Ollama or mock Ollama
```

## TR-214 — Failure testing

The test suite must include scenarios for:

- controller restart;
- node restart;
- network partition;
- node timeout;
- database lock;
- model removed during operation;
- all nodes for a model becoming unavailable;
- client disconnect during generation.

## TR-215 — Load testing

Load tests should measure:

- concurrent streams;
- routing latency;
- memory growth;
- connection reuse;
- queue behaviour;
- failure recovery;
- controller CPU utilisation.

---

# 24. Continuous Integration and Release

## TR-220 — CI checks

Every pull request must run:

```text
cargo fmt --check
cargo clippy
cargo test
cargo build --release
dependency audit
licence checks
```

## TR-221 — Security scanning

The project should use:

- `cargo audit`;
- dependency update automation;
- container scanning;
- secret scanning.

## TR-222 — Release artefacts

Releases should include:

- Linux binaries;
- checksums;
- container images;
- systemd units;
- sample configuration;
- shell completion scripts;
- migration notes;
- changelog.

## TR-223 — Versioning

The project should use semantic versioning.

The following must be versioned independently where necessary:

- application version;
- management API version;
- configuration schema version;
- database schema version;
- agent protocol version.

---

# 25. Initial Technical Scope

The recommended initial technical release should include:

1. Rust Cargo workspace.
2. Tokio asynchronous runtime.
3. Axum management API.
4. Hyper or Pingora streaming proxy.
5. TOML configuration.
6. SQLite persistence.
7. Static node registration.
8. Ollama connectivity checks.
9. Automatic model discovery.
10. Discover, allow and static model modes.
11. In-memory model-to-node routing index.
12. Least-active-request routing.
13. Loaded-model preference.
14. Passive node failure detection.
15. Circuit-breaker-based node ejection.
16. Recovery checks with exponential backoff.
17. Streaming request proxying.
18. Retry before downstream streaming begins.
19. Request cancellation handling.
20. `ocluster` CLI.
21. Human-readable and JSON output.
22. Structured logs.
23. Prometheus-compatible metrics.
24. Systemd service packaging.
25. Mock Ollama integration tests.

The following may be deferred:

- node agent;
- interactive Ratatui dashboard;
- remote management;
- user authentication;
- model pulling and deletion;
- GPU telemetry;
- live event streaming;
- controller high availability;
- distributed controller state;
- advanced token-aware scheduling;
- model placement automation.

---

# 26. Recommended Initial Architecture

```text
                         Client Applications
                                │
                                │ Ollama-compatible API
                                ▼
┌────────────────────────────────────────────────────────────┐
│                      ocluster controller                   │
│                                                            │
│  ┌────────────────┐       ┌────────────────────────────┐   │
│  │ Inference proxy│──────▶│ Routing engine             │   │
│  └────────────────┘       │ Model registry             │   │
│                           │ Node registry              │   │
│                           │ Request tracker            │   │
│                           └────────────────────────────┘   │
│                                      │                     │
│  ┌────────────────┐       ┌──────────▼─────────────────┐   │
│  │ Management API │       │ Health and recovery manager│   │
│  └────────────────┘       └────────────────────────────┘   │
│          │                           │                     │
│  ┌───────▼────────┐        ┌─────────▼─────────┐           │
│  │ CLI and TUI    │        │ SQLite persistence│           │
│  └────────────────┘        └───────────────────┘           │
└───────────────────────────────┬────────────────────────────┘
                                │
               ┌────────────────┼────────────────┐
               ▼                ▼                ▼
         Ollama node 1     Ollama node 2    Ollama node 3
```

This architecture keeps the first release relatively simple while preserving clean extension points for:

- node agents;
- remote administration;
- model placement;
- richer scheduling;
- distributed controllers;
- alternative inference runtimes.
