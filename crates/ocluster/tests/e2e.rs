//! End-to-end integration tests for Ollama Cluster.
//!
//! Covers: TXR-022, TR-213

use mock_ollama::{MockConfig, MockOllama};
use ocluster_client::ManagementClient;
use ocluster_config::{defaults::default_config, init_config, types::NodeConfig};
use ocluster_core::ModelMode;
use ocluster_protocol::AddNodeRequest;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

fn free_port() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    format!("127.0.0.1:{}", listener.local_addr().unwrap().port())
}

async fn wait_for_health(url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..100 {
        if client
            .get(format!("{url}/health/live"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("controller did not become ready");
}

/// Covers: TXR-100-01, TXR-022-01
#[tokio::test]
async fn init_serve_and_status() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("ocluster.toml");
    let mut config = default_config();
    config.inference.listen = free_port();
    config.management.listen = free_port();
    config.metrics.listen = free_port();
    config.database.path = temp.path().join("ocluster.db").display().to_string();
    config.discovery.interval_ms = 300_000;
    init_config(&config_path, &config).unwrap();

    let binary = env!("CARGO_BIN_EXE_ocluster");
    let mut child = Command::new(binary)
        .arg("serve")
        .arg("--config")
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mgmt = format!("http://{}", config.management.listen);
    wait_for_health(&mgmt).await;

    let client = ManagementClient::new(&mgmt).unwrap();
    let status = client.cluster_status().await.unwrap();
    assert_eq!(status.state, "running");

    let _ = child.kill();
    let _ = child.wait();
}

/// Covers: TXR-110-01, TXR-022-02, TXR-150-01
#[tokio::test]
async fn proxy_routes_to_mock_ollama() {
    let mock = MockOllama::new(MockConfig::default()).start().await;

    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("ocluster.toml");
    let mut config = default_config();
    config.inference.listen = free_port();
    config.management.listen = free_port();
    config.metrics.listen = free_port();
    config.database.path = temp.path().join("ocluster.db").display().to_string();
    config.discovery.interval_ms = 300_000;
    config.nodes.push(NodeConfig {
        name: "gpu-01".into(),
        url: mock.url.clone(),
        model_mode: ModelMode::Discover,
        configured_models: vec![],
        max_concurrent: 8,
        priority: 0,
        labels: Default::default(),
    });
    init_config(&config_path, &config).unwrap();

    let binary = env!("CARGO_BIN_EXE_ocluster");
    let mut child = Command::new(binary)
        .arg("serve")
        .arg("--config")
        .arg(&config_path)
        .spawn()
        .unwrap();

    let mgmt = format!("http://{}", config.management.listen);
    let inference = format!("http://{}", config.inference.listen);
    wait_for_health(&mgmt).await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let http = reqwest::Client::new();
    let resp = http
        .post(format!("{inference}/api/generate"))
        .json(&serde_json::json!({
            "model": "llama3.2:latest",
            "prompt": "hello",
            "stream": false
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    assert!(resp.headers().contains_key("x-ocluster-node"));

    let _ = child.kill();
    mock.shutdown().await;
}

/// Covers: TXR-110-07, TXR-022-03
#[tokio::test]
async fn disabled_node_not_used_for_routing() {
    let mock1 = MockOllama::new(MockConfig::default()).start().await;
    let mock2 = MockOllama::new(MockConfig::default()).start().await;

    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("ocluster.toml");
    let mut config = default_config();
    config.inference.listen = free_port();
    config.management.listen = free_port();
    config.metrics.listen = free_port();
    config.database.path = temp.path().join("ocluster.db").display().to_string();
    config.discovery.interval_ms = 300_000;
    init_config(&config_path, &config).unwrap();

    let binary = env!("CARGO_BIN_EXE_ocluster");
    let mut child = Command::new(binary)
        .arg("serve")
        .arg("--config")
        .arg(&config_path)
        .spawn()
        .unwrap();

    let mgmt = format!("http://{}", config.management.listen);
    wait_for_health(&mgmt).await;

    let client = ManagementClient::new(&mgmt).unwrap();
    client
        .add_node(&AddNodeRequest {
            name: "node-a".into(),
            url: mock1.url.clone(),
            model_mode: None,
            max_concurrent: None,
        })
        .await
        .unwrap();
    client
        .add_node(&AddNodeRequest {
            name: "node-b".into(),
            url: mock2.url.clone(),
            model_mode: None,
            max_concurrent: None,
        })
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;
    client.disable_node("node-a").await.unwrap();

    let inference = format!("http://{}", config.inference.listen);
    let http = reqwest::Client::new();
    let resp = http
        .post(format!("{inference}/api/generate"))
        .json(&serde_json::json!({
            "model": "llama3.2:latest",
            "prompt": "hello",
            "stream": false
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let node = resp
        .headers()
        .get("x-ocluster-node")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(node, "node-b");

    let _ = child.kill();
    mock1.shutdown().await;
    mock2.shutdown().await;
}
