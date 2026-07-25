use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    can_transition_message, AgentPosition, ConversationMessage, MessageAuthor, MessageStatus,
    PhaseOneConversation, ProvisionalAgent, DEFAULT_KEEP_ALIVE_MINUTES, MAX_KEEP_ALIVE_MINUTES,
    MAX_USER_MESSAGE_BYTES,
};

const MIGRATION_0001: &str = include_str!("../migrations/0001_phase0.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_phase1_conversations.sql");
const MIGRATION_0003: &str = include_str!("../migrations/0003_phase1_agent_settings.sql");
const MIGRATION_0004: &str = include_str!("../migrations/0004_phase2_identity.sql");
const MIGRATIONS: [(i64, &str); 4] = [
    (1, MIGRATION_0001),
    (2, MIGRATION_0002),
    (3, MIGRATION_0003),
    (4, MIGRATION_0004),
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
        if agent.name.trim().is_empty()
            || agent.birthday.len() != 10
            || agent.age_category.trim().is_empty()
            || agent.species.trim().is_empty()
            || agent.pronouns.trim().is_empty()
            || agent.fictive_age > 10_000
            || !matches!(agent.appearance_preset.as_str(), "astra" | "luma")
        {
            return Err(DatabaseError::InvalidValue);
        }
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
        if agents.is_empty()
            || agents.len() > 2
            || agents
                .iter()
                .any(|agent| !matches!(agent.id.as_str(), ASTRA_ID | LUMA_ID))
        {
            return Err(DatabaseError::InvalidValue);
        }
        for agent in agents {
            self.update_profile(agent)?;
        }
        self.set_setting("phase2_onboarding_complete", "true")
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
        Ok(messages)
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
        assert_eq!(snapshot.migration_version, 4);
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
        assert_eq!(database.snapshot().unwrap().migration_version, 4);
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
