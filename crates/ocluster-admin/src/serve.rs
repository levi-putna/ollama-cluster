use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use reqwest::Client;
use rust_embed::RustEmbed;
use tracing::info;

/// Embedded static assets for the admin panel.
#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

/// Shared state for the admin web server.
#[derive(Clone)]
struct AdminState {
    backend: String,
    client: Client,
}

/// Start the admin web server on `listen`, proxying API calls to `management_url`.
pub async fn run_admin_server(listen: &str, management_url: &str) -> Result<()> {
    let addr: SocketAddr = listen.parse().context("invalid admin listen address")?;
    let state = AdminState {
        backend: management_url.trim_end_matches('/').to_string(),
        client: Client::new(),
    };

    let app = Router::new().fallback(fallback_handler).with_state(state);

    info!(%listen, management_url, "admin panel listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind admin listener")?;
    axum::serve(listener, app)
        .await
        .context("admin server error")?;

    Ok(())
}

async fn fallback_handler(State(state): State<AdminState>, req: Request) -> Response {
    let path = req.uri().path();
    if path.starts_with("/api/") || path.starts_with("/health/") {
        return proxy_api(State(state), req).await;
    }
    serve_static(req.uri().clone()).await
}

async fn proxy_api(State(state): State<AdminState>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", state.backend, path_and_query);

    let mut builder = state.client.request(parts.method, url).body(bytes);
    for (name, value) in parts.headers.iter() {
        if name != header::HOST && name != header::CONNECTION {
            builder = builder.header(name, value);
        }
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            let headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();
            let mut response = Response::builder().status(status);
            if let Some(response_headers) = response.headers_mut() {
                *response_headers = headers;
            }
            response
                .body(Body::from(body))
                .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
        }
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

async fn serve_static(uri: Uri) -> Response {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None if !path.contains('.') => serve_file("index.html"),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn serve_file(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
