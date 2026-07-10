# Ollama Cluster — Functional Requirements

## 1. Purpose

Ollama Cluster is a lightweight cluster management and intelligent routing platform for multiple Ollama instances.

The system must provide:

- model-aware request routing;
- automatic failover between Ollama nodes;
- node and model discovery;
- cluster health monitoring;
- operational controls for starting, stopping, draining and restarting nodes;
- command-line and interactive terminal management;
- visibility into node, model and request status.

The primary terminal command and service name will be:

```bash
ocluster
```

---

# 2. Core Concepts

## 2.1 Cluster

A cluster is a collection of Ollama nodes managed by a single Ollama Cluster controller.

## 2.2 Node

A node is an Ollama instance available to service inference requests.

Each node may have:

- a different hardware configuration;
- different installed models;
- different permitted models;
- different concurrency limits;
- different routing priority;
- different health and availability status.

## 2.3 Model

A model is an Ollama model that may be available on one or more nodes.

The system must distinguish between:

- **installed** — the model exists on the node;
- **permitted** — the cluster configuration allows the model to be used;
- **available** — the model is installed, permitted and the node is ready;
- **loaded** — the model is currently loaded into memory;
- **unavailable** — the model cannot currently receive requests on the node.

## 2.4 Controller

The controller maintains the authoritative cluster state and performs:

- routing;
- health management;
- node registration;
- model indexing;
- configuration management;
- recovery checks;
- operational command handling.

## 2.5 Node Agent

An optional node agent may run on each Ollama host to provide:

- Ollama process control;
- node hardware status;
- service restart operations;
- local model operations;
- local log access;
- event notifications to the controller.

---

# 3. Cluster Initialisation

## FR-001 — Initialise a cluster

The system must provide an initialisation command:

```bash
ocluster init
```

The initialisation process must:

- create the initial cluster configuration;
- configure the management endpoint;
- configure the inference proxy endpoint;
- optionally register one or more Ollama nodes;
- test connectivity to registered nodes;
- discover available models;
- create required service files;
- validate the completed configuration.

## FR-002 — Interactive and non-interactive initialisation

The initialisation command must support:

- an interactive setup wizard;
- command-line flags for scripted installation;
- configuration file input;
- unattended installation.

## FR-003 — Configuration validation

The system must validate:

- duplicate node names;
- invalid node URLs;
- unreachable nodes;
- invalid model configuration;
- unsupported routing policies;
- invalid timeout or retry settings;
- conflicting configuration values.

---

# 4. Node Registration and Management

## FR-010 — Add a node

The system must allow an administrator to register a new Ollama node.

```bash
ocluster node add gpu-01 --url http://gpu-01:11434
```

The system must:

- validate the node URL;
- verify that Ollama is reachable;
- retrieve the Ollama version;
- perform model discovery where enabled;
- add the node to the cluster registry;
- make the node eligible for routing once readiness checks pass.

## FR-011 — Remove a node

The system must allow a node to be removed from the cluster.

```bash
ocluster node remove gpu-01
```

The system must:

- prevent new requests from being sent to the node;
- warn if the node has active requests;
- support forced removal;
- remove the node from the model routing index;
- retain an audit event for the removal.

## FR-012 — Enable a node

The system must allow a disabled node to be enabled.

```bash
ocluster node enable gpu-01
```

Before returning the node to service, the system must:

- verify connectivity;
- perform readiness checks;
- synchronise model availability;
- confirm the node is eligible for routing.

## FR-013 — Disable a node

The system must allow a node to be immediately removed from routing.

```bash
ocluster node disable gpu-01
```

Disabling a node must:

- prevent new requests;
- optionally cancel active requests;
- preserve node registration;
- preserve model configuration.

## FR-014 — Drain a node

The system must support graceful node draining.

```bash
ocluster node drain gpu-01
```

Draining must:

- stop new requests from being assigned;
- allow active requests to complete;
- report the number of active requests;
- mark the node as drained once all requests have completed;
- support a configurable drain timeout;
- support forced termination after the timeout.

## FR-015 — Restart a node

Where a node agent or supported service control mechanism is available, the system must support:

