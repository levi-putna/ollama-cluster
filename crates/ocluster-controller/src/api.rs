use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use ocluster_core::{explain_routing, AdminState, ModelMode, NodeRuntimeState};
use ocluster_config::validate::is_url_allowed;
use ocluster_protocol::{
    AddNodeRequest, ClusterStatusResponse, ConfigResponse, EventResponse, ExplainResponse,
    HealthSummaryResponse, ModelDetailResponse, ModelNodeDetail, ModelSummary, NodeDetailResponse,
    NodeSummary, OperationResponse, RequestSummary, UpdateNodeRequest, VersionResponse,
    API_VERSION, APP_VERSION,
};
use ocluster_protocol::ApiErrorBody;
use ocluster_storage::NodeRecord;
use serde_json::json;
use uuid::Uuid;

use crate::discovery::{apply_discovery_to_runtime, discover_node, sync_all_nodes};
use crate::health::record_success;
use crate::runtime::{ClusterRuntime, SharedRuntime};

/// Application state for management API handlers.
#[derive(Clone)]
pub struct AppState {
    pub runtime: SharedRuntime,
    pub http_client: reqwest::Client,
}

/// Build the management API router.
pub fn management_router(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/api/v1/version", get(version))
        .route("/api/v1/cluster", get(cluster_status))
        .route("/api/v1/nodes", get(list_nodes).post(add_node))
        .route(
            "/api/v1/nodes/:name",
            get(get_node).delete(remove_node).patch(update_node),
        )
        .route("/api/v1/nodes/:name/enable", post(enable_node))
        .route("/api/v1/nodes/:name/disable", post(disable_node))
        .route("/api/v1/nodes/:name/drain", post(drain_node))
        .route("/api/v1/nodes/:name/probe", post(probe_node))
        .route("/api/v1/models", get(list_models))
        .route("/api/v1/models/:name", get(get_model))
        .route("/api/v1/models/sync", post(sync_models))
        .route("/api/v1/models/:name/explain", get(explain_model))
        .route("/api/v1/requests", get(list_requests))
        .route("/api/v1/requests/:id", delete(cancel_request))
        .route("/api/v1/events", get(list_events))
        .route("/api/v1/config", get(show_config))
        .route("/api/v1/config/validate", post(validate_config_handler))
        .route("/api/v1/config/reload", post(reload_config))
        .with_state(state)
}

async fn liveness() -> impl IntoResponse {
    Json(json!({ "status": "live" }))
}

async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let ready = !rt.shutting_down;
    let status = if ready { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(json!({ "status": if ready { "ready" } else { "not_ready" } })))
}

async fn version() -> impl IntoResponse {
    Json(VersionResponse {
        app_version: APP_VERSION.to_string(),
        api_version: API_VERSION.to_string(),
        features: vec![
            "routing".into(),
            "discovery".into(),
            "streaming_proxy".into(),
            "metrics".into(),
        ],
    })
}

async fn cluster_status(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let nodes_ready = rt.nodes.values().filter(|n| n.is_routable()).count();
    let nodes_unavailable = rt
        .nodes
        .values()
        .filter(|n| n.record.runtime_state == NodeRuntimeState::Unavailable)
        .count();
    let nodes_draining = rt
        .nodes
        .values()
        .filter(|n| n.record.admin_state == AdminState::Draining)
        .count();
    let models = rt.storage.list_models(None).unwrap_or_default();
    let unique_models: std::collections::HashSet<_> =
        models.iter().map(|m| m.model_name.clone()).collect();

    Json(ClusterStatusResponse {
        version: APP_VERSION.to_string(),
        api_version: API_VERSION.to_string(),
        state: if rt.shutting_down {
            "shutting_down".into()
        } else {
            "running".into()
        },
        uptime_seconds: rt.started_at.elapsed().as_secs(),
        nodes_total: rt.nodes.len(),
        nodes_ready,
        nodes_unavailable,
        nodes_draining,
        models_total: unique_models.len(),
        active_requests: rt.requests.len(),
        queued_requests: rt.queued_requests.load(std::sync::atomic::Ordering::Relaxed),
    })
}

async fn list_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let summaries: Vec<NodeSummary> = rt
        .nodes
        .values()
        .map(|n| NodeSummary {
            name: n.record.name.clone(),
            url: n.record.url.clone(),
            admin_state: n.record.admin_state,
            runtime_state: n.record.runtime_state,
            ollama_version: n.record.ollama_version.clone(),
            active_requests: n.active_requests,
            model_count: n.discovered_models.len(),
            loaded_models: n.loaded_models.len(),
            last_contact: n.last_contact,
        })
        .collect();
    Json(summaries)
}

