use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::defaults::default_config;
use crate::error::ConfigError;
use crate::types::ClusterConfig;
use crate::validate::{parse_config_file, validate_config};

/// Optional overrides from CLI and environment.
#[derive(Debug, Default, Clone)]
pub struct ConfigOverrides {
    pub inference_listen: Option<String>,
    pub management_listen: Option<String>,
    pub database_path: Option<String>,
    pub log_level: Option<String>,
}

/// Load configuration with precedence: defaults → file → env → overrides.
pub fn load_config(
    path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<ClusterConfig, ConfigError> {
    let mut config = default_config();

    if let Some(path) = path {
        let contents = fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
        config = parse_config_file(path, &contents)?;
    }

    apply_env(&mut config);
    apply_overrides(&mut config, overrides);
    validate_config(&config)?;
    Ok(config)
}

fn apply_env(config: &mut ClusterConfig) {
    if let Ok(v) = env::var("OCLUSTER_INFERENCE_LISTEN") {
        config.inference.listen = v;
    }
    if let Ok(v) = env::var("OCLUSTER_MANAGEMENT_LISTEN") {
        config.management.listen = v;
    }
    if let Ok(v) = env::var("OCLUSTER_DATABASE_PATH") {
        config.database.path = v;
    }
    if let Ok(v) = env::var("OCLUSTER_LOG_LEVEL") {
        config.logging.level = v;
    }
}

fn apply_overrides(config: &mut ClusterConfig, overrides: &ConfigOverrides) {
    if let Some(v) = &overrides.inference_listen {
        config.inference.listen = v.clone();
    }
    if let Some(v) = &overrides.management_listen {
        config.management.listen = v.clone();
    }
    if let Some(v) = &overrides.database_path {
        config.database.path = v.clone();
    }
    if let Some(v) = &overrides.log_level {
        config.logging.level = v.clone();
    }
}

/// Default configuration file path for user-mode installs.
pub fn default_config_path() -> PathBuf {
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ocluster")
            .join("ocluster.toml");
    }
    PathBuf::from("ocluster.toml")
}

/// Initialise a new configuration file on disk.
pub fn init_config(path: &Path, config: &ClusterConfig) -> Result<(), ConfigError> {
    validate_config(config)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
    }
    let contents = toml::to_string_pretty(config).map_err(|e| ConfigError::Parse {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;
    fs::write(path, contents).map_err(|e| ConfigError::Io(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Covers: FR-114, TR-111, TXR-180-03
    #[test]
    fn env_overrides_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ocluster.toml");
        let mut file_config = default_config();
        file_config.inference.listen = "127.0.0.1:9999".into();
        init_config(&path, &file_config).unwrap();

        std::env::set_var("OCLUSTER_INFERENCE_LISTEN", "127.0.0.1:8888");
        let config = load_config(Some(&path), &ConfigOverrides::default()).unwrap();
        assert_eq!(config.inference.listen, "127.0.0.1:8888");
        std::env::remove_var("OCLUSTER_INFERENCE_LISTEN");
    }

    /// Covers: TR-116, TXR-180-08
    #[test]
    fn parses_schema_version() {
        let config = default_config();
        assert_eq!(config.version, 1);
    }
}
