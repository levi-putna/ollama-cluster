use std::path::PathBuf;

use ocluster_core::ModelMode;
use serde::{Deserialize, Serialize};

/// Root cluster configuration (schema version 1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterConfig {
    pub version: u32,
    pub inference: ListenerConfig,
    pub management: ListenerConfig,
    pub metrics: ListenerConfig,
    pub database: DatabaseConfig,
    pub routing: RoutingSection,
    pub health: HealthSection,
    pub proxy: ProxySection,
    pub discovery: DiscoverySection,
    pub logging: LoggingSection,
    pub nodes: Vec<NodeConfig>,
    #[serde(default)]
    pub model_aliases: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub denied_models: Vec<String>,
}

/// HTTP listener configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListenerConfig {
    pub listen: String,
}

/// SQLite database path configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    pub path: String,
}

/// Routing configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingSection {
    pub policy: String,
    pub loaded_model_preference: bool,
    pub max_retries: u32,
    pub no_node_behaviour: String,
    pub queue_depth: u32,
    pub queue_timeout_ms: u64,
}

/// Health monitoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthSection {
    pub failure_threshold: u32,
    pub recovery_backoff_ms: u64,
    pub recovery_max_backoff_ms: u64,
    pub recovery_success_threshold: u32,
    pub idle_check_interval_ms: u64,
}

/// Proxy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxySection {
    pub request_body_buffer_bytes: usize,
    pub upstream_timeout_ms: u64,
    pub max_idle_connections: u32,
}

/// Model discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoverySection {
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingSection {
    pub level: String,
    pub format: String,
}

/// Registered Ollama node configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeConfig {
    pub name: String,
    pub url: String,
    pub model_mode: ModelMode,
    #[serde(default)]
    pub configured_models: Vec<String>,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
}

fn default_max_concurrent() -> u32 {
    8
}

impl ClusterConfig {
    /// Resolve a model alias to its canonical name.
    pub fn resolve_model<'a>(&'a self, model: &'a str) -> &'a str {
        self.model_aliases
            .get(model)
            .map(String::as_str)
            .unwrap_or(model)
    }

    /// Expand tilde in database path for the current user.
    pub fn database_path(&self) -> PathBuf {
        expand_tilde(&self.database.path)
    }
}

/// Expand leading `~` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