async fn get_node(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let Some(node) = rt.nodes.get(&name) else {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    };
    Json(node_detail(node)).into_response()
}

async fn add_node(
    State(state): State<AppState>,
    Json(req): Json<AddNodeRequest>,
) -> impl IntoResponse {
    if is_url_allowed(&req.url, false).is_err() {
        return api_error(StatusCode::BAD_REQUEST, "INVALID_URL", "Invalid node URL");
    }

    let mut rt = state.runtime.write().await;
    if rt.nodes.contains_key(&req.name) {
        return api_error(
            StatusCode::CONFLICT,
            "NODE_EXISTS",
            format!("Node '{}' already exists", req.name),
        );
    }

    let record = NodeRecord {
        id: Uuid::new_v4().to_string(),
        name: req.name.clone(),
        url: req.url.clone(),
        admin_state: AdminState::Enabled,
        runtime_state: NodeRuntimeState::Warming,
        ollama_version: None,
        model_mode: match req.model_mode.as_deref() {
            Some("allow") => ModelMode::Allow,
            Some("static") => ModelMode::Static,
            _ => ModelMode::Discover,
        },
        configured_models: vec![],
        max_concurrent: req.max_concurrent.unwrap_or(8),
        priority: 0,
        labels: Default::default(),
        inventory_fingerprint: None,
    };

    if let Err(e) = rt.upsert_runtime_node(record) {
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR", e.to_string());
    }

    let url = req.url.clone();
    let timeout = rt.config.discovery.timeout_ms;
    drop(rt);

    match discover_node(&state.http_client, &url, timeout).await {
        Ok(result) => {
            let mut rt = state.runtime.write().await;
            let _ = apply_discovery_to_runtime(&mut rt, &req.name, &result);
            let _ = rt.storage.append_event("node_added", Some(&req.name), "node registered");
            let _ = rt.storage.append_audit("node_add", Some(&req.name), "success", None);
        }
        Err(e) => {
            let rt = state.runtime.write().await;
            let _ = rt.storage.append_event(
                "node_add_warning",
                Some(&req.name),
                &format!("registered but discovery failed: {e}"),
            );
        }
    }

    Json(OperationResponse {
        success: true,
        message: format!("Node '{}' added", req.name),
    })
    .into_response()
}

async fn update_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<UpdateNodeRequest>,
) -> impl IntoResponse {
    if let Some(ref url) = req.url {
        if is_url_allowed(url, false).is_err() {
            return api_error(StatusCode::BAD_REQUEST, "INVALID_URL", "Invalid node URL");
        }
    }

    let mut rt = state.runtime.write().await;
    let Some(node) = rt.nodes.get_mut(&name) else {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    };

    let url_changed = if let Some(ref url) = req.url {
        node.record.url = url.clone();
        true
    } else {
        false
    };

    if let Some(ref mode) = req.model_mode {
        node.record.model_mode = match mode.as_str() {
            "allow" => ModelMode::Allow,
            "static" => ModelMode::Static,
            _ => ModelMode::Discover,
        };
    }

    if let Some(max) = req.max_concurrent {
        node.record.max_concurrent = max;
    }

    if url_changed {
        node.record.runtime_state = NodeRuntimeState::Warming;
    }

    let record = node.record.clone();
    let url = record.url.clone();
    let timeout = rt.config.discovery.timeout_ms;
    if let Err(e) = rt.storage.upsert_node(&record) {
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR", e.to_string());
    }
    drop(rt);

    if url_changed {
        match discover_node(&state.http_client, &url, timeout).await {
            Ok(result) => {
                let mut rt = state.runtime.write().await;
                let _ = apply_discovery_to_runtime(&mut rt, &name, &result);
            }
            Err(e) => {
                let rt = state.runtime.write().await;
                let _ = rt.storage.append_event(
                    "node_update_warning",
                    Some(&name),
                    &format!("updated but discovery failed: {e}"),
                );
            }
        }
    }

    let rt = state.runtime.read().await;
    let _ = rt.storage.append_event("node_updated", Some(&name), "node configuration updated");
    drop(rt);

    Json(OperationResponse {
        success: true,
        message: format!("Node '{name}' updated"),
    })
    .into_response()
}