```bash
ocluster node restart gpu-01
```

The restart process must:

- optionally drain the node first;
- stop or restart the Ollama service;
- mark the node unavailable;
- wait for the service to recover;
- perform readiness checks;
- synchronise model availability;
- return the node to routing.

## FR-016 — Start and stop Ollama

Where supported, the system must provide:

```bash
ocluster node start gpu-01
ocluster node stop gpu-01
```

The system must not expose unrestricted remote shell access.

## FR-017 — Inspect a node

The system must provide detailed node information.

```bash
ocluster node inspect gpu-01
```

The output should include:

- node name;
- address;
- Ollama version;
- node state;
- uptime;
- last successful connection;
- last failure;
- active requests;
- queued requests;
- installed models;
- loaded models;
- permitted models;
- configured concurrency;
- recent response latency;
- hardware and GPU information where available.

---

# 5. Node States

## FR-020 — Maintain node state

Each node must have one of the following operational states:

- `initialising`;
- `ready`;
- `suspect`;
- `draining`;
- `drained`;
- `disabled`;
- `unavailable`;
- `recovering`;
- `warming`;
- `error`.

## FR-021 — State transitions

The controller must manage valid transitions between node states.

Examples include:

```text
ready → suspect → unavailable → recovering → ready
ready → draining → drained
disabled → recovering → ready
```

## FR-022 — State persistence

Node administrative state must persist across controller restarts.

For example, a manually disabled node must not automatically return to routing after a controller restart.

---

# 6. Health Monitoring and Recovery

## FR-030 — Passive health monitoring

The controller must use normal inference requests to detect node failures.

Failure conditions may include:

- connection refused;
- connection timeout;
- network unreachable;
- invalid HTTP response;
- repeated server errors;
- unexpected stream termination.

## FR-031 — Failure thresholds

The system must support configurable failure thresholds based on failure type.

Examples:

- immediate ejection for connection refusal;
- ejection after repeated timeouts;
- ejection after a configurable number of HTTP 500 or 503 responses.

## FR-032 — Node ejection

When a node exceeds its failure threshold, the system must:

- mark the node unavailable;
- stop assigning new requests;
- remove the node from active model routing pools;
- schedule recovery checks;
- record an operational event.

## FR-033 — Recovery checks

Unavailable nodes must be checked periodically using:

- exponential backoff;
- a configurable maximum interval;
- random jitter;
- configurable success thresholds.

## FR-034 — Recovery confirmation

A node must pass a configurable number of successful health checks before it is returned to routing.

## FR-035 — Idle node checks

The system should support infrequent health checks for idle healthy nodes to identify silent failures.

## FR-036 — Manual health probe

The CLI must allow an administrator to trigger an immediate probe.

```bash
ocluster node probe gpu-01
```

---

# 7. Model Discovery and Registry

## FR-040 — Automatic model discovery

The system must discover installed models from an Ollama node using the Ollama API.

Discovery must occur:

- during cluster initialisation;
- when a node is added;
- when the controller starts;
- when a node reconnects;
- after an Ollama restart;
- after a model operation performed through Ollama Cluster;
- when manually requested.

## FR-041 — Model discovery modes

Each node must support the following model configuration modes.

### Discover mode

All discovered models are eligible for routing.

```toml
model_mode = "discover"
```

### Allow mode

Only models that are both discovered and explicitly permitted are eligible.

```toml
model_mode = "allow"
models = ["llama3.3:70b", "qwen3:32b"]
```

### Static mode

The configured model list is authoritative.

```toml
model_mode = "static"
models = ["llama3.3:70b"]
```

## FR-042 — Discovery as the default

New nodes must use automatic discovery by default unless otherwise configured.

## FR-043 — Maintain discovered and effective models

For each node, the system must separately maintain:

- discovered models;
- configured models;
- permitted models;
- effective routable models;
- currently loaded models.

## FR-044 — Cluster model index

The controller must maintain a model-to-node index.

Example:

```text
llama3.3:70b
  gpu-01 — ready, loaded
  gpu-02 — ready, cold
  gpu-03 — unavailable
```

