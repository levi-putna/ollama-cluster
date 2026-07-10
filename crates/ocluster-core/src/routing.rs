use serde::{Deserialize, Serialize};

use crate::circuit_breaker::CircuitState;
use crate::state::{AdminState, NodeRuntimeState};

/// Supported routing policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingPolicy {
    #[default]
    LeastActiveRequests,
    RoundRobin,
    LeastQueuedRequests,
    Weighted,
    Priority,
}

/// Routing weights and preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub policy: RoutingPolicy,
    pub loaded_model_preference: bool,
    pub loaded_model_bonus: f64,
    pub active_weight: f64,
    pub queue_weight: f64,
    pub cold_model_penalty: f64,
    pub failure_penalty: f64,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            policy: RoutingPolicy::LeastActiveRequests,
            loaded_model_preference: true,
            loaded_model_bonus: 5.0,
            active_weight: 1.0,
            queue_weight: 0.5,
            cold_model_penalty: 2.0,
            failure_penalty: 3.0,
        }
    }
}

/// Node candidate for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCandidate {
    pub node_id: String,
    pub node_name: String,
    pub admin_state: AdminState,
    pub runtime_state: NodeRuntimeState,
    pub circuit_state: CircuitState,
    pub has_model: bool,
    pub model_loaded: bool,
    pub model_denied: bool,
    pub active_requests: u32,
    pub queued_requests: u32,
    pub max_concurrent: u32,
    pub priority: i32,
    pub recent_failures: u32,
}

/// Immutable snapshot for consistent routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSnapshot {
    pub model: String,
    pub candidates: Vec<RoutingCandidate>,
    pub config: RoutingConfig,
    pub rotation_index: usize,
}

/// Rejection reason for explain output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    Disabled,
    Draining,
    Drained,
    Unavailable,
    Recovering,
    CircuitOpen,
    OverConcurrencyLimit,
    ModelMissing,
    ModelDenied,
}

/// Routing explanation for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingExplanation {
    pub model: String,
    pub eligible: Vec<ScoredCandidate>,
    pub rejected: Vec<RejectedCandidate>,
    pub preferred: Option<String>,
}

/// Scored eligible candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredCandidate {
    pub node_name: String,
    pub score: f64,
}

/// Rejected candidate with reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub node_name: String,
    pub reason: RejectionReason,
}

/// Filter candidates to those eligible for routing.
pub fn filter_candidates(
    candidates: &[RoutingCandidate],
) -> (Vec<&RoutingCandidate>, Vec<RejectedCandidate>) {
    let mut eligible = Vec::new();
    let mut rejected = Vec::new();

    for c in candidates {
        if let Some(reason) = rejection_reason(c) {
            rejected.push(RejectedCandidate {
                node_name: c.node_name.clone(),
                reason,
            });
        } else {
            eligible.push(c);
        }
    }

    (eligible, rejected)
}

fn rejection_reason(c: &RoutingCandidate) -> Option<RejectionReason> {
    match c.admin_state {
        AdminState::Disabled => return Some(RejectionReason::Disabled),
        AdminState::Draining => return Some(RejectionReason::Draining),
        AdminState::Drained => return Some(RejectionReason::Drained),
        AdminState::Enabled => {}
    }

    match c.runtime_state {
        NodeRuntimeState::Unavailable | NodeRuntimeState::Warming => {
            return Some(RejectionReason::Unavailable);
        }
        NodeRuntimeState::Recovering => return Some(RejectionReason::Recovering),
        _ => {}
    }

    if c.circuit_state == CircuitState::Open {
        return Some(RejectionReason::CircuitOpen);
    }

    if !c.has_model {
        return Some(RejectionReason::ModelMissing);
    }

    if c.model_denied {
        return Some(RejectionReason::ModelDenied);
    }

    if c.active_requests >= c.max_concurrent {
        return Some(RejectionReason::OverConcurrencyLimit);
    }

    None
}

/// Calculate routing score (lower is better).
pub fn score_node(candidate: &RoutingCandidate, config: &RoutingConfig) -> f64 {
    let mut score = candidate.active_requests as f64 * config.active_weight
        + candidate.queued_requests as f64 * config.queue_weight
        + candidate.recent_failures as f64 * config.failure_penalty;

    if !candidate.model_loaded {
        score += config.cold_model_penalty;
    }

    if config.loaded_model_preference && candidate.model_loaded {
        score -= config.loaded_model_bonus;
    }

    score -= candidate.priority as f64 * 0.1;

    score
}

