use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ocluster_core::{AdminState, NodeRuntimeState, RoutingExplanation};
use serde::{Deserialize, Serialize};

use crate::error::ApiErrorBody;

pub const API_VERSION: &str = "v1";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cluster status summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusResponse {
    pub version: String,
    pub api_version: String,
    pub state: String,
    pub uptime_seconds: u64,
    pub nodes_total: usize,
    pub nodes_ready: usize,
    pub nodes_unavailable: usize,
    pub nodes_draining: usize,
    pub models_total: usize,
    pub active_requests: usize,
    pub queued_requests: usize,
}

/// Version and capability information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    pub app_version: String,
    pub api_version: String,
    pub features: Vec<String>,
}

/// Node summary for list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub name: String,
    pub url: String,
    pub admin_state: AdminState,
    pub runtime_state: NodeRuntimeState,
    pub ollama_version: Option<String>,
    pub active_requests: u32,
    pub model_count: usize,
    pub loaded_models: usize,
    pub last_contact: Option<DateTime<Utc>>,
}

/// Detailed node inspection response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDetailResponse {
    #[serde(flatten)]
    pub summary: NodeSummary,
    pub labels: HashMap<String, String>,
    pub max_concurrent: u32,
    pub models: Vec<String>,
}

/// Request to register a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddNodeRequest {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub model_mode: Option<String>,
    #[serde(default)]
    pub max_concurrent: Option<u32>,
}

/// Request to update an existing node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeRequest {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub model_mode: Option<String>,
    #[serde(default)]
    pub max_concurrent: Option<u32>,
}

/// Model summary across the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSummary {
    pub name: String,
    pub node_count: usize,
    pub ready_nodes: usize,
    pub loaded_instances: usize,
    pub active_requests: u32,
}

/// Model inspection response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetailResponse {
    pub name: String,
    pub nodes: Vec<ModelNodeDetail>,
}

/// Per-node model detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelNodeDetail {
    pub node: String,
    pub available: bool,
    pub loaded: bool,
    pub digest: Option<String>,
    pub size: Option<u64>,
}

/// Active request summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSummary {
    pub id: String,
    pub model: String,
    pub node: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub streaming: bool,
    pub state: String,
}

/// Cluster event response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResponse {
    pub id: String,
    pub event_type: String,
    pub target: Option<String>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// Explain routing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainResponse {
    #[serde(flatten)]
    pub explanation: RoutingExplanation,
}

/// Generic operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResponse {
    pub success: bool,
    pub message: String,
}

/// Config show response with redacted secrets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub config: serde_json::Value,
}

/// Health summary response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummaryResponse {
    pub unavailable_nodes: Vec<String>,
    pub suspect_nodes: Vec<String>,
    pub models_without_nodes: Vec<String>,
    pub drift_detected: bool,
}

/// Re-export error type for handlers.
pub type ApiError = ApiErrorBody;
