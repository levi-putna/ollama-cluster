use serde::{Deserialize, Serialize};

/// Administrative node state persisted across restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdminState {
    #[default]
    Enabled,
    Disabled,
    Draining,
    Drained,
}

impl AdminState {
    /// Whether the node may receive new routing assignments.
    pub fn accepts_new_requests(&self) -> bool {
        matches!(self, AdminState::Enabled)
    }
}

/// Runtime health state rebuilt from probes and request outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeRuntimeState {
    #[default]
    Ready,
    Suspect,
    Unavailable,
    Recovering,
    Warming,
}

impl std::fmt::Display for NodeRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NodeRuntimeState::Ready => "ready",
            NodeRuntimeState::Suspect => "suspect",
            NodeRuntimeState::Unavailable => "unavailable",
            NodeRuntimeState::Recovering => "recovering",
            NodeRuntimeState::Warming => "warming",
        };
        write!(f, "{s}")
    }
}

/// Valid runtime state transitions.
pub fn validate_runtime_transition(
    from: NodeRuntimeState,
    to: NodeRuntimeState,
) -> Result<(), crate::CoreError> {
    if from == to {
        return Ok(());
    }

    let valid = matches!(
        (from, to),
        (NodeRuntimeState::Ready, NodeRuntimeState::Suspect)
            | (NodeRuntimeState::Ready, NodeRuntimeState::Unavailable)
            | (NodeRuntimeState::Suspect, NodeRuntimeState::Ready)
            | (NodeRuntimeState::Suspect, NodeRuntimeState::Unavailable)
            | (NodeRuntimeState::Unavailable, NodeRuntimeState::Recovering)
            | (NodeRuntimeState::Recovering, NodeRuntimeState::Ready)
            | (NodeRuntimeState::Recovering, NodeRuntimeState::Unavailable)
            | (NodeRuntimeState::Warming, NodeRuntimeState::Ready)
            | (NodeRuntimeState::Warming, NodeRuntimeState::Unavailable)
    );

    if valid {
        Ok(())
    } else {
        Err(crate::CoreError::InvalidStateTransition {
            from: from.to_string(),
            to: to.to_string(),
        })
    }
}

/// Runtime state transition helper.
pub struct RuntimeStateTransition;

#[cfg(test)]
mod tests {
    use super::*;

    /// Covers: FR-020, FR-021, TR-072, TXR-020
    #[test]
    fn valid_runtime_transitions() {
        assert!(validate_runtime_transition(
            NodeRuntimeState::Ready,
            NodeRuntimeState::Suspect
        )
        .is_ok());
        assert!(validate_runtime_transition(
            NodeRuntimeState::Unavailable,
            NodeRuntimeState::Recovering
        )
        .is_ok());
        assert!(validate_runtime_transition(
            NodeRuntimeState::Recovering,
            NodeRuntimeState::Ready
        )
        .is_ok());
    }

    /// Covers: FR-020, FR-021, TXR-020
    #[test]
    fn invalid_runtime_transitions_rejected() {
        assert!(validate_runtime_transition(
            NodeRuntimeState::Ready,
            NodeRuntimeState::Recovering
        )
        .is_err());
    }

    /// Covers: FR-013, TR-072, TXR-111-03
    #[test]
    fn disabled_admin_state_blocks_routing() {
        assert!(!AdminState::Disabled.accepts_new_requests());
        assert!(!AdminState::Draining.accepts_new_requests());
        assert!(AdminState::Enabled.accepts_new_requests());
    }
}