## FR-045 — Manual model synchronisation

The CLI must support synchronising all discoverable nodes.

```bash
ocluster models sync
```

It must also support synchronising a single node.

```bash
ocluster node models sync gpu-01
```

## FR-046 — Model synchronisation preview

The system should support a dry-run mode.

```bash
ocluster node models sync gpu-01 --dry-run
```

The output should identify:

- added models;
- removed models;
- changed model digests;
- configuration drift.

## FR-047 — Background discovery

The system must support configurable low-frequency background discovery.

The background discovery interval must be configurable globally and per node.

## FR-048 — Model change detection

The system must calculate a model inventory fingerprint based on attributes such as:

- model name;
- digest;
- modification time;
- size.

The model index should only be updated when the fingerprint changes.

## FR-049 — Optional node notification

Where a node agent is installed, the node should be able to notify the controller that its model inventory may have changed.

The controller must then perform authoritative discovery through the Ollama API.

---

# 8. Model Management

## FR-050 — List models

The system must provide:

```bash
ocluster models
```

The output should include:

- model name;
- number of nodes;
- number of ready nodes;
- number of loaded instances;
- active requests;
- queued requests.

## FR-051 — Inspect a model

The system must provide:

```bash
ocluster model inspect llama3.3:70b
```

The output should include:

- all nodes containing the model;
- node readiness;
- loaded state;
- model digest;
- model size;
- model family;
- quantisation;
- active requests by node.

## FR-052 — Pull a model

Where supported, the system must allow a model to be pulled to one or more nodes.

```bash
ocluster model pull llama3.3:70b --node gpu-01
ocluster model pull llama3.3:70b --nodes gpu-01,gpu-02
ocluster model pull llama3.3:70b --all
```

The system must:

- report pull progress;
- verify successful completion;
- refresh the node model registry;
- update the cluster routing index.

## FR-053 — Remove a model

The system must allow a model to be removed from selected nodes.

The operation must:

- warn if the model is active;
- prevent new requests for the model on affected nodes;
- optionally drain active requests;
- refresh the model registry after deletion.

## FR-054 — Model aliases

The system should support optional aliases that map client-facing model names to Ollama model names.

Example:

```toml
[model_aliases]
"production-chat" = "llama3.3:70b"
"fast-chat" = "qwen3:8b"
```

## FR-055 — Model restrictions

The system must support denying specific models globally or per node.

---

# 9. Request Routing

## FR-060 — Model-aware routing

The controller must only route a request to a node that:

- is in a routable state;
- has the requested model available;
- permits the requested model;
- is below applicable concurrency limits.

## FR-061 — Routing policies

The system must support configurable routing policies, including:

- round robin;
- least active requests;
- least queued requests;
- weighted routing;
- priority routing;
- loaded-model preference;
- estimated least workload.

## FR-062 — Loaded-model preference

The system should prefer a node where the requested model is already loaded, unless another node is significantly less busy.

## FR-063 — Routing score

The system should support a configurable routing score based on:

- active requests;
- queued requests;
- node priority;
- model loaded state;
- recent failures;
- response latency;
- node capacity;
- estimated request size.

## FR-064 — Per-node limits

Each node must support:

- maximum concurrent requests;
- maximum queued requests;
- optional per-model limits;
- optional request admission limits.

## FR-065 — No available node behaviour

Where no eligible node is available, the system must either:

- place the request into a bounded queue; or
- return a clear service-unavailable response.

This behaviour must be configurable.

## FR-066 — Queue limits

The system must enforce:

- maximum queue depth;
- maximum queue wait time;
- optional per-model queue limits.

## FR-067 — Routing transparency

The system must provide a command to explain model routing eligibility.

```bash
ocluster explain llama3.3:70b
```

The output must identify:

- eligible nodes;
- rejected nodes;
- rejection reasons;
- routing scores;
- the preferred node.

---

# 10. Proxy and Streaming Behaviour

## FR-070 — Ollama API compatibility

The proxy must preserve compatibility with supported Ollama API endpoints used for inference.

## FR-071 — Streaming responses

The system must stream responses from Ollama to the client without buffering the complete response.

