use crate::types::{
    ClusterConfig, DatabaseConfig, DiscoverySection, HealthSection, ListenerConfig, LoggingSection,
    ProxySection, RoutingSection,
};

/// Built-in default configuration for user-mode installs.
pub fn default_config() -> ClusterConfig {
    ClusterConfig {
        version: 1,
        inference: ListenerConfig {
            listen: "127.0.0.1:11434".into(),
        },
        management: ListenerConfig {
            listen: "127.0.0.1:11600".into(),
        },
        metrics: ListenerConfig {
            listen: "127.0.0.1:11601".into(),
        },
        database: DatabaseConfig {
            path: "~/.local/share/ocluster/ocluster.db".into(),
        },
        routing: RoutingSection {
            policy: "least_active_requests".into(),
            loaded_model_preference: true,
            max_retries: 2,
            no_node_behaviour: "reject".into(),
            queue_depth: 32,
            queue_timeout_ms: 60_000,
        },
        health: HealthSection {
            failure_threshold: 3,
            recovery_backoff_ms: 1_000,
            recovery_max_backoff_ms: 60_000,
            recovery_success_threshold: 2,
            idle_check_interval_ms: 60_000,
        },
        proxy: ProxySection {
            request_body_buffer_bytes: 1_048_576,
            upstream_timeout_ms: 300_000,
            max_idle_connections: 32,
        },
        discovery: DiscoverySection {
            interval_ms: 300_000,
            timeout_ms: 10_000,
        },
        logging: LoggingSection {
            level: "info".into(),
            format: "text".into(),
        },
        nodes: Vec::new(),
        model_aliases: std::collections::HashMap::new(),
        denied_models: Vec::new(),
    }
}
