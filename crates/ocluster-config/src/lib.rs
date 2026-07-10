//! Configuration loading, validation, and precedence for Ollama Cluster.

pub mod defaults;
pub mod error;
pub mod loader;
pub mod types;
pub mod validate;

pub use error::ConfigError;
pub use loader::{default_config_path, init_config, load_config, ConfigOverrides};
pub use types::ClusterConfig;
pub use validate::validate_config;
