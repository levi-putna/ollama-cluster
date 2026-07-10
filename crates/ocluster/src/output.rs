use clap::ValueEnum;
use ocluster_protocol::{
    ClusterStatusResponse, EventResponse, ExplainResponse, ModelDetailResponse, ModelSummary,
    NodeDetailResponse, NodeSummary, OperationResponse, RequestSummary,
};

/// Output format for read commands.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Yaml,
}

/// Writes command output in the selected format.
pub struct OutputWriter {
    format: OutputFormat,
}

impl OutputWriter {
    /// Create a new output writer.
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Write arbitrary JSON-serialisable value.
    pub fn write_json<T: serde::Serialize + ?Sized>(&self, value: &T) {
        match self.format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value).unwrap()),
            OutputFormat::Yaml => println!("{}", serde_yaml::to_string(value).unwrap()),
            OutputFormat::Table => println!("{}", serde_json::to_string_pretty(value).unwrap()),
        }
    }

    /// Write cluster status.
    pub fn write_cluster_status(&self, status: &ClusterStatusResponse) {
        match self.format {
            OutputFormat::Table => {
                println!("Cluster state: {}", status.state);
                println!("Uptime: {}s", status.uptime_seconds);
                println!(
                    "Nodes: {} total, {} ready, {} unavailable, {} draining",
                    status.nodes_total,
                    status.nodes_ready,
                    status.nodes_unavailable,
                    status.nodes_draining
                );
                println!("Models: {}", status.models_total);
                println!(
                    "Requests: {} active, {} queued",
                    status.active_requests, status.queued_requests
                );
            }
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(status),
        }
    }

    /// Write node list.
    pub fn write_nodes(&self, nodes: &[NodeSummary]) {
        match self.format {
            OutputFormat::Table => {
                for n in nodes {
                    println!(
                        "{:<12} {:<10} {:<12} {} active={} models={}",
                        n.name,
                        format!("{:?}", n.admin_state),
                        format!("{:?}", n.runtime_state),
                        n.url,
                        n.active_requests,
                        n.model_count
                    );
                }
            }
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(nodes),
        }
    }

    /// Write health summary from nodes.
    pub fn write_health(&self, nodes: &[NodeSummary]) {
        match self.format {
            OutputFormat::Table => {
                for n in nodes {
                    if format!("{:?}", n.runtime_state).contains("Unavailable") {
                        println!("UNAVAILABLE: {}", n.name);
                    }
                }
                if nodes.is_empty() {
                    println!("No nodes registered");
                }
            }
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(nodes),
        }
    }

    /// Write events list.
    pub fn write_events(&self, events: &[EventResponse]) {
        match self.format {
            OutputFormat::Table => {
                for e in events {
                    println!(
                        "{} [{}] {} — {}",
                        e.created_at,
                        e.event_type,
                        e.target.as_deref().unwrap_or("-"),
                        e.message
                    );
                }
            }
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(events),
        }
    }

    /// Write models list.
    pub fn write_models(&self, models: &[ModelSummary]) {
        match self.format {
            OutputFormat::Table => {
                for m in models {
                    println!(
                        "{:<30} nodes={} ready={} loaded={}",
                        m.name, m.node_count, m.ready_nodes, m.loaded_instances
                    );
                }
            }
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(models),
        }
    }

    /// Write node detail.
    pub fn write_node_detail(&self, node: &NodeDetailResponse) {
        self.write_json(node);
    }

    /// Write model detail.
    pub fn write_model_detail(&self, model: &ModelDetailResponse) {
        self.write_json(model);
    }

    /// Write routing explanation.
    pub fn write_explain(&self, explain: &ExplainResponse) {
        self.write_json(explain);
    }

    /// Write active requests.
    pub fn write_requests(&self, requests: &[RequestSummary]) {
        match self.format {
            OutputFormat::Table => {
                for r in requests {
                    println!(
                        "{} model={} node={} {}ms streaming={}",
                        r.id, r.model, r.node, r.duration_ms, r.streaming
                    );
                }
            }
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(requests),
        }
    }

    /// Write operation result.
    pub fn write_operation(&self, op: &OperationResponse) {
        match self.format {
            OutputFormat::Table => println!("{}", op.message),
            OutputFormat::Json | OutputFormat::Yaml => self.write_json(op),
        }
    }
}
