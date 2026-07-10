use rusqlite::Connection;

use crate::error::StorageError;

const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Apply database migrations.
pub fn migrate(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );",
    )?;

    let version: i32 = conn
        .query_row(
            "SELECT version FROM schema_version LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version == 0 {
        apply_v1(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [CURRENT_SCHEMA_VERSION])?;
    } else if version != CURRENT_SCHEMA_VERSION {
        return Err(StorageError::Migration(format!(
            "unsupported schema version {version}"
        )));
    }

    Ok(())
}

fn apply_v1(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(
        "
        CREATE TABLE nodes (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            url TEXT NOT NULL,
            admin_state TEXT NOT NULL,
            runtime_state TEXT NOT NULL,
            ollama_version TEXT,
            model_mode TEXT NOT NULL,
            configured_models TEXT NOT NULL,
            max_concurrent INTEGER NOT NULL,
            priority INTEGER NOT NULL,
            labels TEXT NOT NULL,
            inventory_fingerprint TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE models (
            id TEXT PRIMARY KEY NOT NULL,
            node_id TEXT NOT NULL,
            model_name TEXT NOT NULL,
            digest TEXT,
            size INTEGER,
            family TEXT,
            parameter_size TEXT,
            quantisation TEXT,
            modified_at TEXT,
            discovered INTEGER NOT NULL,
            configured INTEGER NOT NULL,
            permitted INTEGER NOT NULL,
            available INTEGER NOT NULL,
            loaded INTEGER NOT NULL,
            last_seen_at TEXT NOT NULL,
            FOREIGN KEY(node_id) REFERENCES nodes(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_models_node ON models(node_id);
        CREATE INDEX idx_models_name ON models(model_name);

        CREATE TABLE events (
            id TEXT PRIMARY KEY NOT NULL,
            event_type TEXT NOT NULL,
            target TEXT,
            message TEXT NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE audit (
            id TEXT PRIMARY KEY NOT NULL,
            action TEXT NOT NULL,
            target TEXT,
            outcome TEXT NOT NULL,
            actor TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE config_snapshots (
            id TEXT PRIMARY KEY NOT NULL,
            config_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Covers: TR-102, TXR-023
    #[test]
    fn migrations_apply_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let version: i32 = conn
            .query_row("SELECT version FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }
}