async fn remove_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut rt = state.runtime.write().await;
    if !rt.nodes.contains_key(&name) {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    }
    if rt.requests.values().any(|r| r.node_name == name) {
        return api_error(
            StatusCode::CONFLICT,
            "NODE_BUSY",
            "Node has active requests; use force removal",
        );
    }
    rt.nodes.remove(&name);
    let _ = rt.storage.remove_node(&name);
    let _ = rt.storage.append_event("node_removed", Some(&name), "node removed");
    let _ = rt.storage.append_audit("node_remove", Some(&name), "success", None);
    Json(OperationResponse {
        success: true,
        message: format!("Node '{name}' removed"),
    })
    .into_response()
}

async fn enable_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut rt = state.runtime.write().await;
    let Some(node) = rt.nodes.get_mut(&name) else {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    };
    node.record.admin_state = AdminState::Enabled;
    let record = node.record.clone();
    let url = record.url.clone();
    let timeout = rt.config.discovery.timeout_ms;
    let _ = rt.storage.upsert_node(&record);
    drop(rt);

    if let Ok(result) = discover_node(&state.http_client, &url, timeout).await {
        let mut rt = state.runtime.write().await;
        let _ = apply_discovery_to_runtime(&mut rt, &name, &result);
        record_success(&mut rt, &name);
    }

    Json(OperationResponse {
        success: true,
        message: format!("Node '{name}' enabled"),
    })
    .into_response()
}

async fn disable_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut rt = state.runtime.write().await;
    let Some(node) = rt.nodes.get_mut(&name) else {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    };
    node.record.admin_state = AdminState::Disabled;
    let record = node.record.clone();
    let _ = rt.storage.upsert_node(&record);
    let _ = rt.storage.append_event("node_disabled", Some(&name), "node disabled");
    Json(OperationResponse {
        success: true,
        message: format!("Node '{name}' disabled"),
    })
    .into_response()
}

async fn drain_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut rt = state.runtime.write().await;
    let Some(node) = rt.nodes.get_mut(&name) else {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    };
    node.record.admin_state = AdminState::Draining;
    let record = node.record.clone();
    let _ = rt.storage.upsert_node(&record);
    let _ = rt.storage.append_event("drain_started", Some(&name), "node draining");
    Json(OperationResponse {
        success: true,
        message: format!("Node '{name}' draining"),
    })
    .into_response()
}

async fn probe_node(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let Some(node) = rt.nodes.get(&name) else {
        return api_error(StatusCode::NOT_FOUND, "NODE_NOT_FOUND", format!("Node '{name}' not found"));
    };
    let url = node.record.url.clone();
    let timeout = rt.config.discovery.timeout_ms;
    drop(rt);

    match discover_node(&state.http_client, &url, timeout).await {
        Ok(result) => {
            let mut rt = state.runtime.write().await;
            let _ = apply_discovery_to_runtime(&mut rt, &name, &result);
            record_success(&mut rt, &name);
            Json(json!({ "status": "healthy", "version": result.ollama_version }))
                .into_response()
        }
        Err(e) => api_error(StatusCode::SERVICE_UNAVAILABLE, "PROBE_FAILED", e),
    }
}

async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let models = rt.storage.list_models(None).unwrap_or_default();
    let mut map: std::collections::HashMap<String, ModelSummary> = std::collections::HashMap::new();
    for m in models {
        let entry = map.entry(m.model_name.clone()).or_insert(ModelSummary {
            name: m.model_name.clone(),
            node_count: 0,
            ready_nodes: 0,
            loaded_instances: 0,
            active_requests: 0,
        });
        entry.node_count += 1;
        if m.available {
            entry.ready_nodes += 1;
        }
        if m.loaded {
            entry.loaded_instances += 1;
        }
    }
    Json(map.into_values().collect::<Vec<_>>())
}

async fn get_model(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let models = rt.storage.list_models(Some(&name)).unwrap_or_default();
    if models.is_empty() {
        return api_error(
            StatusCode::NOT_FOUND,
            "MODEL_NOT_FOUND",
            format!("Model '{name}' not found"),
        );
    }
    let nodes: Vec<ModelNodeDetail> = models
        .into_iter()
        .map(|m| {
            let node_name = rt
                .nodes
                .values()
                .find(|n| n.record.id == m.node_id)
                .map(|n| n.record.name.clone())
                .unwrap_or_default();
            ModelNodeDetail {
                node: node_name,
                available: m.available,
                loaded: m.loaded,
                digest: m.digest,
                size: m.size,
            }
        })
        .collect();
    Json(ModelDetailResponse { name, nodes }).into_response()
}

