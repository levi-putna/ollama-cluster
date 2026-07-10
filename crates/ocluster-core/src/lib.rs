//! Shared domain types for Ollama Cluster.

pub mod circuit_breaker;
pub mod error;
pub mod fingerprint;
pub mod model_mode;
pub mod retry;
pub mod routing;
pub mod state;

pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use error::CoreError;
pub use fingerprint::inventory_fingerprint;
pub use model_mode::{effective_models, ModelMode};
pub use retry::{classify_error, RetryPolicy, RetryableError};
pub use routing::{
    explain_routing, filter_candidates, score_node, select_node, RoutingCandidate,
    RoutingConfig, RoutingExplanation, RoutingPolicy, RoutingSnapshot,
};
pub use state::{
    AdminState, NodeRuntimeState, RuntimeStateTransition, validate_runtime_transition,
};