## FR-072 — Backpressure

The proxy must respect backpressure between:

- the Ollama node;
- the controller;
- the client.

## FR-073 — Client cancellation

When a client disconnects or cancels a request, the system must attempt to cancel the corresponding upstream Ollama request.

## FR-074 — Retry before response

If a request fails before response streaming begins, the controller may retry the request on another eligible node.

## FR-075 — No transparent retry after streaming

Once response data has been sent to the client, the system must not transparently retry the request on another node.

## FR-076 — Retry limits

The system must provide configurable limits for:

- maximum retry attempts;
- retryable error types;
- retry timeout;
- alternate-node selection.

## FR-077 — Connection reuse

The proxy must reuse upstream HTTP connections where possible.

---

# 11. Request Monitoring

## FR-080 — List active requests

The system must provide:

```bash
ocluster requests
```

The output should include:

- request identifier;
- model;
- assigned node;
- start time;
- duration;
- request state;
- streaming state.

## FR-081 — Watch requests

The CLI must support live request monitoring.

```bash
ocluster requests watch
```

## FR-082 — Cancel a request

An administrator must be able to cancel an active request.

```bash
ocluster request cancel <request-id>
```

## FR-083 — Request history

The system should maintain configurable short-term request history containing:

- model;
- node;
- response status;
- duration;
- time to first token;
- prompt token count where available;
- output token count where available;
- failure reason.

Request content must not be stored by default.

---

# 12. Cluster Status and Visibility

## FR-090 — Cluster status

The system must provide:

```bash
ocluster status
```

The output must summarise:

- overall cluster state;
- controller uptime;
- total nodes;
- ready nodes;
- unavailable nodes;
- draining nodes;
- total models;
- active requests;
- queued requests;
- recent failures.

## FR-091 — List nodes

The system must provide:

```bash
ocluster nodes
```

The output should include:

- node name;
- state;
- address;
- active requests;
- model count;
- loaded models;
- recent latency;
- last contact.

## FR-092 — Health summary

The system must provide:

```bash
ocluster health
```

The output must highlight:

- unavailable nodes;
- suspect nodes;
- failed health checks;
- recovery attempts;
- configuration drift;
- models with no available nodes.

## FR-093 — Events

The system must provide:

```bash
ocluster events
```

Events should include:

- node added or removed;
- node failure;
- node recovery;
- model discovered;
- model removed;
- configuration changed;
- drain started or completed;
- service restarted;
- routing failure.

---

# 13. Interactive Terminal Interface

## FR-100 — Launch interactive interface

Running the command without arguments should launch an interactive terminal interface where supported.

```bash
ocluster
```

The system may also support:

```bash
ocluster dashboard
```

## FR-101 — Dashboard views

The terminal interface should provide views for:

- cluster overview;
- nodes;
- models;
- active requests;
- metrics;
- logs;
- events;
- configuration.

## FR-102 — Node actions

From the terminal interface, an administrator should be able to:

- inspect a node;
- enable a node;
- disable a node;
- drain a node;
- restart a node;
- trigger a model sync;
- view logs;
- trigger a health probe.

## FR-103 — Model actions

From the terminal interface, an administrator should be able to:

- inspect a model;
- identify eligible nodes;
- pull a model;
- remove a model;
- synchronise model state.

## FR-104 — Confirmation prompts

Destructive operations must require confirmation unless a force or non-interactive flag is supplied.

---

# 14. Configuration Management

## FR-110 — Show configuration

The system must provide:

```bash
ocluster config show
```

## FR-111 — Edit configuration

The system should provide:

```bash
ocluster config edit
```

## FR-112 — Validate configuration

The system must provide:

```bash
ocluster config validate
```

## FR-113 — Apply configuration

The system must allow validated configuration changes to be applied without restarting the controller where practical.

## FR-114 — Configuration precedence

The system must define precedence between:

- built-in defaults;
- configuration file;
- environment variables;
- command-line options;
- runtime controller state.

## FR-115 — Configuration reload

The system should support:

```bash
ocluster config reload
```

## FR-116 — Configuration rollback

The system should retain the previous valid configuration and allow rollback after an invalid or unsuccessful update.

