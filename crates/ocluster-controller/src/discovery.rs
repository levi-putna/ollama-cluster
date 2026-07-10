use ocluster_core::{effective_models, fingerprint::ModelInventoryEntry, inventory_fingerprint};
use ocluster_storage::StoredModel;
use reqwest::Client;
use uuid::Uuid;

use crate::runtime::ClusterRuntime;

/// Ollama tags API response fragment.
#[derive(Debug, serde::Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TagModel {
    name: String,
    digest: Option<String>,
    size: Option<u64>,
    modified_at: Option<String>,
    details: Option<TagDetails>,
}

#[derive(Debug, serde::Deserialize)]
struct TagDetails {
    family: Option<String>,
    parameter_size: Option<String>,
    quantization_level: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct PsResponse {
    models: Vec<PsModel>,
}

#[derive(Debug, serde::Deserialize)]
struct PsModel {
    name: String,
}

/// Result of model discovery against a node.
pub struct DiscoveryResult {
    pub ollama_version: Option<String>,
    pub discovered: Vec<String>,
    pub loaded: Vec<String>,
    pub tags: Vec<TagModel>,
    pub fingerprint: String,
}

/// Discover models from an Ollama node URL.
pub async fn discover_node(
    client: &Client,
    url: &str,
    timeout_ms: u64,
) -> Result<DiscoveryResult, String> {
    let base = url.trim_end_matches('/');
    let version: serde_json::Value = client
        .get(format!("{base}/api/version"))
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let tags: TagsResponse = client
        .get(format!("{base}/api/tags"))
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let ps: PsResponse = client
        .get(format!("{base}/api/ps"))
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .unwrap_or(PsResponse { models: vec![] });

    let loaded: Vec<String> = ps.models.into_iter().map(|m| m.name).collect();
    let discovered: Vec<String> = tags.models.iter().map(|m| m.name.clone()).collect();

    let inventory: Vec<ModelInventoryEntry> = tags
        .models
        .iter()
        .map(|m| ModelInventoryEntry {
            name: m.name.clone(),
            digest: m.digest.clone().unwrap_or_default(),
            size: m.size.unwrap_or(0),
            modified_at: m.modified_at.clone().unwrap_or_default(),
        })
        .collect();
    let fingerprint = inventory_fingerprint(&inventory);

    Ok(DiscoveryResult {
        ollama_version: version
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        discovered,
        loaded,
        tags: tags.models,
        fingerprint,
    })
}

/// Apply discovery to runtime and storage.
pub fn apply_discovery_to_runtime(
    runtime: &mut ClusterRuntime,
    node_name: &str,
    result: &DiscoveryResult,
) -> Result<bool, ocluster_storage::StorageError> {
    let node = runtime.nodes.get(node_name).ok_or_else(|| {
        ocluster_storage::StorageError::Migration(format!("node {node_name} missing"))
    })?;

    if node.record.inventory_fingerprint.as_deref() == Some(&result.fingerprint) {
        return Ok(false);
    }

    let node_id = node.record.id.clone();
    let mode = node.record.model_mode;
    let configured = node.record.configured_models.clone();
    let effective = effective_models(mode, &result.discovered, &configured);

    let stored: Vec<StoredModel> = result
        .tags
        .iter()
        .map(|m| {
            let permitted = effective.contains(&m.name);
            StoredModel {
                id: Uuid::new_v4().to_string(),
                node_id: node_id.clone(),
                model_name: m.name.clone(),
                digest: m.digest.clone(),
                size: m.size,
                family: m.details.as_ref().and_then(|d| d.family.clone()),
                parameter_size: m.details.as_ref().and_then(|d| d.parameter_size.clone()),
                quantisation: m
                    .details
                    .as_ref()
                    .and_then(|d| d.quantization_level.clone()),
                modified_at: m.modified_at.clone(),
                discovered: true,
                configured: configured.contains(&m.name),
                permitted,
                available: permitted,
                loaded: result.loaded.contains(&m.name),
            }
        })
        .collect();

    if let Some(node) = runtime.nodes.get_mut(node_name) {
        node.record.ollama_version = result.ollama_version.clone();
    }

    runtime.apply_discovery(
        node_name,
        result.discovered.clone(),
        result.loaded.clone(),
        stored,
        Some(result.fingerprint.clone()),
    )?;
    runtime.storage.append_event(
        "model_discovered",
        Some(node_name),
        &format!("discovered {} models", result.discovered.len()),
    )?;
    Ok(true)
}

/// Sync all nodes in the cluster.
pub async fn sync_all_nodes(
    runtime: &mut ClusterRuntime,
    client: &Client,
) -> Result<usize, String> {
    let names: Vec<String> = runtime.nodes.keys().cloned().collect();
    let timeout = runtime.config.discovery.timeout_ms;
    let mut updated = 0;
    for name in names {
        let url = runtime.nodes.get(&name).unwrap().record.url.clone();
        let result = discover_node(client, &url, timeout).await?;
        if apply_discovery_to_runtime(runtime, &name, &result).unwrap_or(false) {
            updated += 1;
        }
    }
    Ok(updated)
}
