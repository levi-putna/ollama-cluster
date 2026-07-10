use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use futures::stream;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_stream::StreamExt as _;

use crate::types::MockConfig;

/// Handle to a running mock Ollama server.
pub struct MockOllamaHandle {
    pub addr: SocketAddr,
    pub url: String,
    shutdown: Option<oneshot::Sender<()>>,
}

impl MockOllamaHandle {
    /// Stop the mock server.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

/// Mock Ollama server builder.
pub struct MockOllama {
    config: MockConfig,
}

impl MockOllama {
    /// Create a mock server with the given configuration.
    pub fn new(config: MockConfig) -> Self {
        Self { config }
    }

    /// Start the server on an ephemeral port.
    pub async fn start(self) -> MockOllamaHandle {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let config = Arc::new(RwLock::new(self.config));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let app = Router::new()
            .route("/api/version", get(version_handler))
            .route("/api/tags", get(tags_handler))
            .route("/api/ps", get(ps_handler))
            .route("/api/generate", post(generate_handler))
            .route("/api/chat", post(chat_handler))
            .route("/api/embed", post(embed_handler))
            .route("/api/embeddings", post(embed_handler))
            .with_state(config.clone());

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        MockOllamaHandle {
            url: format!("http://{addr}"),
            addr,
            shutdown: Some(shutdown_tx),
        }
    }
}

type SharedConfig = Arc<RwLock<MockConfig>>;

async fn version_handler(State(config): State<SharedConfig>) -> impl IntoResponse {
    let config = config.read().unwrap();
    Json(serde_json::json!({ "version": config.version }))
}

async fn tags_handler(State(config): State<SharedConfig>) -> impl IntoResponse {
    let config = config.read().unwrap();
    Json(serde_json::json!({
        "models": config.models.iter().map(|m| serde_json::json!({
            "name": m.name,
            "model": m.name,
            "digest": m.digest,
            "size": m.size,
            "modified_at": m.modified_at,
            "details": {
                "family": "llama",
                "parameter_size": "8B",
                "quantization_level": "Q4_0"
            }
        })).collect::<Vec<_>>()
    }))
}

async fn ps_handler(State(config): State<SharedConfig>) -> impl IntoResponse {
    let config = config.read().unwrap();
    Json(serde_json::json!({
        "models": config.loaded.iter().map(|m| serde_json::json!({
            "name": m.name,
            "model": m.name,
        })).collect::<Vec<_>>()
    }))
}

async fn generate_handler(
    State(config): State<SharedConfig>,
    body: String,
) -> Result<Response, StatusCode> {
    let config = config.read().unwrap();
    if let Some(status) = config.error_status {
        return Err(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let stream = body.contains("\"stream\":true") || body.contains("\"stream\": true");
    if stream {
        Ok(streaming_response(&config))
    } else {
        Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            config.generate_response.clone(),
        )
            .into_response())
    }
}

async fn chat_handler(
    State(config): State<SharedConfig>,
    body: String,
) -> Result<Response, StatusCode> {
    let config = config.read().unwrap();
    if let Some(status) = config.error_status {
        return Err(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let stream = body.contains("\"stream\":true") || body.contains("\"stream\": true");
    if stream {
        Ok(streaming_response(&config))
    } else {
        Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            config.chat_response.clone(),
        )
            .into_response())
    }
}

async fn embed_handler(State(config): State<SharedConfig>) -> Result<Response, StatusCode> {
    let config = config.read().unwrap();
    if config.error_status.is_some() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"embedding":[0.1,0.2,0.3]}"#,
    )
        .into_response())
}

fn streaming_response(config: &MockConfig) -> Response {
    let delay = config.stream_delay_ms;
    let chunks = config.stream_chunks.clone();
    let disconnect = config.disconnect_mid_stream;

    let stream = stream::iter(chunks.into_iter().enumerate()).then(move |(idx, chunk)| {
        let disconnect = disconnect && idx == 0;
        async move {
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            if disconnect {
                Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "mid-stream disconnect",
                ))
            } else {
                Ok(Bytes::from(format!("{chunk}\n")))
            }
        }
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(stream))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MockConfig;

    /// Covers: TXR-010, TR-212
    #[tokio::test]
    async fn mock_serves_tags_and_generate() {
        let handle = MockOllama::new(MockConfig::default()).start().await;
        let client = reqwest::Client::new();
        let tags: serde_json::Value = client
            .get(format!("{}/api/tags", handle.url))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(tags["models"].as_array().unwrap().len() > 0);

        let resp = client
            .post(format!("{}/api/generate", handle.url))
            .header("content-type", "application/json")
            .body(r#"{"model":"llama3.2:latest","prompt":"hi","stream":false}"#)
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        handle.shutdown().await;
    }
}