async fn sync_models(State(state): State<AppState>) -> impl IntoResponse {
    let mut rt = state.runtime.write().await;
    match sync_all_nodes(&mut rt, &state.http_client).await {
        Ok(updated) => Json(OperationResponse {
            success: true,
            message: format!("Synchronised {updated} node(s)"),
        })
        .into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, "SYNC_FAILED", e),
    }
}

async fn explain_model(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let snapshot = rt.routing_snapshot(&name);
    Json(ExplainResponse {
        explanation: explain_routing(&snapshot),
    })
}

async fn list_requests(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let now = chrono::Utc::now();
    let requests: Vec<RequestSummary> = rt
        .requests
        .values()
        .map(|r| RequestSummary {
            id: r.id.clone(),
            model: r.model.clone(),
            node: r.node_name.clone(),
            started_at: r.started_at,
            duration_ms: (now - r.started_at).num_milliseconds().max(0) as u64,
            streaming: r.streaming,
            state: r.state.clone(),
        })
        .collect();
    Json(requests)
}

async fn cancel_request(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut rt = state.runtime.write().await;
    let Some(req) = rt.requests.get(&id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "REQUEST_NOT_FOUND",
            format!("Request '{id}' not found"),
        );
    };
    let _ = req.cancel.send(true);
    rt.finish_request(&id);
    Json(OperationResponse {
        success: true,
        message: format!("Request '{id}' cancelled"),
    })
    .into_response()
}

async fn list_events(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let events = rt.storage.list_events(100).unwrap_or_default();
    let response: Vec<EventResponse> = events
        .into_iter()
        .map(|e| EventResponse {
            id: e.id,
            event_type: e.event_type,
            target: e.target,
            message: e.message,
            created_at: e.created_at,
        })
        .collect();
    Json(response)
}

async fn show_config(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let value = serde_json::to_value(&rt.config).unwrap_or(json!({}));
    Json(ConfigResponse { config: value })
}

async fn validate_config_handler(State(state): State<AppState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    match ocluster_config::validate_config(&rt.config) {
        Ok(()) => Json(OperationResponse {
            success: true,
            message: "Configuration valid".into(),
        })
        .into_response(),
        Err(e) => api_error(StatusCode::BAD_REQUEST, "CONFIG_INVALID", e.to_string()),
    }
}

async fn reload_config(State(_state): State<AppState>) -> impl IntoResponse {
    Json(OperationResponse {
        success: true,
        message: "Configuration reload acknowledged (runtime config unchanged in 0.1.0)".into(),
    })
}

fn node_detail(node: &crate::runtime::RuntimeNode) -> NodeDetailResponse {
    NodeDetailResponse {
        summary: NodeSummary {
            name: node.record.name.clone(),
            url: node.record.url.clone(),
            admin_state: node.record.admin_state,
            runtime_state: node.record.runtime_state,
            ollama_version: node.record.ollama_version.clone(),
            active_requests: node.active_requests,
            model_count: node.discovered_models.len(),
            loaded_models: node.loaded_models.len(),
            last_contact: node.last_contact,
        },
        labels: node.record.labels.clone(),
        max_concurrent: node.record.max_concurrent,
        models: node.discovered_models.clone(),
    }
}

fn api_error(status: StatusCode, code: &str, message: impl Into<String>) -> axum::response::Response {
    (
        status,
        Json(ApiErrorBody::new(code, message.into())),
    )
        .into_response()
}

/// Health summary for CLI health command.
pub async fn health_summary(runtime: &ClusterRuntime) -> HealthSummaryResponse {
    let unavailable: Vec<String> = runtime
        .nodes
        .values()
        .filter(|n| n.record.runtime_state == NodeRuntimeState::Unavailable)
        .map(|n| n.record.name.clone())
        .collect();
    let suspect: Vec<String> = runtime
        .nodes
        .values()
        .filter(|n| n.record.runtime_state == NodeRuntimeState::Suspect)
        .map(|n| n.record.name.clone())
        .collect();
    HealthSummaryResponse {
        unavailable_nodes: unavailable,
        suspect_nodes: suspect,
        models_without_nodes: vec![],
        drift_detected: false,
    }
}
