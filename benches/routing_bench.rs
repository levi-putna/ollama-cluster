//! Performance benchmark scaffold for TXR-300.
//!
//! Run with: cargo bench -p ocluster-core

use ocluster_core::{
    routing::{RoutingCandidate, RoutingConfig, RoutingSnapshot, score_node, select_node},
    state::{AdminState, NodeRuntimeState},
    circuit_breaker::CircuitState,
};

fn bench_snapshot() -> RoutingSnapshot {
    let candidates: Vec<RoutingCandidate> = (0..10)
        .map(|i| RoutingCandidate {
            node_id: format!("id-{i}"),
            node_name: format!("node-{i}"),
            admin_state: AdminState::Enabled,
            runtime_state: NodeRuntimeState::Ready,
            circuit_state: CircuitState::Closed,
            has_model: true,
            model_loaded: i % 2 == 0,
            model_denied: false,
            active_requests: i,
            queued_requests: 0,
            max_concurrent: 10,
            priority: 0,
            recent_failures: 0,
        })
        .collect();

    RoutingSnapshot {
        model: "llama3.2:latest".into(),
        candidates,
        config: RoutingConfig::default(),
        rotation_index: 0,
    }
}

fn main() {
    let snapshot = bench_snapshot();
    let start = std::time::Instant::now();
    for _ in 0..100_000 {
        let _ = select_node(&snapshot);
        for c in &snapshot.candidates {
            let _ = score_node(c, &snapshot.config);
        }
    }
    println!(
        "100k routing decisions in {:?}",
        start.elapsed()
    );
}
