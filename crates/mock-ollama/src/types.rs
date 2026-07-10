use serde::{Deserialize, Serialize};

/// Mock model tag entry matching Ollama `/api/tags`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockModel {
    pub name: String,
    pub digest: String,
    pub size: u64,
    pub modified_at: String,
}

/// Loaded model entry matching Ollama `/api/ps`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockLoadedModel {
    pub name: String,
}

/// Behaviour configuration for the mock server.
#[derive(Debug, Clone)]
pub struct MockConfig {
    pub version: String,
    pub models: Vec<MockModel>,
    pub loaded: Vec<MockLoadedModel>,
    pub generate_response: String,
    pub chat_response: String,
    pub stream_chunks: Vec<String>,
    pub stream_delay_ms: u64,
    pub error_status: Option<u16>,
    pub fail_connection: bool,
    pub disconnect_mid_stream: bool,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            version: "0.5.0".into(),
            models: vec![MockModel {
                name: "llama3.2:latest".into(),
                digest: "abc123".into(),
                size: 4_000_000_000,
                modified_at: "2026-01-01T00:00:00Z".into(),
            }],
            loaded: vec![],
            generate_response: r#"{"model":"llama3.2:latest","response":"hello","done":true}"#
                .into(),
            chat_response: r#"{"model":"llama3.2:latest","message":{"role":"assistant","content":"hello"},"done":true}"#
                .into(),
            stream_chunks: vec![
                r#"{"model":"llama3.2:latest","message":{"role":"assistant","content":"he"},"done":false}"#
                    .into(),
                r#"{"model":"llama3.2:latest","message":{"role":"assistant","content":"llo"},"done":true}"#
                    .into(),
            ],
            stream_delay_ms: 0,
            error_status: None,
            fail_connection: false,
            disconnect_mid_stream: false,
        }
    }
}
