# About Ollama Cluster

**Ollama Cluster** (`ocluster`) is a lightweight management and routing layer for multiple [Ollama](https://ollama.com) instances. It does not replace Ollama. It sits in front of your nodes, exposes a single Ollama-compatible API, and decides which node should handle each request.

If you run Ollama on more than one machine, or want to, this project gives you one entry point, automatic failover, and the tools to monitor and operate the cluster.

## The problem

Traditional web servers are measured in milliseconds. AI inference is measured in **seconds**.

A single Ollama request can take anywhere from a few seconds to over a minute depending on the model, prompt size, and hardware. That is fine for a quick chat, but it becomes a serious bottleneck the moment you use Ollama in any meaningful way: multiple apps, agents, batch jobs, or a household sharing the same instance. Requests do not fail gracefully; they **stack up in a queue**, waiting their turn while the GPU sits saturated on one machine.

Scaling up by buying ever more expensive hardware is one answer, but it is not always the best one. A bigger GPU helps individual request latency, yet you still have a **single queue on a single node**. Throughput is capped by however many concurrent requests that one machine can handle.

The alternative is **horizontal capacity**: add more nodes, spread the load, and let the cluster route each request to whichever machine is ready. That is what Ollama Cluster is for.

Without a cluster layer you also end up with:

- clients hard-coded to individual machines
- no automatic failover when a node goes down
- uneven load when one device is busy and others sit idle
- manual tracking of which models exist on which nodes

Ollama Cluster treats your Ollama servers as a pool and routes requests intelligently across them, so throughput grows with the number of nodes you add, not just the price of your GPU.

## Why I built this

I built Ollama Cluster to make use of the **smaller devices I already had**: old laptops, mini PCs, spare workstations, each running Ollama on modest hardware.

Individually, none of them could compete with a top-tier GPU. Together, they give me **more parallel throughput** than I was getting from chasing a single expensive upgrade, and often a better practical result, because the bottleneck was never raw speed on one request; it was **how many requests could run at once**.

Rather than buy ever more expensive hardware for diminishing returns, I wired up what I had, pointed them all at one cluster, and let the routing layer spread work across them. Ollama Cluster is the tool I wanted for that: simple, local, and built around the reality that AI workloads queue in seconds, not milliseconds.

## How it works

```text
Clients (curl, Open WebUI, agents, etc.)
              ↓
     ocluster inference proxy (:11434)
              ↓
        Routing engine
    ┌─────────┼─────────┐
    ↓         ↓         ↓
 Ollama    Ollama    Ollama
 node A    node B    node C
```

Each node is a standard Ollama installation, the same software you download from [ollama.com](https://ollama.com). The cluster discovers models on each node, tracks health, and forwards inference requests using the familiar Ollama HTTP API (`/api/generate`, `/api/chat`, `/api/tags`, and so on).

Clients do not need to know how many nodes exist. They talk to the cluster as if it were a single Ollama server.

## What you get

| Capability | Description |
| ---------- | ----------- |
| **Unified endpoint** | One Ollama-compatible address for all clients |
| **Model-aware routing** | Sends requests to nodes that have the model, preferring loaded copies and least-busy nodes |
| **Failover** | Retries on alternate nodes when a backend fails before streaming starts |
| **Health monitoring** | Passive failure detection, circuit breakers, and node probes |
| **Management API** | REST API for nodes, models, requests, events, and configuration |
| **CLI** | `ocluster` command for init, serve, status, and node operations |
| **Terminal dashboard** | Live monitoring and node actions from the terminal |
| **Web admin panel** | Browser dashboard with polling, node CRUD, and model management |
| **Metrics** | Prometheus endpoint for observability |
| **Persistence** | SQLite storage so cluster state survives controller restarts |

## Who it is for

- **People with spare hardware** who want to turn a collection of smaller devices into meaningful AI throughput
- **Homelab and solo developers** running Ollama on a few machines who want a single API endpoint
- **Small teams** sharing GPU capacity across workstations or servers
- **Operators** who need visibility into node health, active requests, and model placement
- **Anyone building apps against Ollama** who has outgrown a single-node queue and wants routing without changing client code

Ollama Cluster targets **self-hosted Ollama instances**. It is not a replacement for [Ollama Cloud](https://ollama.com). It clusters the local `ollama serve` processes you control.

## Project status

Version **0.1.0** is an MVP focused on static node registration, discovery, routing, proxying, and operations. Some features described in the requirements documents (remote node agent, TLS, authentication, model pull/delete via cluster, controller HA) are planned or stubbed for future releases.

See [README.md](README.md) for install instructions and [docs/traceability.md](docs/traceability.md) for requirement coverage.

## Get started

```bash
# Install pre-built binary (see README for your platform)
curl -LO https://github.com/levi-putna/ollama-cluster/releases/latest/download/ocluster-aarch64-apple-darwin.tar.gz
tar xzf ocluster-aarch64-apple-darwin.tar.gz
sudo mv ocluster /usr/local/bin/

# Initialise and run
ocluster init --node gpu-01@http://127.0.0.1:11435
ocluster serve
```

Or install from source with Cargo:

```bash
cargo install --git https://github.com/levi-putna/ollama-cluster --bin ocluster
```

Full instructions: [README.md](README.md)

## Technology

Built in **Rust** as a Cargo workspace. The controller runs three listeners by default:

| Service | Port |
| ------- | ---- |
| Inference proxy | 11434 |
| Management API | 11600 |
| Prometheus metrics | 11601 |
| Web admin panel | 11602 |

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and the Developer Certificate of Origin (DCO) sign-off process.

## License

Ollama Cluster is **source-available** under a fair-source, two-tier licence:

- **Free** for individuals, non-profits, and for-profit organisations with annual gross revenue of **AUD $100,000 or less** (commercial use included)
- **Commercial licence** required above that threshold. See [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md)

Full terms: [LICENSE.md](LICENSE.md)

## Links

- **Repository:** [github.com/levi-putna/ollama-cluster](https://github.com/levi-putna/ollama-cluster)
- **Releases:** [GitHub Releases](https://github.com/levi-putna/ollama-cluster/releases)
- **Ollama:** [ollama.com](https://ollama.com)
