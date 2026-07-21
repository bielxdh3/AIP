use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::domain::{AgentPosition, ProvisionalAgent};

const MIGRATION_0001: &str = include_str!("../migrations/0001_phase0.sql");
const OWNER_ID: &str = "usr_owner_local";
const ASTRA_ID: &str = "agt_astra_provisional";
const LUMA_ID: &str = "agt_luma_provisional";

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database unavailable")]
    Unavailable,
    #[error("record not found")]
    NotFound,
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}

impl From<std::io::Error> for DatabaseError {
    fn from(_: std::io::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}

pub struct DatabaseSnapshot {
    pub safe_mode: bool,
    pub migration_version: i64,
    pub agents: Vec<ProvisionalAgent>,
}

impl Database {
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let database = Self { path };
        let mut connection = database.open()?;
        connection.execute_batch(MIGRATION_0001)?;
        Self::seed_phase_zero(&mut connection)?;
        Ok(database)
    }

    fn open(&self) -> Result<Connection, DatabaseError> {
        let connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Ok(connection)
    }

    fn seed_phase_zero(connection: &mut Connection) -> Result<(), DatabaseError> {
        let now = now_millis();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT OR IGNORE INTO users (id, role, display_name, created_at, updated_at)
             VALUES (?1, 'owner', 'Proprietário local', ?2, ?2)",
            params![OWNER_ID, now],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO agents
             (id, owner_user_id, name, profile_key, sprite_key, status, created_at, updated_at)
             VALUES (?1, ?2, 'Astra', 'owner', 'astra', 'active', ?3, ?3)",
            params![ASTRA_ID, OWNER_ID, now],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO agents
             (id, owner_user_id, name, profile_key, sprite_key, status, created_at, updated_at)
             VALUES (?1, ?2, 'Luma', 'companion', 'luma', 'active', ?3, ?3)",
            params![LUMA_ID, OWNER_ID, now],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO agent_screen_preferences
             (agent_id, preferred_x, preferred_y, always_on_top, hide_fullscreen, updated_at)
             VALUES (?1, 80.0, 120.0, 1, 1, ?2)",
            params![ASTRA_ID, now],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO agent_screen_preferences
             (agent_id, preferred_x, preferred_y, always_on_top, hide_fullscreen, updated_at)
             VALUES (?1, 300.0, 160.0, 1, 1, ?2)",
            params![LUMA_ID, now],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO app_settings (key, value_json, updated_at)
             VALUES ('safe_mode', 'false', ?1)",
            params![now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<DatabaseSnapshot, DatabaseError> {
        let connection = self.open()?;
        let safe_mode = connection
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = 'safe_mode'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .is_some_and(|value| value == "true");
        let migration_version = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;

        let mut statement = connection.prepare(
            "SELECT a.id, a.name, a.profile_key, a.sprite_key,
                    p.preferred_x, p.preferred_y
             FROM agents a
             JOIN agent_screen_preferences p ON p.agent_id = a.id
             WHERE a.status = 'active'
             ORDER BY a.profile_key DESC",
        )?;
        let agents = statement
            .query_map([], |row| {
                Ok(ProvisionalAgent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    profile_key: row.get(2)?,
                    sprite_key: row.get(3)?,
                    position: AgentPosition {
                        x: row.get(4)?,
                        y: row.get(5)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DatabaseSnapshot {
            safe_mode,
            migration_version,
            agents,
        })
    }

    pub fn set_safe_mode(&self, enabled: bool) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        connection.execute(
            "INSERT INTO app_settings (key, value_json, updated_at)
             VALUES ('safe_mode', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = excluded.updated_at",
            params![if enabled { "true" } else { "false" }, now_millis()],
        )?;
        Ok(())
    }

    pub fn update_position(&self, agent_id: &str, x: f64, y: f64) -> Result<(), DatabaseError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(DatabaseError::NotFound);
        }
        let connection = self.open()?;
        let changed = connection.execute(
            "UPDATE agent_screen_preferences
             SET preferred_x = ?1, preferred_y = ?2, updated_at = ?3
             WHERE agent_id = ?4",
            params![x, y, now_millis(), agent_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use super::Database;

    fn test_path() -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("aip-test-{}", Uuid::now_v7()))
            .join("aip.sqlite3")
    }

    #[test]
    fn initialization_is_idempotent_and_agents_are_separate() {
        let path = test_path();
        let first = Database::initialize(&path).expect("database should initialize");
        let second = Database::initialize(&path).expect("database should reinitialize");
        let snapshot = second.snapshot().expect("snapshot should load");

        assert_eq!(snapshot.migration_version, 1);
        assert_eq!(snapshot.agents.len(), 2);
        assert_ne!(snapshot.agents[0].id, snapshot.agents[1].id);
        assert_ne!(
            snapshot.agents[0].profile_key,
            snapshot.agents[1].profile_key
        );
        drop(first);
        drop(second);
        let _ = fs::remove_dir_all(path.parent().expect("test path should have a parent"));
    }

    #[test]
    fn position_and_safe_mode_persist_across_reopen() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        let agent_id = database.snapshot().expect("snapshot should load").agents[0]
            .id
            .clone();
        database
            .update_position(&agent_id, 412.0, 216.0)
            .expect("position should update");
        database
            .set_safe_mode(true)
            .expect("safe mode should update");
        drop(database);

        let reopened = Database::initialize(&path).expect("database should reopen");
        let snapshot = reopened.snapshot().expect("snapshot should load");
        let updated = snapshot
            .agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .expect("agent should remain present");
        assert_eq!(updated.position.x, 412.0);
        assert_eq!(updated.position.y, 216.0);
        assert!(snapshot.safe_mode);
        drop(reopened);
        let _ = fs::remove_dir_all(path.parent().expect("test path should have a parent"));
    }
}
