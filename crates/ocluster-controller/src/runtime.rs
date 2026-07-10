use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use chrono::{DateTime, Utc};
use ocluster_config::ClusterConfig;
use ocluster_core::{
    circuit_breaker::CircuitBreaker, effective_models, NodeRuntimeState, RoutingConfig,
    RoutingSnapshot,
};
use ocluster_storage::{NodeRecord, Storage, StoredModel};
use uuid::Uuid;

/// Runtime view of an active inference request.
#[derive(Debug, Clone)]
pub struct ActiveRequest {
    pub id: String,
    pub model: String,
    pub node_name: String,
    pub started_at: DateTime<Utc>,
    pub streaming: bool,
    pub state: String,
    pub cancel: tokio::sync::watch::Sender<bool>,
}

/// In-memory runtime state for a node.
#[derive(Debug, Clone)]
pub struct RuntimeNode {
    pub record: NodeRecord,
    pub circuit: CircuitBreaker,
    pub active_requests: u32,
    pub queued_requests: u32,
    pub recent_failures: u32,
    pub loaded_models: Vec<String>,
    pub discovered_models: Vec<String>,
    pub last_contact: Option<DateTime<Utc>>,
}

impl RuntimeNode {
    /// Whether this node is routable for new requests.
    pub fn is_routable(&self) -> bool {
        self.record.admin_state.accepts_new_requests()
            && matches!(
                self.record.runtime_state,
                NodeRuntimeState::Ready | NodeRuntimeState::Suspect
            )
            && self.circuit.allows_routing()
    }
}

/// Shared cluster runtime state.
pub struct ClusterRuntime {
    pub config: ClusterConfig,
    pub storage: Storage,
    pub nodes: HashMap<String, RuntimeNode>,
    pub requests: HashMap<String, ActiveRequest>,
    pub queued_requests: AtomicUsize,
    pub rotation_index: AtomicUsize,
    pub started_at: Instant,
    pub shutting_down: bool,
}

impl ClusterRuntime {
    /// Build runtime from persisted storage and configuration.
    pub fn from_storage(
        config: ClusterConfig,
        storage: Storage,
    ) -> Result<Self, ocluster_storage::StorageError> {
        let failure_threshold = config.health.failure_threshold;
        let success_threshold = config.health.recovery_success_threshold;
        let nodes = storage
            .list_nodes()?
            .into_iter()
            .map(|record| {
                let name = record.name.clone();
                (
                    name,
                    RuntimeNode {
                        record,
                        circuit: CircuitBreaker::new(failure_threshold, success_threshold),
                        active_requests: 0,
                        queued_requests: 0,
                        recent_failures: 0,
                        loaded_models: Vec::new(),
                        discovered_models: Vec::new(),
                        last_contact: None,
                    },
                )
            })
            .collect();

        Ok(Self {
            config,
            storage,
            nodes,
            requests: HashMap::new(),
            queued_requests: AtomicUsize::new(0),
            rotation_index: AtomicUsize::new(0),
            started_at: Instant::now(),
            shutting_down: false,
        })
    }

    /// Build routing configuration from cluster config.
    pub fn routing_config(&self) -> RoutingConfig {
        RoutingConfig {
            loaded_model_preference: self.config.routing.loaded_model_preference,
            ..RoutingConfig::default()
        }
    }

    /// Build an immutable routing snapshot for a model.
    pub fn routing_snapshot(&self, model: &str) -> RoutingSnapshot {
        let canonical = self.config.resolve_model(model);
        let mut candidates = Vec::new();

        for node in self.nodes.values() {
            let effective = effective_models(
                node.record.model_mode,
                &node.discovered_models,
                &node.record.configured_models,
            );
            let has_model = effective.iter().any(|m| m == canonical);
            let model_loaded = node.loaded_models.iter().any(|m| m == canonical);
            let model_denied = self.config.denied_models.iter().any(|m| m == canonical);

            candidates.push(ocluster_core::RoutingCandidate {
                node_id: node.record.id.clone(),
                node_name: node.record.name.clone(),
                admin_state: node.record.admin_state,
                runtime_state: node.record.runtime_state,
                circuit_state: node.circuit.state,
                has_model,
                model_loaded,
                model_denied,
                active_requests: node.active_requests,
                queued_requests: node.queued_requests,
                max_concurrent: node.record.max_concurrent,
                priority: node.record.priority,
                recent_failures: node.recent_failures,
            });
        }

        RoutingSnapshot {
            model: canonical.to_string(),
            candidates,
            config: self.routing_config(),
            rotation_index: self.rotation_index.load(Ordering::Relaxed),
        }
    }

    /// Increment rotation index after routing decision.
    pub fn advance_rotation(&self) {
        self.rotation_index.fetch_add(1, Ordering::Relaxed);
    }

    /// Register a new active request.
    pub fn track_request(
        &mut self,
        model: String,
        node_name: String,
        streaming: bool,
    ) -> (String, tokio::sync::watch::Receiver<bool>) {
        let id = Uuid::new_v4().to_string();
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        if let Some(node) = self.nodes.get_mut(&node_name) {
            node.active_requests += 1;
        }
        self.requests.insert(
            id.clone(),
            ActiveRequest {
                id: id.clone(),
                model,
                node_name,
                started_at: Utc::now(),
                streaming,
                state: "active".into(),
                cancel: cancel_tx,
            },
        );
        (id, cancel_rx)
    }

    /// Complete or cancel a tracked request.
    pub fn finish_request(&mut self, request_id: &str) {
        if let Some(req) = self.requests.remove(request_id) {
            if let Some(node) = self.nodes.get_mut(&req.node_name) {
                node.active_requests = node.active_requests.saturating_sub(1);
            }
        }
    }

    /// Upsert a node from configuration and discovery data.
    pub fn upsert_runtime_node(
        &mut self,
        record: NodeRecord,
    ) -> Result<(), ocluster_storage::StorageError> {
        self.storage.upsert_node(&record)?;
        let circuit = CircuitBreaker::new(
            self.config.health.failure_threshold,
            self.config.health.recovery_success_threshold,
        );
        self.nodes.insert(
            record.name.clone(),
            RuntimeNode {
                record,
                circuit,
                active_requests: 0,
                queued_requests: 0,
                recent_failures: 0,
                loaded_models: Vec::new(),
                discovered_models: Vec::new(),
                last_contact: None,
            },
        );
        Ok(())
    }

    /// Apply discovered models to storage and runtime.
    pub fn apply_discovery(
        &mut self,
        node_name: &str,
        discovered: Vec<String>,
        loaded: Vec<String>,
        models: Vec<StoredModel>,
        fingerprint: Option<String>,
    ) -> Result<(), ocluster_storage::StorageError> {
        if let Some(node) = self.nodes.get_mut(node_name) {
            node.discovered_models = discovered;
            node.loaded_models = loaded;
            node.last_contact = Some(Utc::now());
            node.record.inventory_fingerprint = fingerprint;
            node.record.runtime_state = NodeRuntimeState::Ready;
            self.storage.upsert_node(&node.record.clone())?;
            self.storage.replace_node_models(&node.record.id, &models)?;
        }
        Ok(())
    }
}

/// Thread-safe handle to cluster runtime.
pub type SharedRuntime = std::sync::Arc<tokio::sync::RwLock<ClusterRuntime>>;
