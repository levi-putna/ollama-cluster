//! Configurable mock Ollama HTTP server for integration and E2E tests.

pub mod server;
pub mod types;

pub use server::{MockOllama, MockOllamaHandle};
pub use types::MockConfig;
