//! Ollama Cluster controller runtime.

pub mod api;
pub mod discovery;
pub mod error;
pub mod health;
pub mod proxy;
pub mod runtime;
pub mod serve;

pub use error::ControllerError;
pub use runtime::{ClusterRuntime, RuntimeNode};
pub use serve::run_controller;