/// Select the best node from a snapshot.
pub fn select_node(snapshot: &RoutingSnapshot) -> Option<String> {
    let (eligible, _) = filter_candidates(&snapshot.candidates);
    if eligible.is_empty() {
        return None;
    }

    let mut scored: Vec<_> = eligible
        .iter()
        .map(|c| (c.node_name.clone(), score_node(c, &snapshot.config)))
        .collect();

    scored.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    if scored.len() > 1 && (scored[0].1 - scored[1].1).abs() < f64::EPSILON {
        let idx = snapshot.rotation_index % scored.len();
        return Some(scored[idx].0.clone());
    }

    scored.first().map(|(name, _)| name.clone())
}

/// Build routing explanation for a model.
pub fn explain_routing(snapshot: &RoutingSnapshot) -> RoutingExplanation {
    let (eligible, rejected) = filter_candidates(&snapshot.candidates);

    let mut scored: Vec<ScoredCandidate> = eligible
        .iter()
        .map(|c| ScoredCandidate {
            node_name: c.node_name.clone(),
            score: score_node(c, &snapshot.config),
        })
        .collect();
    scored.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let preferred = select_node(snapshot);

    RoutingExplanation {
        model: snapshot.model.clone(),
        eligible: scored,
        rejected,
        preferred,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_breaker::CircuitState;

    fn candidate(name: &str, active: u32, loaded: bool) -> RoutingCandidate {
        RoutingCandidate {
            node_id: format!("id-{name}"),
            node_name: name.into(),
            admin_state: AdminState::Enabled,
            runtime_state: NodeRuntimeState::Ready,
            circuit_state: CircuitState::Closed,
            has_model: true,
            model_loaded: loaded,
            model_denied: false,
            active_requests: active,
            queued_requests: 0,
            max_concurrent: 10,
            priority: 0,
            recent_failures: 0,
        }
    }

    /// Covers: FR-060, TR-060, TXR-140-01
    #[test]
    fn filters_disabled_and_unavailable() {
        let mut disabled = candidate("a", 0, true);
        disabled.admin_state = AdminState::Disabled;
        let mut unavailable = candidate("b", 0, true);
        unavailable.runtime_state = NodeRuntimeState::Unavailable;
        let ready = candidate("c", 0, true);

        let candidates = [disabled, unavailable, ready];
        let (eligible, rejected) = filter_candidates(&candidates);
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].node_name, "c");
        assert_eq!(rejected.len(), 2);
    }

    /// Covers: FR-062, FR-063, TR-062, TXR-140-02
    #[test]
    fn least_active_request_wins() {
        let config = RoutingConfig::default();
        let a = candidate("busy", 5, true);
        let b = candidate("idle", 1, true);
        let snapshot = RoutingSnapshot {
            model: "llama".into(),
            candidates: vec![a, b],
            config,
            rotation_index: 0,
        };
        assert_eq!(select_node(&snapshot).unwrap(), "idle");
    }

    /// Covers: FR-062, TR-062, TXR-140-03
    #[test]
    fn loaded_model_preference() {
        let config = RoutingConfig::default();
        let cold = candidate("cold", 2, false);
        let warm = candidate("warm", 2, true);
        let snapshot = RoutingSnapshot {
            model: "llama".into(),
            candidates: vec![cold, warm],
            config,
            rotation_index: 0,
        };
        assert_eq!(select_node(&snapshot).unwrap(), "warm");
    }

    /// Covers: FR-067, TXR-140-08
    #[test]
    fn explain_includes_rejections() {
        let mut denied = candidate("denied", 0, true);
        denied.model_denied = true;
        let ok = candidate("ok", 0, true);
        let snapshot = RoutingSnapshot {
            model: "llama".into(),
            candidates: vec![denied, ok],
            config: RoutingConfig::default(),
            rotation_index: 0,
        };
        let explanation = explain_routing(&snapshot);
        assert_eq!(explanation.rejected.len(), 1);
        assert_eq!(explanation.eligible.len(), 1);
        assert_eq!(explanation.preferred, Some("ok".into()));
    }
}
