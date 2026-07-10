use thiserror::Error;

/// Controller runtime errors.
#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("configuration error: {0}")]
    Config(#[from] ocluster_config::ConfigError),

    #[error("storage error: {0}")]
    Storage(#[from] ocluster_storage::StorageError),

    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("request not found: {0}")]
    RequestNotFound(String),

    #[error("operation rejected: {0}")]
    Rejected(String),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
