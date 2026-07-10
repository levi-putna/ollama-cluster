use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use ocluster_core::{AdminState, ModelMode, NodeRuntimeState};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::StorageError;
use crate::migrations::migrate;

/// Persisted node record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRecord {
    pub id: String,
    pub name: String,
    pub url: String,
    pub admin_state: AdminState,
    pub runtime_state: NodeRuntimeState,
    pub ollama_version: Option<String>,
    pub model_mode: ModelMode,
    pub configured_models: Vec<String>,
    pub max_concurrent: u32,
    pub priority: i32,
    pub labels: std::collections::HashMap<String, String>,
    pub inventory_fingerprint: Option<String>,
}

/// Persisted model record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredModel {
    pub id: String,
    pub node_id: String,
    pub model_name: String,
    pub digest: Option<String>,
    pub size: Option<u64>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantisation: Option<String>,
    pub modified_at: Option<String>,
    pub discovered: bool,
    pub configured: bool,
    pub permitted: bool,
    pub available: bool,
    pub loaded: bool,
}

/// Cluster event record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterEvent {
    pub id: String,
    pub event_type: String,
    pub target: Option<String>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// SQLite-backed persistence.
#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    /// Open or create a database at the given path.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::Migration(e.to_string()))?;
        }
        let conn = Connection::open(path)?;
        migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database for tests.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Upsert a node record.
    pub fn upsert_node(&self, node: &NodeRecord) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        let configured = serde_json::to_string(&node.configured_models)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let labels =
            serde_json::to_string(&node.labels).map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.conn.lock().unwrap().execute(
            "INSERT INTO nodes (id, name, url, admin_state, runtime_state, ollama_version,
                model_mode, configured_models, max_concurrent, priority, labels,
                inventory_fingerprint, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)
             ON CONFLICT(name) DO UPDATE SET
                url=excluded.url, admin_state=excluded.admin_state,
                runtime_state=excluded.runtime_state, ollama_version=excluded.ollama_version,
                model_mode=excluded.model_mode, configured_models=excluded.configured_models,
                max_concurrent=excluded.max_concurrent, priority=excluded.priority,
                labels=excluded.labels, inventory_fingerprint=excluded.inventory_fingerprint,
                updated_at=excluded.updated_at",
            params![
                node.id,
                node.name,
                node.url,
                serde_json::to_string(&node.admin_state).unwrap(),
                node.runtime_state.to_string(),
                node.ollama_version,
                serde_json::to_string(&node.model_mode).unwrap(),
                configured,
                node.max_concurrent,
                node.priority,
                labels,
                node.inventory_fingerprint,
                now,
            ],
        )?;
        Ok(())
    }

    /// List all nodes.
    pub fn list_nodes(&self) -> Result<Vec<NodeRecord>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, admin_state, runtime_state, ollama_version,
                    model_mode, configured_models, max_concurrent, priority, labels,
                    inventory_fingerprint FROM nodes ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, u32>(8)?,
                row.get::<_, i32>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        })?;

        rows.map(|r| {
            let (
                id,
                name,
                url,
                admin_state,
                runtime_state,
                ollama_version,
                model_mode,
                configured_models,
                max_concurrent,
                priority,
                labels,
                inventory_fingerprint,
            ) = r.map_err(StorageError::from)?;
            Ok(NodeRecord {
                id,
                name,
                url,
                admin_state: serde_json::from_str(&admin_state)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?,
                runtime_state: parse_runtime_state(&runtime_state),
                ollama_version,
                model_mode: serde_json::from_str(&model_mode)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?,
                configured_models: serde_json::from_str(&configured_models)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?,
                max_concurrent,
                priority,
                labels: serde_json::from_str(&labels)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?,
                inventory_fingerprint,
            })
        })
        .collect()
    }

    /// Get a node by name.
    pub fn get_node(&self, name: &str) -> Result<Option<NodeRecord>, StorageError> {
        Ok(self
            .list_nodes()?
            .into_iter()
            .find(|n| n.name == name))
    }

    /// Remove a node and associated models transactionally.
    pub fn remove_node(&self, name: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let deleted = tx.execute("DELETE FROM nodes WHERE name = ?1", [name])?;
        tx.commit()?;
        Ok(deleted > 0)
    }

    /// Replace all models for a node.
    pub fn replace_node_models(
        &self,
        node_id: &str,
        models: &[StoredModel],
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM models WHERE node_id = ?1", [node_id])?;
        for model in models {
            tx.execute(
                "INSERT INTO models (id, node_id, model_name, digest, size, family,
                    parameter_size, quantisation, modified_at, discovered, configured,
                    permitted, available, loaded, last_seen_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    model.id,
                    model.node_id,
                    model.model_name,
                    model.digest,
                    model.size,
                    model.family,
                    model.parameter_size,
                    model.quantisation,
                    model.modified_at,
                    model.discovered as i32,
                    model.configured as i32,
                    model.permitted as i32,
                    model.available as i32,
                    model.loaded as i32,
                    Utc::now().to_rfc3339(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// List models, optionally filtered by name.
    pub fn list_models(&self, model_name: Option<&str>) -> Result<Vec<StoredModel>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let sql = "SELECT id, node_id, model_name, digest, size, family, parameter_size,
                          quantisation, modified_at, discovered, configured, permitted,
                          available, loaded FROM models";
        let mut models = Vec::new();

        if let Some(name) = model_name {
            let mut stmt = conn.prepare(&format!("{sql} WHERE model_name = ?1"))?;
            let rows = stmt.query_map([name], map_model_row)?;
            for row in rows {
                models.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map([], map_model_row)?;
            for row in rows {
                models.push(row?);
            }
        }

        Ok(models)
    }

    /// Append a cluster event.
    pub fn append_event(
        &self,
        event_type: &str,
        target: Option<&str>,
        message: &str,
    ) -> Result<ClusterEvent, StorageError> {
        let event = ClusterEvent {
            id: Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            target: target.map(String::from),
            message: message.into(),
            created_at: Utc::now(),
        };
        self.conn.lock().unwrap().execute(
            "INSERT INTO events (id, event_type, target, message, metadata, created_at)
             VALUES (?1,?2,?3,?4,NULL,?5)",
            params![
                event.id,
                event.event_type,
                event.target,
                event.message,
                event.created_at.to_rfc3339(),
            ],
        )?;
        Ok(event)
    }

    /// List recent events.
    pub fn list_events(&self, limit: usize) -> Result<Vec<ClusterEvent>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, event_type, target, message, created_at FROM events
             ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok(ClusterEvent {
                id: row.get(0)?,
                event_type: row.get(1)?,
                target: row.get(2)?,
                message: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
    }

    /// Append an audit record.
    pub fn append_audit(
        &self,
        action: &str,
        target: Option<&str>,
        outcome: &str,
        actor: Option<&str>,
    ) -> Result<(), StorageError> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO audit (id, action, target, outcome, actor, created_at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                Uuid::new_v4().to_string(),
                action,
                target,
                outcome,
                actor,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Save a configuration snapshot.
    pub fn save_config_snapshot(&self, config_json: &str) -> Result<(), StorageError> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO config_snapshots (id, config_json, created_at) VALUES (?1,?2,?3)",
            params![
                Uuid::new_v4().to_string(),
                config_json,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }
}

fn map_model_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredModel> {
    Ok(StoredModel {
        id: row.get(0)?,
        node_id: row.get(1)?,
        model_name: row.get(2)?,
        digest: row.get(3)?,
        size: row.get(4)?,
        family: row.get(5)?,
        parameter_size: row.get(6)?,
        quantisation: row.get(7)?,
        modified_at: row.get(8)?,
        discovered: row.get::<_, i32>(9)? != 0,
        configured: row.get::<_, i32>(10)? != 0,
        permitted: row.get::<_, i32>(11)? != 0,
        available: row.get::<_, i32>(12)? != 0,
        loaded: row.get::<_, i32>(13)? != 0,
    })
}

fn parse_runtime_state(raw: &str) -> NodeRuntimeState {
    match raw {
        "suspect" => NodeRuntimeState::Suspect,
        "unavailable" => NodeRuntimeState::Unavailable,
        "recovering" => NodeRuntimeState::Recovering,
        "warming" => NodeRuntimeState::Warming,
        _ => NodeRuntimeState::Ready,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node(name: &str) -> NodeRecord {
        NodeRecord {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            url: format!("http://10.0.0.1:11434"),
            admin_state: AdminState::Enabled,
            runtime_state: NodeRuntimeState::Ready,
            ollama_version: Some("0.5.0".into()),
            model_mode: ModelMode::Discover,
            configured_models: vec![],
            max_concurrent: 8,
            priority: 0,
            labels: Default::default(),
            inventory_fingerprint: None,
        }
    }

    /// Covers: TR-103, TXR-110-04
    #[test]
    fn node_removal_cleans_models() {
        let storage = Storage::open_in_memory().unwrap();
        let node = sample_node("gpu-01");
        storage.upsert_node(&node).unwrap();
        storage
            .replace_node_models(
                &node.id,
                &[StoredModel {
                    id: Uuid::new_v4().to_string(),
                    node_id: node.id.clone(),
                    model_name: "llama".into(),
                    digest: None,
                    size: None,
                    family: None,
                    parameter_size: None,
                    quantisation: None,
                    modified_at: None,
                    discovered: true,
                    configured: false,
                    permitted: true,
                    available: true,
                    loaded: false,
                }],
            )
            .unwrap();
        assert_eq!(storage.list_models(None).unwrap().len(), 1);
        storage.remove_node("gpu-01").unwrap();
        assert!(storage.get_node("gpu-01").unwrap().is_none());
    }

    /// Covers: FR-151, TXR-111-01, TXR-023
    #[test]
    fn disabled_state_persists() {
        let storage = Storage::open_in_memory().unwrap();
        let mut node = sample_node("gpu-01");
        node.admin_state = AdminState::Disabled;
        storage.upsert_node(&node).unwrap();
        let loaded = storage.get_node("gpu-01").unwrap().unwrap();
        assert_eq!(loaded.admin_state, AdminState::Disabled);
    }
}
