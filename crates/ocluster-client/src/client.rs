use anyhow::{anyhow, Context, Result};
use ocluster_protocol::{
    AddNodeRequest, ClusterStatusResponse, EventResponse, ExplainResponse, ModelDetailResponse,
    ModelSummary, NodeDetailResponse, NodeSummary, OperationResponse, RequestSummary,
    UpdateNodeRequest, VersionResponse,
};
use reqwest::Url;
use serde::de::DeserializeOwned;

/// Client for the cluster management API.
pub struct ManagementClient {
    base_url: Url,
    http: reqwest::Client,
}

impl ManagementClient {
    /// Connect to a management API endpoint.
    pub fn new(base_url: impl AsRef<str>) -> Result<Self> {
        let base_url = Url::parse(base_url.as_ref()).context("invalid management URL")?;
        Ok(Self {
            base_url,
            http: reqwest::Client::new(),
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.base_url.join(path)?;
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("GET {path} failed: {text}"));
        }
        Ok(resp.json().await?)
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let resp = self.http.post(url).json(body).send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("POST {path} failed: {text}"));
        }
        Ok(resp.json().await?)
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.base_url.join(path)?;
        let resp = self.http.delete(url).send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("DELETE {path} failed: {text}"));
        }
        Ok(resp.json().await?)
    }

    async fn patch<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let resp = self.http.patch(url).json(body).send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("PATCH {path} failed: {text}"));
        }
        Ok(resp.json().await?)
    }

    /// Fetch version information.
    pub async fn version(&self) -> Result<VersionResponse> {
        self.get("/api/v1/version").await
    }

    /// Fetch cluster status.
    pub async fn cluster_status(&self) -> Result<ClusterStatusResponse> {
        self.get("/api/v1/cluster").await
    }

    /// List nodes.
    pub async fn list_nodes(&self) -> Result<Vec<NodeSummary>> {
        self.get("/api/v1/nodes").await
    }

    /// Get node details.
    pub async fn get_node(&self, name: &str) -> Result<NodeDetailResponse> {
        self.get(&format!("/api/v1/nodes/{name}")).await
    }

    /// Add a node.
    pub async fn add_node(&self, req: &AddNodeRequest) -> Result<OperationResponse> {
        self.post("/api/v1/nodes", req).await
    }

    /// Remove a node.
    pub async fn remove_node(&self, name: &str) -> Result<OperationResponse> {
        self.delete(&format!("/api/v1/nodes/{name}")).await
    }

    /// Update node configuration.
    pub async fn update_node(
        &self,
        name: &str,
        req: &UpdateNodeRequest,
    ) -> Result<OperationResponse> {
        self.patch(&format!("/api/v1/nodes/{name}"), req).await
    }

    /// Enable a node.
    pub async fn enable_node(&self, name: &str) -> Result<OperationResponse> {
        self.post(&format!("/api/v1/nodes/{name}/enable"), &()).await
    }

    /// Disable a node.
    pub async fn disable_node(&self, name: &str) -> Result<OperationResponse> {
        self.post(&format!("/api/v1/nodes/{name}/disable"), &()).await
    }

    /// Drain a node.
    pub async fn drain_node(&self, name: &str) -> Result<OperationResponse> {
        self.post(&format!("/api/v1/nodes/{name}/drain"), &()).await
    }

    /// Probe a node.
    pub async fn probe_node(&self, name: &str) -> Result<serde_json::Value> {
        self.post(&format!("/api/v1/nodes/{name}/probe"), &()).await
    }

    /// List models.
    pub async fn list_models(&self) -> Result<Vec<ModelSummary>> {
        self.get("/api/v1/models").await
    }

    /// Get model details.
    pub async fn get_model(&self, name: &str) -> Result<ModelDetailResponse> {
        self.get(&format!("/api/v1/models/{name}")).await
    }

    /// Sync models across the cluster.
    pub async fn sync_models(&self) -> Result<OperationResponse> {
        self.post("/api/v1/models/sync", &()).await
    }

    /// Explain routing for a model.
    pub async fn explain_model(&self, name: &str) -> Result<ExplainResponse> {
        self.get(&format!("/api/v1/models/{name}/explain")).await
    }

    /// List active requests.
    pub async fn list_requests(&self) -> Result<Vec<RequestSummary>> {
        self.get("/api/v1/requests").await
    }

    /// Cancel a request.
    pub async fn cancel_request(&self, id: &str) -> Result<OperationResponse> {
        self.delete(&format!("/api/v1/requests/{id}")).await
    }

    /// List recent events.
    pub async fn list_events(&self) -> Result<Vec<EventResponse>> {
        self.get("/api/v1/events").await
    }

    /// Show effective configuration.
    pub async fn show_config(&self) -> Result<serde_json::Value> {
        #[derive(serde::Deserialize)]
        struct ConfigWrapper {
            config: serde_json::Value,
        }
        let wrapper: ConfigWrapper = self.get("/api/v1/config").await?;
        Ok(wrapper.config)
    }

    /// Validate configuration.
    pub async fn validate_config(&self) -> Result<OperationResponse> {
        self.post("/api/v1/config/validate", &()).await
    }

    /// Reload configuration.
    pub async fn reload_config(&self) -> Result<OperationResponse> {
        self.post("/api/v1/config/reload", &()).await
    }
}
