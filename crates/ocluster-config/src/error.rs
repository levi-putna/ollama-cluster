use thiserror::Error;

/// Configuration validation and loading errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("parse error at {path}: {reason}")]
    Parse { path: String, reason: String },

    #[error("validation error at {path}.{field}: {reason}")]
    Validation {
        path: String,
        field: String,
        reason: String,
    },

    #[error("unsupported schema version {version}")]
    UnsupportedVersion { version: u32 },

    #[error("io error: {0}")]
    Io(String),
}