---

# 15. Logging and Metrics

## FR-120 — Controller logs

The system must produce structured logs for:

- routing decisions;
- health state changes;
- node operations;
- model discovery;
- retries;
- request failures;
- configuration changes.

## FR-121 — View logs

The CLI must support:

```bash
ocluster logs
ocluster logs --follow
```

## FR-122 — Node logs

Where a node agent is installed, the CLI should support:

```bash
ocluster logs gpu-01
ocluster logs gpu-01 --follow
```

## FR-123 — Metrics

The system must expose operational metrics including:

- request count;
- active requests;
- queue depth;
- request duration;
- time to first token;
- node failures;
- retry count;
- node availability;
- model availability;
- token throughput where available.

## FR-124 — Metrics endpoint

The controller should expose metrics in a format compatible with Prometheus.

---

# 16. Service Management

## FR-130 — Run as a service

The controller must be capable of running as a system service named:

```text
ocluster.service
```

## FR-131 — Service commands

The application must support appropriate behaviour when controlled through:

```bash
systemctl start ocluster
systemctl stop ocluster
systemctl restart ocluster
systemctl status ocluster
```

## FR-132 — Graceful shutdown

When shutting down, the controller must:

- stop accepting new requests;
- optionally drain active requests;
- persist cluster state;
- close network connections cleanly.

---

# 17. Security and Access Control

## FR-140 — Separate management and inference interfaces

The system must support separate endpoints for:

- inference traffic;
- management traffic.

## FR-141 — Local management socket

The system should support a Unix domain socket for local management.

Example:

```text
/run/ocluster/ocluster.sock
```

## FR-142 — Remote management security

Remote management must require authenticated and encrypted communication.

## FR-143 — Administrative permissions

The system should support permissions for:

- read-only status access;
- node management;
- model management;
- configuration management;
- cluster administration.

## FR-144 — Audit trail

Administrative actions must be recorded with:

- timestamp;
- action;
- target;
- outcome;
- authenticated identity where available.

---

# 18. High Availability and Persistence

## FR-150 — Persistent cluster state

The controller must persist:

- registered nodes;
- administrative node states;
- node configuration;
- model configuration;
- routing configuration;
- recent operational events.

## FR-151 — Controller restart recovery

After a restart, the controller must:

- restore persisted state;
- reconnect to registered nodes;
- refresh node health;
- synchronise discovered models;
- rebuild the model routing index.

## FR-152 — Controller high availability

A future version should support multiple controller instances with coordinated cluster state and leader election.

This is not required for the initial release unless specifically included in scope.

---

# 19. CLI Output and Automation

## FR-160 — Human-readable output

CLI commands must provide clear terminal-friendly output by default.

## FR-161 — Machine-readable output

Read-only commands must support machine-readable formats.

```bash
ocluster status --output json
ocluster nodes --output json
ocluster models --output yaml
```

## FR-162 — Exit codes

Commands must return consistent exit codes for:

- success;
- validation failure;
- connection failure;
- partial success;
- unavailable resource;
- unauthorised operation.

## FR-163 — Non-interactive operation

All management actions must support non-interactive execution for use in scripts and automation.

---

# 20. Initial Release Scope

The recommended minimum viable release should include:

1. Cluster initialisation.
2. Static node registration.
3. Automatic model discovery.
4. Discover, allow and static model modes.
5. Model-to-node registry.
6. Model-aware routing.
7. Least-active-request routing.
8. Loaded-model preference.
9. Passive failure detection.
10. Automatic node ejection.
11. Recovery health checks.
12. Request retry before streaming begins.
13. Streaming proxy support.
14. Node enable, disable and drain operations.
15. Model synchronisation commands.
16. Cluster, node and model status commands.
17. Structured logging.
18. Prometheus-compatible metrics.
19. TOML configuration.
20. JSON output for automation.

The following features could be deferred:

- remote node agent;
- Ollama process start and stop;
- interactive terminal dashboard;
- model pulling and deletion;
- user management;
- controller high availability;
- advanced token-aware scheduling;
- GPU telemetry;
- web-based administration.
