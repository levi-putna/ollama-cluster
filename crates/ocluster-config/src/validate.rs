use std::collections::HashSet;

use url::Url;

use crate::error::ConfigError;
use crate::types::ClusterConfig;

const SUPPORTED_VERSION: u32 = 1;

/// Validate a cluster configuration.
pub fn validate_config(config: &ClusterConfig) -> Result<(), ConfigError> {
    if config.version != SUPPORTED_VERSION {
        return Err(ConfigError::UnsupportedVersion {
            version: config.version,
        });
    }

    validate_listen("inference", &config.inference.listen)?;
    validate_listen("management", &config.management.listen)?;
    validate_listen("metrics", &config.metrics.listen)?;

    if config.health.failure_threshold == 0 {
        return Err(ConfigError::Validation {
            path: "ocluster.toml".into(),
            field: "health.failure_threshold".into(),
            reason: "must be greater than 0".into(),
        });
    }

    if config.proxy.request_body_buffer_bytes == 0 {
        return Err(ConfigError::Validation {
            path: "ocluster.toml".into(),
            field: "proxy.request_body_buffer_bytes".into(),
            reason: "must be greater than 0".into(),
        });
    }

    let mut names = HashSet::new();
    for node in &config.nodes {
        if !names.insert(&node.name) {
            return Err(ConfigError::Validation {
                path: "ocluster.toml".into(),
                field: format!("nodes.{}", node.name),
                reason: "duplicate node name".into(),
            });
        }

        validate_node_url(&node.url)?;
    }

    Ok(())
}

fn validate_listen(field: &str, listen: &str) -> Result<(), ConfigError> {
    if listen.trim().is_empty() {
        return Err(ConfigError::Validation {
            path: "ocluster.toml".into(),
            field: format!("{field}.listen"),
            reason: "must not be empty".into(),
        });
    }
    Ok(())
}

fn validate_node_url(raw: &str) -> Result<(), ConfigError> {
    let parsed = Url::parse(raw).map_err(|e| ConfigError::Validation {
        path: "ocluster.toml".into(),
        field: "nodes.url".into(),
        reason: e.to_string(),
    })?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(ConfigError::Validation {
            path: "ocluster.toml".into(),
            field: "nodes.url".into(),
            reason: "must use http or https scheme".into(),
        });
    }

    Ok(())
}

/// Check whether a node URL is allowed (SSRF protection).
pub fn is_url_allowed(raw: &str, block_private: bool) -> Result<(), ConfigError> {
    let parsed = Url::parse(raw).map_err(|e| ConfigError::Validation {
        path: "ocluster.toml".into(),
        field: "nodes.url".into(),
        reason: e.to_string(),
    })?;

    if block_private {
        if let Some(host) = parsed.host_str() {
            if host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host == "169.254.169.254"
            {
                return Err(ConfigError::Validation {
                    path: "ocluster.toml".into(),
                    field: "nodes.url".into(),
                    reason: "blocked loopback or metadata address".into(),
                });
            }
        }
    }

    Ok(())
}

/// Parse TOML from a file path.
pub fn parse_config_file(
    path: &std::path::Path,
    contents: &str,
) -> Result<ClusterConfig, ConfigError> {
    toml::from_str(contents).map_err(|e| ConfigError::Parse {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::default_config;
    use crate::types::NodeConfig;
    use ocluster_core::ModelMode;

    /// Covers: FR-003, TR-114, TXR-100-04
    #[test]
    fn rejects_duplicate_node_names() {
        let mut config = default_config();
        config.nodes = vec![
            NodeConfig {
                name: "a".into(),
                url: "http://10.0.0.1:11434".into(),
                model_mode: ModelMode::Discover,
                configured_models: vec![],
                max_concurrent: 8,
                priority: 0,
                labels: Default::default(),
            },
            NodeConfig {
                name: "a".into(),
                url: "http://10.0.0.2:11434".into(),
                model_mode: ModelMode::Discover,
                configured_models: vec![],
                max_concurrent: 8,
                priority: 0,
                labels: Default::default(),
            },
        ];
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
    }

    /// Covers: FR-003, TXR-100-05
    #[test]
    fn rejects_invalid_node_url() {
        let mut config = default_config();
        config.nodes.push(NodeConfig {
            name: "bad".into(),
            url: "not-a-url".into(),
            model_mode: ModelMode::Discover,
            configured_models: vec![],
            max_concurrent: 8,
            priority: 0,
            labels: Default::default(),
        });
        assert!(validate_config(&config).is_err());
    }

    /// Covers: TXR-110-11, TR-176
    #[test]
    fn ssrf_blocks_loopback_when_enabled() {
        assert!(is_url_allowed("http://127.0.0.1:11434", true).is_err());
        assert!(is_url_allowed("http://10.0.0.5:11434", true).is_ok());
    }
}
