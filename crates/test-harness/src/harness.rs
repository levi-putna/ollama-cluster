use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use mock_ollama::{MockConfig, MockOllama};
use ocluster_config::{init_config, defaults::default_config, types::NodeConfig};
use ocluster_core::ModelMode;
use tempfile::TempDir;

/// A test cluster with mock Ollama nodes and a running controller process.
pub struct TestCluster {
    pub _temp: TempDir,
    pub config_path: PathBuf,
    pub management_url: String,
    pub inference_url: String,
    pub mock_urls: Vec<String>,
    controller: Option<Child>,
}

impl TestCluster {
    /// Start a test cluster with the given number of mock Ollama nodes.
    pub async fn start(node_count: usize) -> Self {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("ocluster.toml");
        let db_path = temp.path().join("ocluster.db");

        let mut mock_urls = Vec::new();
        let mut nodes = Vec::new();

        for i in 0..node_count {
            let mut config = MockConfig::default();
            config.models[0].name = "llama3.2:latest".to_string();
            let handle = MockOllama::new(config).start().await;
            let url = handle.url.clone();
            mock_urls.push(url.clone());
            nodes.push(NodeConfig {
                name: format!("node-{i}"),
                url,
                model_mode: ModelMode::Discover,
                configured_models: vec![],
                max_concurrent: 8,
                priority: 0,
                labels: Default::default(),
            });
            // Leak mock servers for test duration — they stop when process exits
            std::mem::forget(handle);
        }

        let mut cluster_config = default_config();
        cluster_config.inference.listen = "127.0.0.1:0".into(); // overwritten below
        cluster_config.management.listen = "127.0.0.1:0".into();
        cluster_config.metrics.listen = "127.0.0.1:0".into();
        cluster_config.database.path = db_path.display().to_string();
        cluster_config.discovery.interval_ms = 60_000;
        cluster_config.nodes = nodes;

        // Bind to ephemeral ports by picking high ports — controller parses listen addresses
        cluster_config.inference.listen = find_free_port();
        cluster_config.management.listen = find_free_port();
        cluster_config.metrics.listen = find_free_port();

        init_config(&config_path, &cluster_config).unwrap();

        let management_url = format!("http://{}", cluster_config.management.listen);
        let inference_url = format!("http://{}", cluster_config.inference.listen);

        let binary = std::env::var("CARGO_BIN_EXE_ocluster")
            .unwrap_or_else(|_| "ocluster".into());
        let controller = Command::new(binary)
            .arg("serve")
            .arg("--config")
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn ocluster serve");

        poll_until(
            Duration::from_secs(15),
            || async {
                reqwest::Client::new()
                    .get(format!("{management_url}/health/live"))
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false)
            },
        )
        .await
        .expect("controller did not become ready");

        Self {
            _temp: temp,
            config_path,
            management_url,
            inference_url,
            mock_urls,
            controller: Some(controller),
        }
    }

    /// Stop the controller process.
    pub fn shutdown(mut self) {
        if let Some(mut child) = self.controller.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Poll until a condition is true or timeout.
pub async fn poll_until<F, Fut>(timeout: Duration, mut f: F) -> Option<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        if f().await {
            return Some(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    None
}

fn find_free_port() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    format!("127.0.0.1:{port}")
}
