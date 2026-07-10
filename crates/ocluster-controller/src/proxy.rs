use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Router,
};
use bytes::Bytes;
use futures::StreamExt;
use metrics::{counter, gauge, histogram};
use ocluster_core::select_node;
use reqwest::Client;
use tracing::{info, warn};

use crate::health::{record_failure, record_success};
use crate::runtime::SharedRuntime;

/// Shared proxy handler state.
#[derive(Clone)]
pub struct ProxyState {
    pub runtime: SharedRuntime,
    pub client: Client,
}

/// Build the inference proxy router.
pub fn proxy_router(state: ProxyState) -> Router {
    Router::new()
        .route("/api/tags", get(cluster_tags))
        .route("/api/ps", get(cluster_ps))
        .route("/api/generate", post(proxy_inference))
        .route("/api/chat", post(proxy_inference))
        .route("/api/embed", post(proxy_inference))
        .route("/api/embeddings", post(proxy_inference))
        .fallback(any(proxy_inference))
        .with_state(state)
}

async fn cluster_tags(State(state): State<ProxyState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let models = rt.storage.list_models(None).unwrap_or_default();
    let tags: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.model_name,
                "model": m.model_name,
                "digest": m.digest,
                "size": m.size,
            })
        })
        .collect();
    JsonBody(serde_json::json!({ "models": tags }))
}

async fn cluster_ps(State(state): State<ProxyState>) -> impl IntoResponse {
    let rt = state.runtime.read().await;
    let models = rt.storage.list_models(None).unwrap_or_default();
    let loaded: Vec<serde_json::Value> = models
        .iter()
        .filter(|m| m.loaded)
        .map(|m| serde_json::json!({ "name": m.model_name, "model": m.model_name }))
        .collect();
    JsonBody(serde_json::json!({ "models": loaded }))
}

async fn proxy_inference(State(state): State<ProxyState>, req: Request) -> Response {
    let method = req.method().clone();
    let uri = req.uri().path().to_string();
    let headers = req.headers().clone();
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "failed to read request body"),
    };

    let model = extract_model(&body_bytes);
    let streaming = is_streaming(&body_bytes);

    let (max_retries, buffer_limit, _no_node_reject) = {
        let rt = state.runtime.read().await;
        if rt.shutting_down {
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "cluster shutting down");
        }
        if let Some(model_name) = &model {
            if rt.config.denied_models.iter().any(|m| m == model_name) {
                return error_response(StatusCode::FORBIDDEN, "model denied");
            }
        }
        (
            rt.config.routing.max_retries,
            rt.config.proxy.request_body_buffer_bytes,
            rt.config.routing.no_node_behaviour == "reject",
        )
    };

    if body_bytes.len() > buffer_limit {
        return error_response(StatusCode::PAYLOAD_TOO_LARGE, "request body too large");
    }

    let snapshot = if let Some(model_name) = &model {
        let rt = state.runtime.read().await;
        rt.routing_snapshot(model_name)
    } else {
        return error_response(StatusCode::BAD_REQUEST, "model not specified in request");
    };

    let mut attempts = 0u32;
    let mut tried_nodes = Vec::new();
    let mut last_error = String::from("no eligible nodes");

    loop {
        let node_name = {
            let mut snap = snapshot.clone();
            snap.candidates.retain(|c| !tried_nodes.contains(&c.node_name));
            select_node(&snap)
        };

        let Some(node_name) = node_name else {
            return error_response(StatusCode::SERVICE_UNAVAILABLE, &last_error);
        };

        tried_nodes.push(node_name.clone());

        let node_url = {
            let rt = state.runtime.read().await;
            rt.nodes
                .get(&node_name)
                .map(|n| n.record.url.clone())
                .unwrap_or_default()
        };

        let request_id = {
            let mut rt = state.runtime.write().await;
            let (id, _cancel) = rt.track_request(
                model.clone().unwrap_or_default(),
                node_name.clone(),
                streaming,
            );
            rt.advance_rotation();
            id
        };

        counter!(
            "ocluster_requests_total",
            "model" => model.clone().unwrap_or_default()
        )
        .increment(1);
        gauge!("ocluster_requests_active").increment(1.0);

        let upstream_url = format!("{}{}", node_url.trim_end_matches('/'), uri);

        let start = std::time::Instant::now();
        let result = forward_request(
            &state.client,
            &method,
            &upstream_url,
            &headers,
            body_bytes.clone(),
            &node_name,
            &request_id,
            attempts,
        )
        .await;

        {
            let mut rt = state.runtime.write().await;
            rt.finish_request(&request_id);
        }
        gauge!("ocluster_requests_active").decrement(1.0);
        histogram!("ocluster_request_duration_seconds").record(start.elapsed().as_secs_f64());

        match result {
            Ok(resp) => {
                let mut rt = state.runtime.write().await;
                record_success(&mut rt, &node_name);
                return resp;
            }
            Err(err) => {
                last_error = err.message.clone();
                if err.bytes_sent {
                    return error_response(StatusCode::BAD_GATEWAY, &err.message);
                }
                let mut rt = state.runtime.write().await;
                record_failure(&mut rt, &node_name, err.status, &err.kind);
                counter!(
                    "ocluster_upstream_failures_total",
                    "node" => node_name.clone()
                )
                .increment(1);
            }
        }

        attempts += 1;
        if attempts >= max_retries {
            counter!("ocluster_retries_total").increment(attempts as u64);
            return error_response(StatusCode::BAD_GATEWAY, &last_error);
        }
        counter!("ocluster_retries_total").increment(1);
        warn!(attempts, node = %node_name, "retrying request on alternate node");
    }
}

