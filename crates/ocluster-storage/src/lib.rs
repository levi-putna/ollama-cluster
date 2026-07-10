//! SQLite persistence layer for Ollama Cluster.

pub mod error;
pub mod migrations;
pub mod store;

pub use error::StorageError;
pub use store::{ClusterEvent, NodeRecord, Storage, StoredModel};
