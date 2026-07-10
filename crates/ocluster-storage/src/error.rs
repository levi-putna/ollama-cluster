use thiserror::Error;

/// Storage layer errors.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("migration error: {0}")]
    Migration(String),
}
