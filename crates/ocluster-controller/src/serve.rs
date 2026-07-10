use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use metrics_exporter_prometheus::PrometheusBuilder;
use ocluster_config::ClusterConfig;
use ocluster_storage::Storage;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::api::{management_router, AppState};
use crate::discovery::sync_all_nodes;
use crate::proxy::ProxyState;
use crate::runtime::{ClusterRuntime, SharedRuntime};

/// Run the cluster controller (management, proxy, metrics).
pub async fn run_controller(config: ClusterConfig) -> anyhow::Result<()> {
    init_tracing(&config);

    let db_path = config.database_path();
    let storage = Storage::open(&db_path)?;
    let runtime = Arc::new(tokio::sync::RwLock::new(ClusterRuntime::from_storage(
        config.clone(),
        storage,
    )?));

    seed_nodes_from_config(&runtime, &config).await?;

    let http_client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .build()?;

    {
        let mut rt = runtime.write().await;
        let _ = sync_all_nodes(&mut rt, &http_client).await;
    }

    let app_state = AppState {
        runtime: runtime.clone(),
        http_client: http_client.clone(),
    };

    let mgmt_router = management_router(app_state);

    let proxy_state = ProxyState {
        runtime: runtime.clone(),
        client: http_client,
    };
    let proxy_router = crate::proxy::proxy_router(proxy_state);

    let mgmt_addr: SocketAddr = config.management.listen.parse()?;
    let inference_addr: SocketAddr = config.inference.listen.parse()?;
    let metrics_addr: SocketAddr = config.metrics.listen.parse()?;

    let metrics_handle = PrometheusBuilder::new()
        .with_http_listener(metrics_addr)
        .install_recorder()?;

    spawn_background_tasks(runtime.clone());

    let mgmt_listener = tokio::net::TcpListener::bind(mgmt_addr).await?;
    let inference_listener = tokio::net::TcpListener::bind(inference_addr).await?;

    tracing::info!(%mgmt_addr, "management API listening");
    tracing::info!(%inference_addr, "inference proxy listening");
    tracing::info!(%metrics_addr, "metrics listening");

    let mgmt_server = tokio::spawn(async move {
        axum::serve(mgmt_listener, mgmt_router)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    });

    let inference_server = tokio::spawn(async move {
        axum::serve(inference_listener, proxy_router)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    });

    tokio::select! {
        res = mgmt_server => res??,
        res = inference_server => res??,
        _ = shutdown_signal() => {
            tracing::info!("shutdown signal received");
            let mut rt = runtime.write().await;
            rt.shutting_down = true;
        }
    }

    let _ = metrics_handle;
    Ok(())
}

async fn seed_nodes_from_config(
    runtime: &SharedRuntime,
    config: &ClusterConfig,
) -> anyhow::Result<()> {
    let mut rt = runtime.write().await;
    for node_cfg in &config.nodes {
        if rt.nodes.contains_key(&node_cfg.name) {
            continue;
        }
        let record = ocluster_storage::NodeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: node_cfg.name.clone(),
            url: node_cfg.url.clone(),
            admin_state: ocluster_core::AdminState::Enabled,
            runtime_state: ocluster_core::NodeRuntimeState::Warming,
            ollama_version: None,
            model_mode: node_cfg.model_mode,
            configured_models: node_cfg.configured_models.clone(),
            max_concurrent: node_cfg.max_concurrent,
            priority: node_cfg.priority,
            labels: node_cfg.labels.clone(),
            inventory_fingerprint: None,
        };
        rt.upsert_runtime_node(record)?;
    }
    Ok(())
}

fn spawn_background_tasks(runtime: SharedRuntime) {
    let discovery_runtime = runtime.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = {
                let rt = discovery_runtime.read().await;
                rt.config.discovery.interval_ms
            };
            tokio::time::sleep(Duration::from_millis(interval)).await;
            let mut rt = discovery_runtime.write().await;
            let _ = sync_all_nodes(&mut rt, &client).await;
        }
    });

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let mut rt = runtime.write().await;
            for name in rt.nodes.keys().cloned().collect::<Vec<_>>() {
                if rt.nodes.get(&name).map(|n| n.record.admin_state)
                    == Some(ocluster_core::AdminState::Draining)
                {
                    let active = rt.requests.values().any(|r| r.node_name == name);
                    if !active {
                        if let Some(node) = rt.nodes.get_mut(&name) {
                            node.record.admin_state = ocluster_core::AdminState::Drained;
                            let record = node.record.clone();
                            let _ = rt.storage.upsert_node(&record);
                            let _ = rt.storage.append_event(
                                "drain_completed",
                                Some(&name),
                                "drain completed",
                            );
                        }
                    }
                }
            }
        }
    });
}

fn init_tracing(config: &ClusterConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.logging.level.clone()));

    if config.logging.format == "json" {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
}