struct ForwardError {
    message: String,
    status: Option<u16>,
    kind: String,
    bytes_sent: bool,
}

#[allow(clippy::too_many_arguments)]
async fn forward_request(
    client: &Client,
    method: &axum::http::Method,
    url: &str,
    headers: &HeaderMap,
    body: Bytes,
    node_name: &str,
    request_id: &str,
    retry_count: u32,
) -> Result<Response, ForwardError> {
    let mut req = client.request(method.clone(), url).body(body.to_vec());

    for (key, value) in headers.iter() {
        if key == header::HOST || key == header::CONTENT_LENGTH {
            continue;
        }
        req = req.header(key, value);
    }

    let resp = req.send().await.map_err(|e| ForwardError {
        message: e.to_string(),
        status: None,
        kind: "connection_refused".into(),
        bytes_sent: false,
    })?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ForwardError {
            message: text,
            status: Some(status.as_u16()),
            kind: if status.is_server_error() {
                "http_server_error".into()
            } else {
                "application_error".into()
            },
            bytes_sent: false,
        });
    }

    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let is_stream = content_type.contains("ndjson") || content_type.contains("stream");

    if is_stream {
        let byte_stream = resp.bytes_stream().map(|chunk| {
            chunk.map_err(std::io::Error::other)
        });

        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            "X-OCluster-Node",
            HeaderValue::from_str(node_name).unwrap_or(HeaderValue::from_static("unknown")),
        );
        response_headers.insert(
            "X-OCluster-Request-ID",
            HeaderValue::from_str(request_id).unwrap_or(HeaderValue::from_static("unknown")),
        );
        response_headers.insert(
            "X-OCluster-Retry-Count",
            HeaderValue::from_str(&retry_count.to_string()).unwrap(),
        );
        if !content_type.is_empty() {
            response_headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(&content_type).unwrap(),
            );
        }

        info!(request_id, node = %node_name, "streaming response");
        Ok((
            StatusCode::OK,
            response_headers,
            Body::from_stream(byte_stream),
        )
            .into_response())
    } else {
        let bytes = resp.bytes().await.map_err(|e| ForwardError {
            message: e.to_string(),
            status: None,
            kind: "upstream_disconnect_before".into(),
            bytes_sent: false,
        })?;

        let mut response_headers = HeaderMap::new();
        response_headers.insert("X-OCluster-Node", HeaderValue::from_str(node_name).unwrap());
        response_headers.insert(
            "X-OCluster-Request-ID",
            HeaderValue::from_str(request_id).unwrap(),
        );
        response_headers.insert(
            "X-OCluster-Retry-Count",
            HeaderValue::from_str(&retry_count.to_string()).unwrap(),
        );
        response_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        Ok((StatusCode::OK, response_headers, bytes).into_response())
    }
}

fn extract_model(body: &Bytes) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    value.get("model").and_then(|m| m.as_str()).map(String::from)
}

fn is_streaming(body: &Bytes) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("stream").and_then(|s| s.as_bool()))
        .unwrap_or(false)
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({ "error": message }).to_string(),
    )
        .into_response()
}

struct JsonBody(serde_json::Value);

impl IntoResponse for JsonBody {
    fn into_response(self) -> Response {
        (StatusCode::OK, axum::Json(self.0)).into_response()
    }
}
