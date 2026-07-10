use ocluster_core::{classify_error, NodeRuntimeState};

use crate::runtime::ClusterRuntime;

/// Record a successful upstream interaction.
pub fn record_success(runtime: &mut ClusterRuntime, node_name: &str) {
    if let Some(node) = runtime.nodes.get_mut(node_name) {
        node.recent_failures = 0;
        node.circuit.record_success();
        node.record.runtime_state = NodeRuntimeState::Ready;
        node.last_contact = Some(chrono::Utc::now());
    }
}

/// Record an upstream failure for health tracking.
pub fn record_failure(
    runtime: &mut ClusterRuntime,
    node_name: &str,
    status: Option<u16>,
    kind: &str,
) {
    let error = classify_error(status, kind);
    if !error.counts_as_node_failure() {
        return;
    }

    if let Some(node) = runtime.nodes.get_mut(node_name) {
        node.recent_failures += 1;
        node.circuit.record_failure();
        if !node.circuit.allows_routing() {
            node.record.runtime_state = NodeRuntimeState::Unavailable;
            let _ = runtime.storage.append_event(
                "node_failure",
                Some(node_name),
                "node ejected by circuit breaker",
            );
        }
    }
}

/// Mark a node unavailable explicitly.
pub fn mark_unavailable(runtime: &mut ClusterRuntime, node_name: &str, reason: &str) {
    if let Some(node) = runtime.nodes.get_mut(node_name) {
        node.record.runtime_state = NodeRuntimeState::Unavailable;
        let _ = runtime
            .storage
            .append_event("node_unavailable", Some(node_name), reason);
        let _ = runtime.storage.upsert_node(&node.record.clone());
    }
}

/// Begin recovery probing for unavailable nodes.
pub fn begin_recovery(runtime: &mut ClusterRuntime, node_name: &str) {
    if let Some(node) = runtime.nodes.get_mut(node_name) {
        node.record.runtime_state = NodeRuntimeState::Recovering;
        node.circuit.begin_recovery();
    }
}

/// Compute exponential backoff delay for recovery.
pub fn recovery_delay_ms(runtime: &ClusterRuntime, attempt: u32) -> u64 {
    let base = runtime.config.health.recovery_backoff_ms;
    let max = runtime.config.health.recovery_max_backoff_ms;
    let delay = base.saturating_mul(2u64.saturating_pow(attempt.min(10)));
    delay.min(max)
}
