use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    can_transition_message, AgentMemory, AgentPosition, AgentSimulatedState, ConversationMessage,
    MessageAuthor, MessageStatus, PhaseOneConversation, ProvisionalAgent,
    DEFAULT_KEEP_ALIVE_MINUTES, MAX_KEEP_ALIVE_MINUTES, MAX_USER_MESSAGE_BYTES,
};

const MIGRATION_0001: &str = include_str!("../migrations/0001_phase0.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_phase1_conversations.sql");
const MIGRATION_0003: &str = include_str!("../migrations/0003_phase1_agent_settings.sql");
const MIGRATION_0004: &str = include_str!("../migrations/0004_phase2_identity.sql");
const MIGRATION_0005: &str = include_str!("../migrations/0005_phase3_conversations_memory.sql");
const MIGRATION_0006: &str = include_str!("../migrations/0006_phase4_agent_state.sql");
const MIGRATION_0007: &str = include_str!("../migrations/0007_phase5_pixel_documents.sql");
const MIGRATIONS: [(i64, &str); 7] = [
    (1, MIGRATION_0001),
    (2, MIGRATION_0002),
    (3, MIGRATION_0003),
    (4, MIGRATION_0004),
    (5, MIGRATION_0005),
    (6, MIGRATION_0006),
    (7, MIGRATION_0007),
];
pub const OWNER_ID: &str = "usr_owner_local";
pub const ASTRA_ID: &str = "agt_astra_provisional";
pub const LUMA_ID: &str = "agt_luma_provisional";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DatabaseError {
    #[error("database unavailable")]
    Unavailable,
    #[error("record not found")]
    NotFound,
    #[error("record does not belong to agent")]
    OwnershipMismatch,
    #[error("invalid value")]
    InvalidValue,
    #[error("invalid state transition")]
    InvalidTransition,
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
    pub onboarding_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseOneSettings {
    pub selected_model_ref: Option<String>,
    pub keep_alive_minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAttempt {
    pub request_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMessage {
    pub author: MessageAuthor,
    pub content: String,
}

impl Database {
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let database = Self { path };
        let mut connection = database.open()?;
        Self::apply_migrations(&mut connection)?;
        Self::seed_phase_zero(&mut connection)?;
        Self::seed_phase_one(&mut connection)?;
        Self::recover_interrupted(&connection)?;
        Ok(database)
    }

    fn open(&self) -> Result<Connection, DatabaseError> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Ok(connection)
    }

    fn apply_migrations(connection: &mut Connection) -> Result<(), DatabaseError> {
        let migration_table_exists = connection.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'schema_migrations'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )?;
        let current_version = if migration_table_exists {
            connection.query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get::<_, i64>(0),
            )?
        } else {
            0
        };

        for (version, sql) in MIGRATIONS {
            if version > current_version {
                connection.execute_batch(sql)?;
            }
        }
        Ok(())
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

    fn seed_phase_one(connection: &mut Connection) -> Result<(), DatabaseError> {
        let now = now_millis();
        let transaction = connection.transaction()?;
        for agent_id in [ASTRA_ID, LUMA_ID] {
            transaction.execute(
                "INSERT OR IGNORE INTO conversations
                 (id, agent_id, owner_user_id, title, kind, is_main, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'Conversa principal', 'normal', 1, ?4, ?4)",
                params![Uuid::now_v7().to_string(), agent_id, OWNER_ID, now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO agent_identity_profiles
                 (agent_id, birthday, fictive_age, age_category, species, pronouns, personality_summary, traits_json, appearance_preset, created_at, updated_at)
                 VALUES (?1, '2000-01-01', 18, 'adult', 'agent', 'they/them', '', '{}', ?2, ?3, ?3)",
                params![agent_id, if agent_id == ASTRA_ID { "astra" } else { "luma" }, now],
            )?;
        }
        transaction.execute(
            "INSERT OR IGNORE INTO app_settings (key, value_json, updated_at)
             VALUES ('phase1_keep_alive_minutes', ?1, ?2)",
            params![DEFAULT_KEEP_ALIVE_MINUTES.to_string(), now],
        )?;
        for agent_id in [ASTRA_ID, LUMA_ID] {
            transaction.execute(
                "INSERT OR IGNORE INTO agent_phase1_settings
                 (agent_id, selected_model_ref, keep_alive_minutes, updated_at)
                 VALUES (?1, NULL, ?2, ?3)",
                params![agent_id, DEFAULT_KEEP_ALIVE_MINUTES, now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO agent_phase3_settings (agent_id, active_conversation_id, updated_at)
                 SELECT ?1, id, ?2 FROM conversations
                 WHERE agent_id = ?1 AND is_main = 1 AND archived_at IS NULL",
                params![agent_id, now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO agent_simulated_states
                 (agent_id, sleep, energy, mood, focus, curiosity, social_fatigue, mode, suspended, last_simulated_at, updated_at)
                 VALUES (?1, 20, 80, 70, 70, 70, 20, 'normal', 0, ?2, ?2)",
                params![agent_id, now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO pixel_documents (id, agent_id, owner_user_id, schema_version, width, height, source_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 1, 64, 64, '{\"layers\":[{\"id\":\"body\",\"name\":\"Body\",\"visible\":true,\"locked\":false,\"pixels\":[]}],\"attachmentPoints\":{}}', ?4, ?4)",
                params![Uuid::now_v7().to_string(), agent_id, OWNER_ID, now],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn recover_interrupted(connection: &Connection) -> Result<(), DatabaseError> {
        connection.execute(
            "UPDATE conversation_messages
             SET status = 'failed', completed_at = ?1,
                 terminal_error_code = 'runtime_interrupted'
             WHERE author_type = 'agent' AND status IN ('pending', 'streaming')",
            params![now_millis()],
        )?;
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
        let onboarding_required = connection
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = 'phase2_onboarding_complete'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .is_none_or(|value| value != "true");
        let migration_version = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;

        let mut statement = connection.prepare(
            "SELECT a.id, a.name, a.profile_key, a.sprite_key,
                    p.preferred_x, p.preferred_y, i.birthday, i.fictive_age,
                    i.age_category, i.species, i.pronouns, i.personality_summary,
                    i.traits_json, i.appearance_preset
             FROM agents a
             JOIN agent_screen_preferences p ON p.agent_id = a.id
             JOIN agent_identity_profiles i ON i.agent_id = a.id
             WHERE a.status = 'active'
             ORDER BY a.profile_key DESC",
        )?;
        let agents = statement
            .query_map([], map_agent)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DatabaseSnapshot {
            safe_mode,
            migration_version,
            agents,
            onboarding_required,
        })
    }

    pub fn agent(&self, agent_id: &str) -> Result<ProvisionalAgent, DatabaseError> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT a.id, a.name, a.profile_key, a.sprite_key,
                        p.preferred_x, p.preferred_y, i.birthday, i.fictive_age,
                        i.age_category, i.species, i.pronouns, i.personality_summary,
                        i.traits_json, i.appearance_preset
                 FROM agents a
                 JOIN agent_screen_preferences p ON p.agent_id = a.id
                 JOIN agent_identity_profiles i ON i.agent_id = a.id
                 WHERE a.id = ?1 AND a.status = 'active'",
                params![agent_id],
                map_agent,
            )
            .optional()?
            .ok_or(DatabaseError::NotFound)
    }

    pub fn main_conversation(&self, agent_id: &str) -> Result<PhaseOneConversation, DatabaseError> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT id, agent_id, title, model_override_ref FROM conversations
                 WHERE agent_id = ?1 AND is_main = 1 AND archived_at IS NULL",
                params![agent_id],
                |row| {
                    Ok(PhaseOneConversation {
                        id: row.get(0)?,
                        agent_id: row.get(1)?,
                        title: row.get(2)?,
                        model_override_ref: row.get(3)?,
                    })
                },
            )
            .optional()?
            .ok_or(DatabaseError::NotFound)
    }

    pub fn conversations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<PhaseOneConversation>, DatabaseError> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id, agent_id, title, model_override_ref FROM conversations
             WHERE agent_id = ?1 AND archived_at IS NULL ORDER BY is_main DESC, updated_at DESC, id ASC",
        )?;
        let conversations = statement
            .query_map(params![agent_id], |row| {
                Ok(PhaseOneConversation {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    title: row.get(2)?,
                    model_override_ref: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(conversations)
    }

    pub fn create_conversation(
        &self,
        agent_id: &str,
        title: &str,
    ) -> Result<PhaseOneConversation, DatabaseError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 160 {
            return Err(DatabaseError::InvalidValue);
        }
        self.agent(agent_id)?;
        let conversation = PhaseOneConversation {
            id: Uuid::now_v7().to_string(),
            agent_id: agent_id.into(),
            title: title.into(),
            model_override_ref: None,
        };
        let connection = self.open()?;
        let now = now_millis();
        connection.execute("INSERT INTO conversations (id, agent_id, owner_user_id, title, kind, is_main, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 'normal', 0, ?5, ?5)", params![conversation.id, agent_id, OWNER_ID, title, now])?;
        Ok(conversation)
    }

    pub fn rename_conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
        title: &str,
    ) -> Result<(), DatabaseError> {
        let title = title.trim();
        if title.is_empty() || title.len() > 160 {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute("UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3 AND agent_id = ?4 AND archived_at IS NULL", params![title, now_millis(), conversation_id, agent_id])? == 1 { Ok(()) } else { Err(DatabaseError::OwnershipMismatch) }
    }

    pub fn set_active_conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<(), DatabaseError> {
        self.verify_conversation(agent_id, conversation_id)?;
        let connection = self.open()?;
        connection.execute("UPDATE agent_phase3_settings SET active_conversation_id = ?1, updated_at = ?2 WHERE agent_id = ?3", params![conversation_id, now_millis(), agent_id])?;
        Ok(())
    }

    pub fn active_conversation(
        &self,
        agent_id: &str,
    ) -> Result<PhaseOneConversation, DatabaseError> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT c.id, c.agent_id, c.title, c.model_override_ref
             FROM agent_phase3_settings s JOIN conversations c ON c.id = s.active_conversation_id
             WHERE s.agent_id = ?1 AND c.agent_id = ?1 AND c.archived_at IS NULL",
                params![agent_id],
                |row| {
                    Ok(PhaseOneConversation {
                        id: row.get(0)?,
                        agent_id: row.get(1)?,
                        title: row.get(2)?,
                        model_override_ref: row.get(3)?,
                    })
                },
            )
            .optional()?
            .map_or_else(|| self.main_conversation(agent_id), Ok)
    }

    pub fn conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<PhaseOneConversation, DatabaseError> {
        let connection = self.open()?;
        connection.query_row("SELECT id, agent_id, title, model_override_ref FROM conversations WHERE id = ?1 AND agent_id = ?2 AND archived_at IS NULL", params![conversation_id, agent_id], |row| Ok(PhaseOneConversation { id: row.get(0)?, agent_id: row.get(1)?, title: row.get(2)?, model_override_ref: row.get(3)? })).optional()?.ok_or(DatabaseError::OwnershipMismatch)
    }

    pub fn archive_conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        if connection.execute("UPDATE conversations SET archived_at = ?1, updated_at = ?1 WHERE id = ?2 AND agent_id = ?3 AND is_main = 0 AND archived_at IS NULL", params![now_millis(), conversation_id, agent_id])? == 1 { Ok(()) } else { Err(DatabaseError::OwnershipMismatch) }
    }

    pub fn restore_conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        if connection.execute("UPDATE conversations SET archived_at = NULL, updated_at = ?1 WHERE id = ?2 AND agent_id = ?3 AND is_main = 0 AND archived_at IS NOT NULL", params![now_millis(), conversation_id, agent_id])? == 1 { Ok(()) } else { Err(DatabaseError::OwnershipMismatch) }
    }

    pub fn memories(&self, agent_id: &str) -> Result<Vec<AgentMemory>, DatabaseError> {
        self.agent(agent_id)?;
        let connection = self.open()?;
        let mut statement = connection.prepare("SELECT id, agent_id, category, content, status, confirmation_status, confidence, importance, source_type, source_message_id, source_conversation_id, conflict_key, created_at, updated_at FROM agent_memories WHERE agent_id = ?1 ORDER BY updated_at DESC, id ASC")?;
        let memories = statement
            .query_map(params![agent_id], map_memory)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(memories)
    }

    pub fn simulated_state(&self, agent_id: &str) -> Result<AgentSimulatedState, DatabaseError> {
        self.advance_simulated_state(agent_id, now_millis())?;
        let connection = self.open()?;
        connection.query_row("SELECT agent_id, sleep, energy, mood, focus, curiosity, social_fatigue, mode, suspended, wake_now_until, last_simulated_at FROM agent_simulated_states WHERE agent_id = ?1", params![agent_id], map_simulated_state).optional()?.ok_or(DatabaseError::NotFound)
    }

    pub fn advance_simulated_state(&self, agent_id: &str, now: i64) -> Result<(), DatabaseError> {
        let current = {
            let connection = self.open()?;
            connection
                .query_row("SELECT agent_id, sleep, energy, mood, focus, curiosity, social_fatigue, mode, suspended, wake_now_until, last_simulated_at FROM agent_simulated_states WHERE agent_id = ?1", params![agent_id], map_simulated_state)
                .optional()?
                .ok_or(DatabaseError::NotFound)?
        };
        if current.suspended || now <= current.last_simulated_at {
            return Ok(());
        }
        let elapsed_minutes = ((now - current.last_simulated_at) / 60_000).clamp(0, 240) as u8;
        if elapsed_minutes == 0 {
            return Ok(());
        }
        let protected_by_wake = current.wake_now_until.is_some_and(|until| now < until);
        let sleep = if protected_by_wake {
            0
        } else {
            current.sleep.saturating_sub(elapsed_minutes)
        };
        let energy = if sleep > 0 {
            current.energy.saturating_add(elapsed_minutes / 2).min(100)
        } else {
            current.energy.saturating_sub((elapsed_minutes / 8).max(1))
        };
        let social_fatigue = current
            .social_fatigue
            .saturating_sub((elapsed_minutes / 4).max(1));
        let connection = self.open()?;
        connection.execute(
            "UPDATE agent_simulated_states SET sleep = ?1, energy = ?2, social_fatigue = ?3, wake_now_until = CASE WHEN wake_now_until <= ?4 THEN NULL ELSE wake_now_until END, last_simulated_at = ?4, updated_at = ?4 WHERE agent_id = ?5",
            params![sleep, energy, social_fatigue, now, agent_id],
        )?;
        Ok(())
    }

    pub fn pixel_document(&self, agent_id: &str) -> Result<String, DatabaseError> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT source_json FROM pixel_documents WHERE agent_id = ?1",
                params![agent_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(DatabaseError::NotFound)
    }

    pub fn save_pixel_document(
        &self,
        agent_id: &str,
        source_json: &str,
    ) -> Result<(), DatabaseError> {
        let valid = serde_json::from_str::<serde_json::Value>(source_json).is_ok_and(|document| {
            document
                .get("layers")
                .is_some_and(serde_json::Value::is_array)
                && document
                    .get("attachmentPoints")
                    .is_some_and(serde_json::Value::is_object)
        });
        if !valid || source_json.len() > 1_000_000 {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute(
            "UPDATE pixel_documents SET source_json = ?1, updated_at = ?2 WHERE agent_id = ?3",
            params![source_json, now_millis(), agent_id],
        )? == 1
        {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    pub fn set_agent_mode(&self, agent_id: &str, mode: &str) -> Result<(), DatabaseError> {
        if !matches!(mode, "normal" | "voice_muted" | "silent" | "safe") {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute(
            "UPDATE agent_simulated_states SET mode = ?1, updated_at = ?2 WHERE agent_id = ?3",
            params![mode, now_millis(), agent_id],
        )? == 1
        {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    pub fn set_agent_suspended(
        &self,
        agent_id: &str,
        suspended: bool,
    ) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        if connection.execute(
            "UPDATE agent_simulated_states SET suspended = ?1, updated_at = ?2 WHERE agent_id = ?3",
            params![suspended, now_millis(), agent_id],
        )? == 1
        {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    pub fn wake_agent_now(&self, agent_id: &str, until: i64) -> Result<(), DatabaseError> {
        if until <= now_millis() {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute("UPDATE agent_simulated_states SET sleep = 0, energy = MIN(100, energy + 20), wake_now_until = ?1, updated_at = ?2 WHERE agent_id = ?3", params![until, now_millis(), agent_id])? == 1 { Ok(()) } else { Err(DatabaseError::NotFound) }
    }

    pub fn create_memory(
        &self,
        agent_id: &str,
        category: &str,
        content: &str,
        confirmed: bool,
    ) -> Result<AgentMemory, DatabaseError> {
        let category = category.trim();
        let content = content.trim();
        if category.is_empty() || category.len() > 64 || content.is_empty() || content.len() > 4_000
        {
            return Err(DatabaseError::InvalidValue);
        }
        self.agent(agent_id)?;
        let now = now_millis();
        let memory = AgentMemory {
            id: Uuid::now_v7().to_string(),
            agent_id: agent_id.into(),
            category: category.into(),
            content: content.into(),
            status: "active".into(),
            confirmation_status: if confirmed { "confirmed" } else { "pending" }.into(),
            confidence_milli: if confirmed { 1000 } else { 500 },
            importance: 50,
            source_type: "manual".into(),
            source_message_id: None,
            source_conversation_id: None,
            conflict_key: None,
            created_at: now,
            updated_at: now,
        };
        let connection = self.open()?;
        connection.execute("INSERT INTO agent_memories (id, agent_id, owner_user_id, category, content, status, confirmation_status, confidence, importance, source_type, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)", params![memory.id, agent_id, OWNER_ID, memory.category, memory.content, memory.status, memory.confirmation_status, f64::from(memory.confidence_milli) / 1000.0, memory.importance, memory.source_type, now])?;
        Ok(memory)
    }

    pub fn set_memory_status(
        &self,
        agent_id: &str,
        memory_id: &str,
        status: &str,
    ) -> Result<(), DatabaseError> {
        if !matches!(
            status,
            "active" | "archived" | "trashed" | "candidate_rejected"
        ) {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute("UPDATE agent_memories SET status = ?1, archived_at = CASE WHEN ?1 = 'archived' THEN ?2 ELSE NULL END, trashed_at = CASE WHEN ?1 = 'trashed' THEN ?2 ELSE NULL END, updated_at = ?2 WHERE id = ?3 AND agent_id = ?4", params![status, now_millis(), memory_id, agent_id])? == 1 { Ok(()) } else { Err(DatabaseError::OwnershipMismatch) }
    }

    pub fn messages(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<Vec<ConversationMessage>, DatabaseError> {
        self.verify_conversation(agent_id, conversation_id)?;
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id, conversation_id, agent_id, author_type, content,
                    actual_model_ref, status, created_at, completed_at, terminal_error_code
             FROM conversation_messages
             WHERE conversation_id = ?1 AND agent_id = ?2
             ORDER BY created_at ASC, id ASC",
        )?;
        let messages = statement
            .query_map(params![conversation_id, agent_id], map_message)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(messages)
    }

    fn confirmed_memory_context(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<ContextMessage>, DatabaseError> {
        let connection = self.open()?;
        let mut statement = connection.prepare("SELECT content FROM agent_memories WHERE agent_id = ?1 AND status = 'active' AND confirmation_status = 'confirmed' ORDER BY importance DESC, updated_at DESC, id ASC LIMIT ?2")?;
        let memories = statement
            .query_map(params![agent_id, limit as i64], |row| {
                Ok(ContextMessage {
                    author: MessageAuthor::System,
                    content: format!("Memória confirmada: {}", row.get::<_, String>(0)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(memories)
    }

    pub fn settings(&self, agent_id: &str) -> Result<PhaseOneSettings, DatabaseError> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT selected_model_ref, keep_alive_minutes
                 FROM agent_phase1_settings WHERE agent_id = ?1",
                params![agent_id],
                |row| {
                    let keep_alive_minutes = row.get::<_, u32>(1)?;
                    Ok(PhaseOneSettings {
                        selected_model_ref: row
                            .get::<_, Option<String>>(0)?
                            .filter(|value| valid_model_ref(value)),
                        keep_alive_minutes: if keep_alive_minutes <= MAX_KEEP_ALIVE_MINUTES {
                            keep_alive_minutes
                        } else {
                            DEFAULT_KEEP_ALIVE_MINUTES
                        },
                    })
                },
            )
            .optional()?
            .ok_or(DatabaseError::NotFound)
    }

    pub fn set_selected_model(&self, agent_id: &str, model_ref: &str) -> Result<(), DatabaseError> {
        if !valid_model_ref(model_ref) {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute(
            "UPDATE agent_phase1_settings
             SET selected_model_ref = ?1, updated_at = ?2 WHERE agent_id = ?3",
            params![model_ref, now_millis(), agent_id],
        )? == 1
        {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    pub fn set_keep_alive(&self, agent_id: &str, minutes: u32) -> Result<(), DatabaseError> {
        if minutes > MAX_KEEP_ALIVE_MINUTES {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        if connection.execute(
            "UPDATE agent_phase1_settings
             SET keep_alive_minutes = ?1, updated_at = ?2 WHERE agent_id = ?3",
            params![minutes, now_millis(), agent_id],
        )? == 1
        {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    pub fn update_profile(&self, agent: &ProvisionalAgent) -> Result<(), DatabaseError> {
        validate_profile(agent)?;
        let connection = self.open()?;
        let now = now_millis();
        let changed = connection.execute(
            "UPDATE agents SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![agent.name.trim(), now, agent.id],
        )?;
        if changed != 1 {
            return Err(DatabaseError::NotFound);
        }
        connection.execute(
            "UPDATE agent_identity_profiles SET birthday = ?1, fictive_age = ?2,
             age_category = ?3, species = ?4, pronouns = ?5, personality_summary = ?6,
             traits_json = ?7, appearance_preset = ?8, updated_at = ?9 WHERE agent_id = ?10",
            params![
                agent.birthday,
                agent.fictive_age,
                agent.age_category,
                agent.species,
                agent.pronouns,
                agent.personality_summary,
                agent.traits_json,
                agent.appearance_preset,
                now,
                agent.id
            ],
        )?;
        Ok(())
    }

    pub fn complete_onboarding(&self, agents: &[ProvisionalAgent]) -> Result<(), DatabaseError> {
        let ids = agents
            .iter()
            .map(|agent| agent.id.as_str())
            .collect::<HashSet<_>>();
        if agents.len() != 2
            || ids.len() != 2
            || ids != HashSet::from([ASTRA_ID, LUMA_ID])
            || agents.iter().any(|agent| validate_profile(agent).is_err())
        {
            return Err(DatabaseError::InvalidValue);
        }
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let now = now_millis();
        for agent in agents {
            let changed = transaction.execute(
                "UPDATE agents SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![agent.name.trim(), now, agent.id],
            )?;
            if changed != 1 {
                return Err(DatabaseError::NotFound);
            }
            transaction.execute(
                "UPDATE agent_identity_profiles SET birthday = ?1, fictive_age = ?2,
                 age_category = ?3, species = ?4, pronouns = ?5, personality_summary = ?6,
                 traits_json = ?7, appearance_preset = ?8, updated_at = ?9 WHERE agent_id = ?10",
                params![
                    agent.birthday,
                    agent.fictive_age,
                    agent.age_category,
                    agent.species,
                    agent.pronouns,
                    agent.personality_summary,
                    agent.traits_json,
                    agent.appearance_preset,
                    now,
                    agent.id
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO app_settings (key, value_json, updated_at)
             VALUES ('phase2_onboarding_complete', 'true', ?1)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_main_conversation_override(
        &self,
        agent_id: &str,
        model_ref: Option<&str>,
    ) -> Result<(), DatabaseError> {
        if model_ref.is_some_and(|model| !valid_model_ref(model)) {
            return Err(DatabaseError::InvalidValue);
        }
        let connection = self.open()?;
        let changed = connection.execute(
            "UPDATE conversations SET model_override_ref = ?1, updated_at = ?2
             WHERE agent_id = ?3 AND is_main = 1 AND archived_at IS NULL",
            params![model_ref, now_millis(), agent_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(DatabaseError::NotFound)
        }
    }

    pub fn create_message_attempt(
        &self,
        agent_id: &str,
        conversation_id: &str,
        content: &str,
        model_ref: &str,
    ) -> Result<MessageAttempt, DatabaseError> {
        if content.is_empty()
            || content.len() > MAX_USER_MESSAGE_BYTES
            || !valid_model_ref(model_ref)
        {
            return Err(DatabaseError::InvalidValue);
        }
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        verify_conversation_tx(&transaction, agent_id, conversation_id)?;
        let now = now_millis();
        let attempt = MessageAttempt {
            request_id: Uuid::now_v7().to_string(),
            user_message_id: Uuid::now_v7().to_string(),
            assistant_message_id: Uuid::now_v7().to_string(),
        };
        transaction.execute(
            "INSERT INTO conversation_messages
             (id, conversation_id, agent_id, author_type, content, status,
              created_at, completed_at)
             VALUES (?1, ?2, ?3, 'user', ?4, 'complete', ?5, ?5)",
            params![
                attempt.user_message_id,
                conversation_id,
                agent_id,
                content,
                now
            ],
        )?;
        transaction.execute(
            "INSERT INTO conversation_messages
             (id, conversation_id, agent_id, author_type, content, actual_model_ref,
              status, generation_request_id, created_at)
             VALUES (?1, ?2, ?3, 'agent', '', ?4, 'pending', ?5, ?6)",
            params![
                attempt.assistant_message_id,
                conversation_id,
                agent_id,
                model_ref,
                attempt.request_id,
                now + 1
            ],
        )?;
        transaction.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now + 1, conversation_id],
        )?;
        transaction.commit()?;
        Ok(attempt)
    }

    pub fn context_messages(
        &self,
        agent_id: &str,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<ContextMessage>, DatabaseError> {
        self.verify_conversation(agent_id, conversation_id)?;
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "WITH ordered AS (
               SELECT author_type, content, status, created_at, id,
                      LEAD(author_type) OVER (ORDER BY created_at, id) AS next_author,
                      LEAD(status) OVER (ORDER BY created_at, id) AS next_status
               FROM conversation_messages
               WHERE conversation_id = ?1 AND agent_id = ?2
             )
             SELECT author_type, content FROM (
               SELECT author_type, content, created_at, id
               FROM ordered
               WHERE status = 'complete' AND author_type IN ('user', 'agent')
                 AND (author_type = 'agent' OR next_author IS NULL
                      OR next_author != 'agent'
                      OR next_status IN ('pending', 'streaming', 'complete'))
               ORDER BY created_at DESC, id DESC LIMIT ?3
             ) ORDER BY created_at ASC, id ASC",
        )?;
        let messages = statement
            .query_map(params![conversation_id, agent_id, limit as i64], |row| {
                let author_raw: String = row.get(0)?;
                Ok(ContextMessage {
                    author: MessageAuthor::try_from(author_raw.as_str()).map_err(|()| {
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "author_type".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    content: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        let mut context = self.confirmed_memory_context(agent_id, 8)?;
        context.extend(messages);
        Ok(context)
    }

    pub fn mark_streaming(
        &self,
        assistant_message_id: &str,
        request_id: &str,
    ) -> Result<(), DatabaseError> {
        self.transition_message(
            assistant_message_id,
            Some(request_id),
            MessageStatus::Streaming,
            None,
        )
    }

    pub fn append_assistant_chunk(
        &self,
        assistant_message_id: &str,
        request_id: &str,
        chunk: &str,
    ) -> Result<(), DatabaseError> {
        if chunk.is_empty() {
            return Ok(());
        }
        let connection = self.open()?;
        let changed = connection.execute(
            "UPDATE conversation_messages SET content = content || ?1
             WHERE id = ?2 AND generation_request_id = ?3
               AND author_type = 'agent' AND status = 'streaming'",
            params![chunk, assistant_message_id, request_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(DatabaseError::InvalidTransition)
        }
    }

    pub fn finish_assistant(
        &self,
        assistant_message_id: &str,
        request_id: &str,
        status: MessageStatus,
        error_code: Option<&str>,
    ) -> Result<(), DatabaseError> {
        if !matches!(
            status,
            MessageStatus::Complete | MessageStatus::Failed | MessageStatus::Cancelled
        ) {
            return Err(DatabaseError::InvalidTransition);
        }
        self.transition_message(assistant_message_id, Some(request_id), status, error_code)
    }

    fn transition_message(
        &self,
        message_id: &str,
        request_id: Option<&str>,
        next: MessageStatus,
        error_code: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let current_raw = transaction
            .query_row(
                "SELECT status FROM conversation_messages
                 WHERE id = ?1 AND (?2 IS NULL OR generation_request_id = ?2)",
                params![message_id, request_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(DatabaseError::NotFound)?;
        let current = MessageStatus::try_from(current_raw.as_str())
            .map_err(|()| DatabaseError::InvalidValue)?;
        if !can_transition_message(current, next) {
            return Err(DatabaseError::InvalidTransition);
        }
        let terminal = matches!(
            next,
            MessageStatus::Complete | MessageStatus::Failed | MessageStatus::Cancelled
        );
        transaction.execute(
            "UPDATE conversation_messages
             SET status = ?1, completed_at = ?2, terminal_error_code = ?3
             WHERE id = ?4",
            params![
                next.as_str(),
                terminal.then(now_millis),
                error_code,
                message_id
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_safe_mode(&self, enabled: bool) -> Result<(), DatabaseError> {
        self.set_setting("safe_mode", if enabled { "true" } else { "false" })
    }

    fn set_setting(&self, key: &str, value_json: &str) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        connection.execute(
            "INSERT INTO app_settings (key, value_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = excluded.updated_at",
            params![key, value_json, now_millis()],
        )?;
        Ok(())
    }

    pub fn update_position(&self, agent_id: &str, x: f64, y: f64) -> Result<(), DatabaseError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(DatabaseError::InvalidValue);
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

    fn verify_conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        let exists = connection.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM conversations
               WHERE id = ?1 AND agent_id = ?2 AND archived_at IS NULL
             )",
            params![conversation_id, agent_id],
            |row| row.get::<_, bool>(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(DatabaseError::OwnershipMismatch)
        }
    }
}

fn verify_conversation_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    conversation_id: &str,
) -> Result<(), DatabaseError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM conversations
           WHERE id = ?1 AND agent_id = ?2 AND archived_at IS NULL
         )",
        params![conversation_id, agent_id],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(DatabaseError::OwnershipMismatch)
    }
}

fn valid_model_ref(value: &str) -> bool {
    let Some(model_id) = value.strip_prefix("ollama:") else {
        return false;
    };
    !model_id.is_empty()
        && model_id.len() <= 200
        && model_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".:_/-".contains(character))
}

fn validate_profile(agent: &ProvisionalAgent) -> Result<(), DatabaseError> {
    let birthday = agent.birthday.as_bytes();
    let birthday_is_iso_date = birthday.len() == 10
        && birthday[4] == b'-'
        && birthday[7] == b'-'
        && birthday
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    let traits_are_an_object = serde_json::from_str::<serde_json::Value>(&agent.traits_json)
        .is_ok_and(|value| value.is_object());

    if agent.name.trim().is_empty()
        || agent.name.len() > 120
        || !birthday_is_iso_date
        || agent.age_category.trim().is_empty()
        || agent.age_category.len() > 64
        || agent.species.trim().is_empty()
        || agent.species.len() > 120
        || agent.pronouns.trim().is_empty()
        || agent.pronouns.len() > 120
        || agent.personality_summary.len() > 1_000
        || !traits_are_an_object
        || agent.traits_json.len() > 8_192
        || agent.fictive_age > 10_000
        || !matches!(agent.appearance_preset.as_str(), "astra" | "luma")
    {
        return Err(DatabaseError::InvalidValue);
    }
    Ok(())
}

fn map_agent(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProvisionalAgent> {
    Ok(ProvisionalAgent {
        id: row.get(0)?,
        name: row.get(1)?,
        profile_key: row.get(2)?,
        sprite_key: row.get(3)?,
        position: AgentPosition {
            x: row.get(4)?,
            y: row.get(5)?,
        },
        birthday: row.get(6)?,
        fictive_age: row.get(7)?,
        age_category: row.get(8)?,
        species: row.get(9)?,
        pronouns: row.get(10)?,
        personality_summary: row.get(11)?,
        traits_json: row.get(12)?,
        appearance_preset: row.get(13)?,
    })
}

fn map_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentMemory> {
    Ok(AgentMemory {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        category: row.get(2)?,
        content: row.get(3)?,
        status: row.get(4)?,
        confirmation_status: row.get(5)?,
        confidence_milli: (row.get::<_, f64>(6)? * 1000.0).round().clamp(0.0, 1000.0) as u16,
        importance: row.get(7)?,
        source_type: row.get(8)?,
        source_message_id: row.get(9)?,
        source_conversation_id: row.get(10)?,
        conflict_key: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn map_simulated_state(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentSimulatedState> {
    Ok(AgentSimulatedState {
        agent_id: row.get(0)?,
        sleep: row.get(1)?,
        energy: row.get(2)?,
        mood: row.get(3)?,
        focus: row.get(4)?,
        curiosity: row.get(5)?,
        social_fatigue: row.get(6)?,
        mode: row.get(7)?,
        suspended: row.get(8)?,
        wake_now_until: row.get(9)?,
        last_simulated_at: row.get(10)?,
    })
}

fn map_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationMessage> {
    let author_raw: String = row.get(3)?;
    let status_raw: String = row.get(6)?;
    Ok(ConversationMessage {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        agent_id: row.get(2)?,
        author: MessageAuthor::try_from(author_raw.as_str()).map_err(|()| {
            rusqlite::Error::InvalidColumnType(3, "author_type".into(), rusqlite::types::Type::Text)
        })?,
        content: row.get(4)?,
        model_ref: row.get(5)?,
        status: MessageStatus::try_from(status_raw.as_str()).map_err(|()| {
            rusqlite::Error::InvalidColumnType(6, "status".into(), rusqlite::types::Type::Text)
        })?,
        created_at: row.get(7)?,
        completed_at: row.get(8)?,
        error_code: row.get(9)?,
    })
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::{params, Connection};
    use uuid::Uuid;

    use crate::domain::MessageStatus;

    use super::{Database, DatabaseError, PhaseOneSettings, ASTRA_ID, LUMA_ID, MIGRATION_0001};

    fn test_path() -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("aip-test-{}", Uuid::now_v7()))
            .join("aip.sqlite3")
    }

    fn cleanup(path: &std::path::Path) {
        let _ = fs::remove_dir_all(path.parent().expect("test path should have a parent"));
    }

    #[test]
    fn fresh_database_reaches_version_two_and_reopens_idempotently() {
        let path = test_path();
        let first = Database::initialize(&path).expect("database should initialize");
        let second = Database::initialize(&path).expect("database should reinitialize");
        let snapshot = second.snapshot().expect("snapshot should load");
        assert_eq!(snapshot.migration_version, 7);
        assert_eq!(snapshot.agents.len(), 2);
        for agent in &snapshot.agents {
            assert_eq!(
                first.main_conversation(&agent.id).unwrap().title,
                "Conversa principal"
            );
        }
        drop(first);
        drop(second);
        cleanup(&path);
    }

    #[test]
    fn version_one_upgrades_without_losing_existing_data() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(MIGRATION_0001).unwrap();
        connection
            .execute(
                "INSERT INTO users (id, role, display_name, created_at, updated_at)
                 VALUES ('preserved', 'owner', 'Preserved', 1, 1)",
                [],
            )
            .unwrap();
        drop(connection);

        let database = Database::initialize(&path).expect("v1 database should upgrade");
        assert_eq!(database.snapshot().unwrap().migration_version, 7);
        let connection = Connection::open(&path).unwrap();
        let preserved: String = connection
            .query_row(
                "SELECT display_name FROM users WHERE id = 'preserved'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved, "Preserved");
        drop(connection);
        drop(database);
        cleanup(&path);
    }

    #[test]
    fn histories_are_isolated_ordered_and_transitions_are_validated() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let agents = database.snapshot().unwrap().agents;
        let first = &agents[0];
        let second = &agents[1];
        let first_conversation = database.main_conversation(&first.id).unwrap();
        let second_conversation = database.main_conversation(&second.id).unwrap();
        let attempt = database
            .create_message_attempt(
                &first.id,
                &first_conversation.id,
                "Synthetic message",
                "ollama:test",
            )
            .unwrap();
        assert_eq!(
            database
                .messages(&first.id, &first_conversation.id)
                .unwrap()
                .len(),
            2
        );
        assert!(database
            .messages(&second.id, &second_conversation.id)
            .unwrap()
            .is_empty());
        assert_eq!(
            database.messages(&second.id, &first_conversation.id),
            Err(DatabaseError::OwnershipMismatch)
        );
        database
            .mark_streaming(&attempt.assistant_message_id, &attempt.request_id)
            .unwrap();
        database
            .append_assistant_chunk(
                &attempt.assistant_message_id,
                &attempt.request_id,
                "Synthetic reply",
            )
            .unwrap();
        database
            .finish_assistant(
                &attempt.assistant_message_id,
                &attempt.request_id,
                MessageStatus::Complete,
                None,
            )
            .unwrap();
        assert_eq!(
            database.finish_assistant(
                &attempt.assistant_message_id,
                &attempt.request_id,
                MessageStatus::Cancelled,
                None,
            ),
            Err(DatabaseError::InvalidTransition)
        );
        let messages = database
            .messages(&first.id, &first_conversation.id)
            .unwrap();
        assert_eq!(messages[0].content, "Synthetic message");
        assert_eq!(messages[1].content, "Synthetic reply");
        cleanup(&path);
    }

    #[test]
    fn interrupted_messages_recover_and_settings_fall_back() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let agent = database.snapshot().unwrap().agents.remove(0);
        let conversation = database.main_conversation(&agent.id).unwrap();
        let attempt = database
            .create_message_attempt(
                &agent.id,
                &conversation.id,
                "Synthetic message",
                "ollama:test",
            )
            .unwrap();
        database
            .mark_streaming(&attempt.assistant_message_id, &attempt.request_id)
            .unwrap();
        database
            .set_selected_model(&agent.id, "ollama:test")
            .unwrap();
        database.set_keep_alive(&agent.id, 30).unwrap();
        drop(database);

        let reopened = Database::initialize(&path).unwrap();
        let messages = reopened.messages(&agent.id, &conversation.id).unwrap();
        assert_eq!(messages[1].status, MessageStatus::Failed);
        assert_eq!(
            messages[1].error_code.as_deref(),
            Some("runtime_interrupted")
        );
        assert_eq!(
            reopened
                .settings(&agent.id)
                .unwrap()
                .selected_model_ref
                .as_deref(),
            Some("ollama:test")
        );
        assert_eq!(reopened.settings(&agent.id).unwrap().keep_alive_minutes, 30);

        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE app_settings SET value_json = '9999'
                 WHERE key = 'phase1_keep_alive_minutes'",
                params![],
            )
            .unwrap();
        drop(connection);
        assert_eq!(reopened.settings(&agent.id).unwrap().keep_alive_minutes, 30);
        cleanup(&path);
    }

    #[test]
    fn phase_one_settings_and_context_are_scoped_to_each_agent() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let agents = database.snapshot().unwrap().agents;
        let astra = agents.iter().find(|agent| agent.id == ASTRA_ID).unwrap();
        let luma = agents.iter().find(|agent| agent.id == LUMA_ID).unwrap();
        let astra_conversation = database.main_conversation(&astra.id).unwrap();
        let luma_conversation = database.main_conversation(&luma.id).unwrap();

        database
            .set_selected_model(&astra.id, "ollama:astra-model")
            .unwrap();
        database
            .set_selected_model(&luma.id, "ollama:luma-model")
            .unwrap();
        database.set_keep_alive(&astra.id, 5).unwrap();
        database.set_keep_alive(&luma.id, 30).unwrap();

        let astra_attempt = database
            .create_message_attempt(
                &astra.id,
                &astra_conversation.id,
                "Astra-only context",
                "ollama:astra-model",
            )
            .unwrap();
        database
            .mark_streaming(
                &astra_attempt.assistant_message_id,
                &astra_attempt.request_id,
            )
            .unwrap();
        database
            .append_assistant_chunk(
                &astra_attempt.assistant_message_id,
                &astra_attempt.request_id,
                "Astra-only reply",
            )
            .unwrap();
        database
            .finish_assistant(
                &astra_attempt.assistant_message_id,
                &astra_attempt.request_id,
                MessageStatus::Complete,
                None,
            )
            .unwrap();
        database
            .create_message_attempt(
                &luma.id,
                &luma_conversation.id,
                "Luma-only context",
                "ollama:luma-model",
            )
            .unwrap();

        assert_eq!(
            database.settings(&astra.id).unwrap(),
            PhaseOneSettings {
                selected_model_ref: Some("ollama:astra-model".into()),
                keep_alive_minutes: 5,
            }
        );
        assert_eq!(
            database.settings(&luma.id).unwrap(),
            PhaseOneSettings {
                selected_model_ref: Some("ollama:luma-model".into()),
                keep_alive_minutes: 30,
            }
        );
        let luma_context = database
            .context_messages(&luma.id, &luma_conversation.id, 32)
            .unwrap();
        assert_eq!(luma_context.len(), 1);
        assert_eq!(luma_context[0].content, "Luma-only context");
        cleanup(&path);
    }

    #[test]
    fn onboarding_requires_both_profiles_and_is_idempotent() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let agents = database.snapshot().unwrap().agents;
        assert!(database.snapshot().unwrap().onboarding_required);
        assert_eq!(
            database.complete_onboarding(&agents[..1]),
            Err(DatabaseError::InvalidValue)
        );
        assert!(database.snapshot().unwrap().onboarding_required);

        database.complete_onboarding(&agents).unwrap();
        assert!(!database.snapshot().unwrap().onboarding_required);
        drop(database);

        let reopened = Database::initialize(&path).unwrap();
        let snapshot = reopened.snapshot().unwrap();
        assert_eq!(snapshot.agents.len(), 2);
        assert!(!snapshot.onboarding_required);
        for agent in snapshot.agents {
            assert!(matches!(agent.id.as_str(), ASTRA_ID | LUMA_ID));
            assert!(reopened.main_conversation(&agent.id).is_ok());
        }
        cleanup(&path);
    }

    #[test]
    fn profile_edits_preserve_conversations_and_agent_settings() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut agent = database.agent(ASTRA_ID).unwrap();
        let conversation = database.main_conversation(ASTRA_ID).unwrap();
        database
            .set_selected_model(ASTRA_ID, "ollama:astra-model")
            .unwrap();
        database.set_keep_alive(ASTRA_ID, 30).unwrap();
        agent.name = "Astra edited".into();
        agent.species = "fox".into();
        agent.traits_json = r#"{"curiosity":80}"#.into();
        database.update_profile(&agent).unwrap();

        let restored = database.agent(ASTRA_ID).unwrap();
        assert_eq!(restored.name, "Astra edited");
        assert_eq!(restored.species, "fox");
        assert_eq!(restored.traits_json, r#"{"curiosity":80}"#);
        assert_eq!(
            database.main_conversation(ASTRA_ID).unwrap().id,
            conversation.id
        );
        assert_eq!(
            database
                .settings(ASTRA_ID)
                .unwrap()
                .selected_model_ref
                .as_deref(),
            Some("ollama:astra-model")
        );
        assert_eq!(database.settings(ASTRA_ID).unwrap().keep_alive_minutes, 30);
        cleanup(&path);
    }

    #[test]
    fn phase_three_conversations_and_memories_are_agent_scoped() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let astra = database
            .create_conversation(ASTRA_ID, "Astra notes")
            .unwrap();
        database
            .set_active_conversation(ASTRA_ID, &astra.id)
            .unwrap();
        assert_eq!(database.active_conversation(ASTRA_ID).unwrap().id, astra.id);
        assert_eq!(database.conversations(LUMA_ID).unwrap().len(), 1);
        assert_eq!(
            database.set_active_conversation(LUMA_ID, &astra.id),
            Err(DatabaseError::OwnershipMismatch)
        );

        let memory = database
            .create_memory(ASTRA_ID, "preference", "Likes astronomy", true)
            .unwrap();
        assert_eq!(database.memories(ASTRA_ID).unwrap().len(), 1);
        assert!(database.memories(LUMA_ID).unwrap().is_empty());
        assert_eq!(
            database.set_memory_status(LUMA_ID, &memory.id, "archived"),
            Err(DatabaseError::OwnershipMismatch)
        );
        database
            .set_memory_status(ASTRA_ID, &memory.id, "archived")
            .unwrap();
        assert_eq!(database.memories(ASTRA_ID).unwrap()[0].status, "archived");
        cleanup(&path);
    }

    #[test]
    fn confirmed_memory_is_scoped_and_added_to_context() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let astra = database.main_conversation(ASTRA_ID).unwrap();
        let luma = database.main_conversation(LUMA_ID).unwrap();
        database
            .create_memory(ASTRA_ID, "fact", "Astra fact", true)
            .unwrap();
        database
            .create_memory(LUMA_ID, "fact", "Luma fact", true)
            .unwrap();
        database
            .create_memory(ASTRA_ID, "fact", "Pending Astra fact", false)
            .unwrap();
        let astra_context = database.context_messages(ASTRA_ID, &astra.id, 32).unwrap();
        let luma_context = database.context_messages(LUMA_ID, &luma.id, 32).unwrap();
        assert!(astra_context
            .iter()
            .any(|message| message.content.contains("Astra fact")));
        assert!(!astra_context
            .iter()
            .any(|message| message.content.contains("Luma fact")));
        assert!(!astra_context
            .iter()
            .any(|message| message.content.contains("Pending Astra fact")));
        assert!(luma_context
            .iter()
            .any(|message| message.content.contains("Luma fact")));
        cleanup(&path);
    }

    #[test]
    fn simulated_state_is_agent_scoped_and_validated() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        assert_eq!(database.simulated_state(ASTRA_ID).unwrap().mode, "normal");
        database.set_agent_mode(ASTRA_ID, "silent").unwrap();
        assert_eq!(database.simulated_state(ASTRA_ID).unwrap().mode, "silent");
        assert_eq!(database.simulated_state(LUMA_ID).unwrap().mode, "normal");
        assert_eq!(
            database.set_agent_mode(ASTRA_ID, "invalid"),
            Err(DatabaseError::InvalidValue)
        );
        database.set_agent_suspended(ASTRA_ID, true).unwrap();
        database
            .wake_agent_now(ASTRA_ID, super::now_millis() + 60_000)
            .unwrap();
        let state = database.simulated_state(ASTRA_ID).unwrap();
        assert!(state.suspended);
        assert_eq!(state.sleep, 0);
        assert!(state.wake_now_until.is_some());
        cleanup(&path);
    }

    #[test]
    fn simulated_state_advances_without_changing_suspended_agents() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let before = database.simulated_state(ASTRA_ID).unwrap();
        database
            .advance_simulated_state(ASTRA_ID, before.last_simulated_at + 10 * 60_000)
            .unwrap();
        let advanced = database.simulated_state(ASTRA_ID).unwrap();
        assert!(advanced.sleep <= before.sleep);
        assert!(advanced.energy < before.energy || advanced.sleep > 0);
        database.set_agent_suspended(ASTRA_ID, true).unwrap();
        let suspended = database.simulated_state(ASTRA_ID).unwrap();
        database
            .advance_simulated_state(ASTRA_ID, suspended.last_simulated_at + 10 * 60_000)
            .unwrap();
        assert_eq!(database.simulated_state(ASTRA_ID).unwrap(), suspended);
        cleanup(&path);
    }

    #[test]
    fn pixel_documents_are_created_per_agent_at_64_pixels() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let connection = Connection::open(&path).unwrap();
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pixel_documents WHERE width = 64 AND height = 64",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        drop(connection);
        drop(database);
        cleanup(&path);
    }

    #[test]
    fn pixel_document_round_trips_per_agent() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let edited = r##"{"layers":[{"id":"body","pixels":[[0,0,"#fff"]]}],"attachmentPoints":{"bubble":{"x":1,"y":2}}}"##;
        database.save_pixel_document(ASTRA_ID, edited).unwrap();
        assert_eq!(database.pixel_document(ASTRA_ID).unwrap(), edited);
        assert_ne!(database.pixel_document(LUMA_ID).unwrap(), edited);
        assert_eq!(
            database.save_pixel_document(ASTRA_ID, "{}"),
            Err(DatabaseError::InvalidValue)
        );
        cleanup(&path);
    }

    #[test]
    fn position_and_safe_mode_still_persist() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let agent_id = database.snapshot().unwrap().agents[0].id.clone();
        database.update_position(&agent_id, 412.0, 216.0).unwrap();
        database.set_safe_mode(true).unwrap();
        drop(database);
        let reopened = Database::initialize(&path).unwrap();
        let snapshot = reopened.snapshot().unwrap();
        let updated = snapshot
            .agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .unwrap();
        assert_eq!(updated.position.x, 412.0);
        assert!(snapshot.safe_mode);
        cleanup(&path);
    }
}
