use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError};

pub const COMPANION_PROTOCOL_VERSION: i64 = 1;
pub const COMPANION_MIN_PROTOCOL_VERSION: i64 = 1;
pub const COMPANION_FIXTURE_DEVICE_ID: &str = "android-fixture-01";
pub const COMPANION_FIXTURE_FINGERPRINT: &str = "fixture:fingerprint/android-01";
pub const COMPANION_FIXTURE_PAIRING_NONCE: &str = "fixture:pairing/android-01";
pub const COMPANION_FIXTURE_APP_VERSION: &str = "0.1.0-fixture";

const COMPANION_PAIRING_TTL_MS: i64 = 10 * 60 * 1_000;
const COMPANION_SESSION_TTL_MS: i64 = 30 * 60 * 1_000;
const MAX_COMPANION_DEVICES: i64 = 4;
const MAX_COMPANION_QUEUE_ITEMS: i64 = 16;
const MAX_COMPANION_HISTORY_ROWS: i64 = 100;
const MAX_COMPANION_AUDIT_ROWS: i64 = 100;
const MAX_COMPANION_TEXT_BYTES: usize = 16_384;
const MAX_COMPANION_PAYLOAD_BYTES: usize = 12_288;
const MAX_COMPANION_SUMMARY_BYTES: usize = 512;
const MAX_COMPANION_METADATA_BYTES: usize = 2_048;
const MAX_COMPANION_MEDIA_METADATA_BYTES: i64 = 100_000_000;
const MAX_COMPANION_RETRY_COUNT: i64 = 8;
const MAX_COMPANION_REQUEST_BYTES: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionPlatform {
    Android,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionDeviceStatus {
    PairingRequested,
    Paired,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionSessionStatus {
    Connected,
    Disconnected,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionQueueStatus {
    Previewed,
    Queued,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionMessageKind {
    Pairing,
    Session,
    Queue,
    History,
    KeyRotation,
    Revocation,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CompanionQueuePayload {
    Text {
        text: String,
    },
    Audio {
        mime_type: String,
        duration_ms: i64,
        byte_length: i64,
    },
    Image {
        mime_type: String,
        width: i64,
        height: i64,
        byte_length: i64,
    },
    File {
        file_name: String,
        mime_type: String,
        byte_length: i64,
    },
    Task {
        title: String,
        summary: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionProtocolInfo {
    pub schema_version: i64,
    pub protocol_version: i64,
    pub min_protocol_version: i64,
    pub platform: CompanionPlatform,
    pub app_version: String,
    pub transport: String,
    pub network_listener: bool,
    pub standalone_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionProtocolMessage {
    pub schema_version: i64,
    pub protocol_version: i64,
    pub message_id: String,
    pub device_id: String,
    pub platform: CompanionPlatform,
    pub app_version: String,
    pub kind: CompanionMessageKind,
    pub session_id: Option<String>,
    pub nonce_metadata: String,
    pub replay_counter: i64,
    pub payload_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionDevice {
    pub id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub device_id: String,
    pub platform: CompanionPlatform,
    pub app_version: String,
    pub protocol_version: i64,
    pub status: CompanionDeviceStatus,
    pub fingerprint: String,
    pub pairing_nonce_metadata: String,
    pub key_version: i64,
    pub pairing_expires_at: Option<i64>,
    pub paired_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub last_seen_at: Option<i64>,
    pub compatible: bool,
    pub standalone_fallback: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSession {
    pub id: String,
    pub device_id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub status: CompanionSessionStatus,
    pub protocol_version: i64,
    pub app_version: String,
    pub negotiated_protocol_version: i64,
    pub key_fingerprint: String,
    pub session_nonce_metadata: String,
    pub last_replay_counter: i64,
    pub connected_at: i64,
    pub last_seen_at: i64,
    pub disconnected_at: Option<i64>,
    pub protocol: CompanionProtocolInfo,
    pub handshake: CompanionProtocolMessage,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionQueueItem {
    pub id: String,
    pub device_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub kind: String,
    pub status: CompanionQueueStatus,
    pub payload: CompanionQueuePayload,
    pub summary: String,
    pub metadata_only: bool,
    pub media_bytes_persisted: bool,
    pub approval_required: bool,
    pub retry_count: i64,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub previewed_at: i64,
    pub approved_at: Option<i64>,
    pub cancelled_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionHistoryRecord {
    pub id: String,
    pub device_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: String,
    pub owner_user_id: String,
    pub direction: String,
    pub kind: String,
    pub summary: String,
    pub metadata_only: bool,
    pub media_bytes_persisted: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionAuditRecord {
    pub id: String,
    pub device_id: Option<String>,
    pub session_id: Option<String>,
    pub queue_id: Option<String>,
    pub agent_id: String,
    pub owner_user_id: String,
    pub event: String,
    pub result: String,
    pub code: Option<String>,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionKeyRotation {
    pub id: String,
    pub device_id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub old_fingerprint: String,
    pub new_fingerprint: String,
    pub old_key_version: i64,
    pub new_key_version: i64,
    pub nonce_metadata: String,
    pub status: String,
    pub reason: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionRevocation {
    pub id: String,
    pub device_id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub previous_status: String,
    pub reason: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionPairingRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub device_id: String,
    pub platform: CompanionPlatform,
    pub app_version: String,
    pub protocol_version: i64,
    pub fingerprint: String,
    pub pairing_nonce_metadata: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionPairingConfirmationRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub device_id: String,
    pub fingerprint: String,
    pub pairing_nonce_metadata: String,
    pub confirmed: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSessionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub device_id: String,
    pub app_version: String,
    pub protocol_version: i64,
    pub fingerprint: String,
    pub pairing_nonce_metadata: String,
    pub message_nonce_metadata: String,
    pub replay_counter: i64,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSessionProof {
    pub session_id: String,
    pub device_id: String,
    pub session_nonce_metadata: String,
    pub key_fingerprint: String,
    pub app_version: String,
    pub protocol_version: i64,
    pub message_nonce_metadata: String,
    pub replay_counter: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionReconnectRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: CompanionSessionProof,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionQueuePreviewRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: CompanionSessionProof,
    pub payload: CompanionQueuePayload,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionQueueDecisionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: CompanionSessionProof,
    pub queue_id: String,
    pub approved: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionQueueActionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: CompanionSessionProof,
    pub queue_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionDeviceActionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub device_id: String,
    pub reason: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

impl CompanionQueuePayload {
    fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::Audio { .. } => "audio",
            Self::Image { .. } => "image",
            Self::File { .. } => "file",
            Self::Task { .. } => "task",
        }
    }

    fn validate(&self) -> Result<(), DatabaseError> {
        match self {
            Self::Text { text } => validate_text(text, MAX_COMPANION_TEXT_BYTES).map(|_| ()),
            Self::Audio {
                mime_type,
                duration_ms,
                byte_length,
            } => {
                validate_mime(mime_type, "audio/")?;
                if !(1..=300_000).contains(duration_ms)
                    || !(0..=MAX_COMPANION_MEDIA_METADATA_BYTES).contains(byte_length)
                {
                    return Err(DatabaseError::Cognitive("companion_payload_invalid"));
                }
                Ok(())
            }
            Self::Image {
                mime_type,
                width,
                height,
                byte_length,
            } => {
                validate_mime(mime_type, "image/")?;
                if !(1..=8_192).contains(width)
                    || !(1..=8_192).contains(height)
                    || !(0..=MAX_COMPANION_MEDIA_METADATA_BYTES).contains(byte_length)
                {
                    return Err(DatabaseError::Cognitive("companion_payload_invalid"));
                }
                Ok(())
            }
            Self::File {
                file_name,
                mime_type,
                byte_length,
            } => {
                validate_file_name(file_name)?;
                validate_mime(mime_type, "")?;
                if !(0..=MAX_COMPANION_MEDIA_METADATA_BYTES).contains(byte_length) {
                    return Err(DatabaseError::Cognitive("companion_payload_invalid"));
                }
                Ok(())
            }
            Self::Task { title, summary } => {
                validate_text(title, 256)?;
                validate_text(summary, 2_048).map(|_| ())
            }
        }
    }

    fn summary(&self) -> String {
        match self {
            Self::Text { text } => format!("Texto: {}", bounded_summary(text)),
            Self::Audio {
                mime_type,
                duration_ms,
                byte_length,
            } => format!("Áudio metadata-only: {mime_type}, {duration_ms} ms, {byte_length} bytes"),
            Self::Image {
                mime_type,
                width,
                height,
                byte_length,
            } => {
                format!("Imagem metadata-only: {mime_type}, {width}×{height}, {byte_length} bytes")
            }
            Self::File {
                file_name,
                mime_type,
                byte_length,
            } => format!("Arquivo metadata-only: {file_name}, {mime_type}, {byte_length} bytes"),
            Self::Task { title, summary } => {
                format!(
                    "Tarefa: {} — {}",
                    bounded_summary(title),
                    bounded_summary(summary)
                )
            }
        }
    }

    fn json(&self) -> Result<String, DatabaseError> {
        self.validate()?;
        let value = serde_json::to_string(self).map_err(|_| DatabaseError::Unavailable)?;
        if value.len() > MAX_COMPANION_PAYLOAD_BYTES {
            Err(DatabaseError::Cognitive("companion_payload_oversized"))
        } else {
            Ok(value)
        }
    }
}

impl Database {
    pub fn list_companion_devices(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionDevice>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, agent_id, owner_user_id, device_id, platform, app_version,
                    protocol_version, status, fingerprint, pairing_nonce_metadata,
                    key_version, pairing_expires_at, paired_at, revoked_at,
                    last_seen_at, created_at, updated_at
             FROM companion_devices
             WHERE agent_id = ?1
             ORDER BY updated_at DESC, id DESC",
        )?;
        let devices = statement
            .query_map(params![agent_id], companion_device_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(devices)
    }

    pub fn start_companion_pairing(
        &self,
        request: CompanionPairingRequest,
    ) -> Result<CompanionDevice, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_pairing_fixture(&request)?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "pairing_start",
            &idempotency_key,
            &request_json,
            "device",
        )? {
            let device = load_device_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(device);
        }

        let existing = transaction
            .query_row(
                "SELECT id, status, agent_id FROM companion_devices
                 WHERE owner_user_id = ?1 AND device_id = ?2",
                params![request.owner_user_id, request.device_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let has_existing = existing.is_some();
        let (id, existing_status, existing_agent_id) = existing.unwrap_or_else(|| {
            (
                Uuid::now_v7().to_string(),
                String::from("expired"),
                request.agent_id.clone(),
            )
        });
        if has_existing && existing_agent_id != request.agent_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if has_existing && existing_status == "revoked" {
            return Err(DatabaseError::Cognitive("companion_device_revoked"));
        }
        if has_existing && existing_status == "paired" {
            return Err(DatabaseError::Cognitive("companion_device_already_paired"));
        }
        if !has_existing {
            let active_devices: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM companion_devices
                 WHERE agent_id = ?1 AND status IN ('pairing_requested', 'paired')",
                params![request.agent_id],
                |row| row.get(0),
            )?;
            if active_devices >= MAX_COMPANION_DEVICES {
                return Err(DatabaseError::Cognitive("companion_device_limit"));
            }
        }
        let now = now_millis();
        let expires_at = now + COMPANION_PAIRING_TTL_MS;
        if has_existing {
            transaction.execute(
                "UPDATE companion_devices
                 SET agent_id = ?1, owner_user_id = ?2, platform = 'android',
                     app_version = ?3, protocol_version = ?4, status = 'pairing_requested',
                     fingerprint = ?5, pairing_nonce_metadata = ?6, key_version = 1,
                     pairing_expires_at = ?7, paired_at = NULL, revoked_at = NULL,
                     last_seen_at = NULL, updated_at = ?8
                 WHERE id = ?9",
                params![
                    request.agent_id,
                    request.owner_user_id,
                    request.app_version,
                    request.protocol_version,
                    request.fingerprint,
                    request.pairing_nonce_metadata,
                    expires_at,
                    now,
                    id,
                ],
            )?;
        } else {
            transaction.execute(
                "INSERT INTO companion_devices
                 (id, agent_id, owner_user_id, device_id, platform, app_version,
                  protocol_version, status, fingerprint, pairing_nonce_metadata, key_version,
                  pairing_expires_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'android', ?5, ?6, 'pairing_requested',
                         ?7, ?8, 1, ?9, ?10, ?10)",
                params![
                    id,
                    request.agent_id,
                    request.owner_user_id,
                    request.device_id,
                    request.app_version,
                    request.protocol_version,
                    request.fingerprint,
                    request.pairing_nonce_metadata,
                    expires_at,
                    now,
                ],
            )?;
        }
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&id),
                session_id: None,
                queue_id: None,
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "pairing_started",
                result: "awaiting_owner_confirmation",
                code: None,
                summary: "Pareamento sintético Android aguardando confirmação do Owner",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "pairing_start",
            &idempotency_key,
            &request_json,
            "device",
            &id,
        )?;
        let device = load_device_tx(&transaction, &id)?;
        transaction.commit()?;
        Ok(device)
    }

    pub fn confirm_companion_pairing(
        &self,
        request: CompanionPairingConfirmationRequest,
    ) -> Result<CompanionDevice, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_reference(&request.device_id, 96, "companion_device_invalid")?;
        validate_reference(&request.fingerprint, 192, "companion_fingerprint_invalid")?;
        validate_reference(
            &request.pairing_nonce_metadata,
            192,
            "companion_nonce_invalid",
        )?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "pairing_confirm",
            &idempotency_key,
            &request_json,
            "device",
        )? {
            let device = load_device_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(device);
        }
        if !request.confirmed {
            return Err(DatabaseError::Cognitive(
                "companion_pairing_confirmation_required",
            ));
        }
        let mut device = load_device_by_external_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.device_id,
        )?;
        if device.status == CompanionDeviceStatus::Expired {
            return Err(DatabaseError::Cognitive("companion_pairing_expired"));
        }
        if device.status != CompanionDeviceStatus::PairingRequested
            || device.fingerprint != request.fingerprint
            || device.pairing_nonce_metadata != request.pairing_nonce_metadata
        {
            return Err(DatabaseError::Cognitive("companion_pairing_invalid"));
        }
        let now = now_millis();
        if device
            .pairing_expires_at
            .is_none_or(|expires| expires < now)
        {
            transaction.execute(
                "UPDATE companion_devices SET status = 'expired', updated_at = ?1 WHERE id = ?2",
                params![now, device.id],
            )?;
            return Err(DatabaseError::Cognitive("companion_pairing_expired"));
        }
        transaction.execute(
            "UPDATE companion_devices
             SET status = 'paired', paired_at = ?1, pairing_expires_at = NULL, updated_at = ?1
             WHERE id = ?2 AND status = 'pairing_requested'",
            params![now, device.id],
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&device.id),
                session_id: None,
                queue_id: None,
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "pairing_confirmed",
                result: "paired",
                code: None,
                summary: "Pareamento sintético confirmado explicitamente pelo Owner",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "pairing_confirm",
            &idempotency_key,
            &request_json,
            "device",
            &device.id,
        )?;
        device = load_device_tx(&transaction, &device.id)?;
        transaction.commit()?;
        Ok(device)
    }

    pub fn list_companion_sessions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionSession>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT s.id FROM companion_sessions s
             JOIN companion_devices d ON d.id = s.device_id
             WHERE s.agent_id = ?1
             ORDER BY s.last_seen_at DESC, s.id DESC",
        )?;
        let ids = statement
            .query_map(params![agent_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter()
            .map(|id| load_session_connection(&connection, id))
            .collect()
    }

    pub fn connect_companion_session(
        &self,
        request: CompanionSessionRequest,
    ) -> Result<CompanionSession, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_reference(&request.device_id, 96, "companion_device_invalid")?;
        validate_reference(&request.fingerprint, 192, "companion_fingerprint_invalid")?;
        validate_reference(
            &request.pairing_nonce_metadata,
            192,
            "companion_nonce_invalid",
        )?;
        validate_reference(
            &request.message_nonce_metadata,
            192,
            "companion_nonce_invalid",
        )?;
        if request.replay_counter < 1 {
            return Err(DatabaseError::Cognitive("companion_replay_rejected"));
        }
        ensure_compatible(request.protocol_version, &request.app_version)?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "session_connect",
            &idempotency_key,
            &request_json,
            "session",
        )? {
            let session = load_session_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let device = load_device_by_external_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.device_id,
        )?;
        if device.status == CompanionDeviceStatus::Revoked {
            return Err(DatabaseError::Cognitive("companion_device_revoked"));
        }
        if device.status != CompanionDeviceStatus::Paired {
            return Err(DatabaseError::Cognitive("companion_pairing_required"));
        }
        if device.fingerprint != request.fingerprint
            || device.pairing_nonce_metadata != request.pairing_nonce_metadata
        {
            return Err(DatabaseError::Cognitive("companion_authentication_failed"));
        }
        ensure_replay_nonce_available(&transaction, &device.id, &request.message_nonce_metadata)?;
        ensure_replay_counter_fresh(&transaction, &device.id, request.replay_counter)?;
        let now = now_millis();
        transaction.execute(
            "UPDATE companion_sessions
             SET status = 'disconnected', disconnected_at = ?1, updated_at = ?1
             WHERE device_id = ?2 AND status = 'connected'",
            params![now, device.id],
        )?;
        transaction.execute(
            "UPDATE companion_devices SET last_seen_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, device.id],
        )?;
        let session_id = Uuid::now_v7().to_string();
        let session_nonce_metadata =
            format!("fixture:session/{}/{}", device.device_id, Uuid::now_v7());
        transaction.execute(
            "INSERT INTO companion_sessions
             (id, device_id, agent_id, owner_user_id, status, protocol_version, app_version,
              negotiated_protocol_version, key_fingerprint, session_nonce_metadata,
              last_replay_counter, connected_at, last_seen_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'connected', ?5, ?6, ?5, ?7, ?8, ?9, ?10, ?10, ?10)",
            params![
                session_id,
                device.id,
                request.agent_id,
                request.owner_user_id,
                request.protocol_version,
                request.app_version,
                device.fingerprint,
                session_nonce_metadata,
                request.replay_counter,
                now,
            ],
        )?;
        insert_replay_guard_tx(
            &transaction,
            &device.id,
            Some(&session_id),
            &request.message_nonce_metadata,
            request.replay_counter,
            "session_connect",
        )?;
        insert_history_tx(
            &transaction,
            HistoryContext {
                device_id: Some(&device.id),
                session_id: Some(&session_id),
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                direction: "system",
                kind: "session",
                summary: "Sessão local autenticada; fallback desktop disponível",
            },
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&device.id),
                session_id: Some(&session_id),
                queue_id: None,
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "session_connected",
                result: "authenticated",
                code: None,
                summary: "Sessão sintética autenticada com negociação de protocolo local",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "session_connect",
            &idempotency_key,
            &request_json,
            "session",
            &session_id,
        )?;
        let session = load_session_tx(&transaction, &session_id)?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn reconnect_companion_session(
        &self,
        request: CompanionReconnectRequest,
    ) -> Result<CompanionSession, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_session_proof(&request.proof)?;
        ensure_compatible(request.proof.protocol_version, &request.proof.app_version)?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "session_reconnect",
            &idempotency_key,
            &request_json,
            "session",
        )? {
            let session = load_session_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let session = authenticate_session_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            "session_reconnect",
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&session.device_id),
                session_id: Some(&session.id),
                queue_id: None,
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "session_reconnected",
                result: "negotiated",
                code: None,
                summary: "Reconexão local aceita após replay e compatibilidade",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "session_reconnect",
            &idempotency_key,
            &request_json,
            "session",
            &session.id,
        )?;
        let session = load_session_tx(&transaction, &session.id)?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn list_companion_queue(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionQueueItem>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id FROM companion_queue
             WHERE agent_id = ?1
             ORDER BY updated_at DESC, id DESC",
        )?;
        let ids = statement
            .query_map(params![agent_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter()
            .map(|id| load_queue_connection(&connection, id))
            .collect()
    }

    pub fn preview_companion_queue(
        &self,
        request: CompanionQueuePreviewRequest,
    ) -> Result<CompanionQueueItem, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.payload.validate()?;
        validate_session_proof(&request.proof)?;
        let payload_json = request.payload.json()?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "queue_preview",
            &idempotency_key,
            &request_json,
            "queue",
        )? {
            let item = load_queue_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(item);
        }
        let session = authenticate_session_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            "queue_preview",
        )?;
        let active_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM companion_queue
             WHERE session_id = ?1 AND status IN ('previewed', 'queued')",
            params![session.id],
            |row| row.get(0),
        )?;
        if active_count >= MAX_COMPANION_QUEUE_ITEMS {
            return Err(DatabaseError::Cognitive("companion_queue_limit"));
        }
        let queue_id = Uuid::now_v7().to_string();
        let now = now_millis();
        let summary = bounded_summary(&request.payload.summary());
        let inserted = transaction.execute(
            "INSERT INTO companion_queue
             (id, device_id, session_id, agent_id, owner_user_id, kind, status,
              payload_json, summary, metadata_only, media_bytes_persisted,
              approval_required, retry_count, idempotency_key, created_at,
              previewed_at, updated_at)
             SELECT ?1, d.id, ?2, ?3, ?4, ?5, 'previewed', ?6, ?7, 1, 0, 1, 0,
                    ?8, ?9, ?9, ?9
             FROM companion_devices d WHERE d.device_id = ?10 AND d.agent_id = ?3",
            params![
                queue_id,
                session.id,
                request.agent_id,
                request.owner_user_id,
                request.payload.kind(),
                payload_json,
                summary,
                idempotency_key,
                now,
                session_device_external_id(&transaction, &session.id)?,
            ],
        )?;
        if inserted != 1 {
            return Err(DatabaseError::Cognitive("companion_queue_invalid"));
        }
        insert_history_tx(
            &transaction,
            HistoryContext {
                device_id: Some(&session.device_id),
                session_id: Some(&session.id),
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                direction: "outgoing",
                kind: request.payload.kind(),
                summary: "Fila offline: prévia criada e aguardando aprovação do Owner",
            },
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&session.device_id),
                session_id: Some(&session.id),
                queue_id: Some(&queue_id),
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "queue_previewed",
                result: "approval_required",
                code: None,
                summary: "Item offline limitado aguardando aprovação explícita",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "queue_preview",
            &idempotency_key,
            &request_json,
            "queue",
            &queue_id,
        )?;
        let item = load_queue_tx(&transaction, &queue_id)?;
        transaction.commit()?;
        Ok(item)
    }

    pub fn approve_companion_queue(
        &self,
        request: CompanionQueueDecisionRequest,
    ) -> Result<CompanionQueueItem, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_session_proof(&request.proof)?;
        validate_reference(&request.queue_id, 128, "companion_queue_invalid")?;
        if !request.approved {
            return Err(DatabaseError::Cognitive("companion_approval_required"));
        }
        self.transition_companion_queue(
            request.agent_id,
            request.owner_user_id,
            request.proof,
            request.queue_id,
            request.idempotency_key,
            request.temporary_chat,
            "queue_approve",
            "queued",
            "queue_approved",
            "Owner aprovou o item; nenhum transporte foi executado",
        )
    }

    pub fn cancel_companion_queue(
        &self,
        request: CompanionQueueActionRequest,
    ) -> Result<CompanionQueueItem, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_session_proof(&request.proof)?;
        self.transition_companion_queue(
            request.agent_id,
            request.owner_user_id,
            request.proof,
            request.queue_id,
            request.idempotency_key,
            request.temporary_chat,
            "queue_cancel",
            "cancelled",
            "queue_cancelled",
            "Item offline cancelado pelo Owner",
        )
    }

    pub fn retry_companion_queue(
        &self,
        request: CompanionQueueActionRequest,
    ) -> Result<CompanionQueueItem, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_session_proof(&request.proof)?;
        validate_reference(&request.queue_id, 128, "companion_queue_invalid")?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "queue_retry",
            &idempotency_key,
            &request_json,
            "queue",
        )? {
            let item = load_queue_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(item);
        }
        let session = authenticate_session_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            "queue_retry",
        )?;
        let item = load_queue_tx(&transaction, &request.queue_id)?;
        ensure_queue_owner(&item, &session, &request.agent_id, &request.owner_user_id)?;
        if !matches!(
            item.status,
            CompanionQueueStatus::Cancelled | CompanionQueueStatus::Failed
        ) {
            return Err(DatabaseError::Cognitive("companion_queue_state_invalid"));
        }
        if item.retry_count >= MAX_COMPANION_RETRY_COUNT {
            return Err(DatabaseError::Cognitive("companion_retry_limit"));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE companion_queue
             SET status = 'previewed', error_code = NULL, approved_at = NULL,
                 cancelled_at = NULL, retry_count = retry_count + 1, updated_at = ?1
             WHERE id = ?2",
            params![now, item.id],
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&session.device_id),
                session_id: Some(&session.id),
                queue_id: Some(&item.id),
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "queue_retried",
                result: "approval_required",
                code: None,
                summary: "Retry devolvido à prévia para nova aprovação do Owner",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "queue_retry",
            &idempotency_key,
            &request_json,
            "queue",
            &item.id,
        )?;
        let item = load_queue_tx(&transaction, &item.id)?;
        transaction.commit()?;
        Ok(item)
    }

    #[allow(clippy::too_many_arguments)]
    fn transition_companion_queue(
        &self,
        agent_id: String,
        owner_user_id: String,
        proof: CompanionSessionProof,
        queue_id: String,
        idempotency_key_input: String,
        temporary_chat: bool,
        operation: &str,
        target_status: &str,
        event: &str,
        summary: &str,
    ) -> Result<CompanionQueueItem, DatabaseError> {
        validate_reference(&queue_id, 128, "companion_queue_invalid")?;
        let request = CompanionQueueActionRequest {
            agent_id,
            owner_user_id,
            proof,
            queue_id,
            idempotency_key: idempotency_key_input,
            temporary_chat,
        };
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            operation,
            &idempotency_key,
            &request_json,
            "queue",
        )? {
            let item = load_queue_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(item);
        }
        let session = authenticate_session_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            operation,
        )?;
        let item = load_queue_tx(&transaction, &request.queue_id)?;
        ensure_queue_owner(&item, &session, &request.agent_id, &request.owner_user_id)?;
        let valid_state = match target_status {
            "queued" => item.status == CompanionQueueStatus::Previewed,
            "cancelled" => matches!(
                item.status,
                CompanionQueueStatus::Previewed
                    | CompanionQueueStatus::Queued
                    | CompanionQueueStatus::Failed
            ),
            _ => false,
        };
        if !valid_state {
            return Err(DatabaseError::Cognitive("companion_queue_state_invalid"));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE companion_queue
             SET status = ?1, approved_at = CASE WHEN ?1 = 'queued' THEN ?2 ELSE approved_at END,
                 cancelled_at = CASE WHEN ?1 = 'cancelled' THEN ?2 ELSE cancelled_at END,
                 error_code = CASE WHEN ?1 = 'cancelled' THEN 'companion_cancelled' ELSE error_code END,
                 updated_at = ?2
             WHERE id = ?3",
            params![target_status, now, item.id],
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&session.device_id),
                session_id: Some(&session.id),
                queue_id: Some(&item.id),
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event,
                result: target_status,
                code: None,
                summary,
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            operation,
            &idempotency_key,
            &request_json,
            "queue",
            &item.id,
        )?;
        let item = load_queue_tx(&transaction, &item.id)?;
        transaction.commit()?;
        Ok(item)
    }

    pub fn list_companion_history(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionHistoryRecord>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT h.id, d.device_id, h.session_id, h.agent_id, h.owner_user_id, h.direction,
                    h.kind, h.summary, h.metadata_only, h.media_bytes_persisted, h.created_at
             FROM companion_history h
             LEFT JOIN companion_devices d ON d.id = h.device_id
             WHERE h.agent_id = ?1
             ORDER BY h.created_at DESC, h.id DESC LIMIT ?2",
        )?;
        let records = statement
            .query_map(params![agent_id, MAX_COMPANION_HISTORY_ROWS], |row| {
                Ok(CompanionHistoryRecord {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    session_id: row.get(2)?,
                    agent_id: row.get(3)?,
                    owner_user_id: row.get(4)?,
                    direction: row.get(5)?,
                    kind: row.get(6)?,
                    summary: row.get(7)?,
                    metadata_only: row.get(8)?,
                    media_bytes_persisted: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub fn list_companion_audit(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionAuditRecord>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT a.id, d.device_id, a.session_id, a.queue_id, a.agent_id, a.owner_user_id,
                    a.event, a.result, a.code, a.details_json, a.created_at
             FROM companion_audit_log a
             LEFT JOIN companion_devices d ON d.id = a.device_id
             WHERE a.agent_id = ?1
             ORDER BY a.created_at DESC, a.id DESC LIMIT ?2",
        )?;
        let records = statement
            .query_map(params![agent_id, MAX_COMPANION_AUDIT_ROWS], |row| {
                let details_json: String = row.get(9)?;
                let summary = serde_json::from_str::<Value>(&details_json)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "Evento de companion".to_owned());
                Ok(CompanionAuditRecord {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    session_id: row.get(2)?,
                    queue_id: row.get(3)?,
                    agent_id: row.get(4)?,
                    owner_user_id: row.get(5)?,
                    event: row.get(6)?,
                    result: row.get(7)?,
                    code: row.get(8)?,
                    summary,
                    created_at: row.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub fn list_companion_key_rotations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionKeyRotation>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT r.id, d.device_id, r.agent_id, r.owner_user_id, r.old_fingerprint,
                    r.new_fingerprint, r.old_key_version, r.new_key_version, r.nonce_metadata,
                    r.status, r.reason, r.created_at
             FROM companion_key_rotations r
             JOIN companion_devices d ON d.id = r.device_id
             WHERE r.agent_id = ?1
             ORDER BY r.created_at DESC, r.id DESC LIMIT 32",
        )?;
        let records = statement
            .query_map(params![agent_id], |row| {
                Ok(CompanionKeyRotation {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    agent_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    old_fingerprint: row.get(4)?,
                    new_fingerprint: row.get(5)?,
                    old_key_version: row.get(6)?,
                    new_key_version: row.get(7)?,
                    nonce_metadata: row.get(8)?,
                    status: row.get(9)?,
                    reason: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub fn list_companion_revocations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CompanionRevocation>, DatabaseError> {
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT r.id, d.device_id, r.agent_id, r.owner_user_id, r.previous_status,
                    r.reason, r.created_at
             FROM companion_revocations r
             JOIN companion_devices d ON d.id = r.device_id
             WHERE r.agent_id = ?1
             ORDER BY r.created_at DESC, r.id DESC LIMIT 32",
        )?;
        let records = statement
            .query_map(params![agent_id], |row| {
                Ok(CompanionRevocation {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    agent_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    previous_status: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub fn rotate_companion_key(
        &self,
        request: CompanionDeviceActionRequest,
    ) -> Result<CompanionKeyRotation, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_reference(&request.device_id, 96, "companion_device_invalid")?;
        let reason = validate_text(&request.reason, 512)?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "key_rotate",
            &idempotency_key,
            &request_json,
            "rotation",
        )? {
            let rotation = load_rotation_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(rotation);
        }
        let device = load_device_by_external_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.device_id,
        )?;
        if device.status == CompanionDeviceStatus::Revoked {
            return Err(DatabaseError::Cognitive("companion_device_revoked"));
        }
        if device.status != CompanionDeviceStatus::Paired {
            return Err(DatabaseError::Cognitive("companion_key_rotation_invalid"));
        }
        let now = now_millis();
        let new_key_version = device.key_version + 1;
        let new_fingerprint = format!("{}-key-v{}", COMPANION_FIXTURE_FINGERPRINT, new_key_version);
        let new_nonce = format!(
            "{}-nonce-v{}",
            COMPANION_FIXTURE_PAIRING_NONCE, new_key_version
        );
        transaction.execute(
            "UPDATE companion_devices
             SET fingerprint = ?1, pairing_nonce_metadata = ?2, key_version = ?3,
                 last_seen_at = ?4, updated_at = ?4
             WHERE id = ?5",
            params![new_fingerprint, new_nonce, new_key_version, now, device.id],
        )?;
        transaction.execute(
            "UPDATE companion_sessions
             SET status = 'disconnected', disconnected_at = ?1, updated_at = ?1
             WHERE device_id = ?2 AND status = 'connected'",
            params![now, device.id],
        )?;
        let rotation_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO companion_key_rotations
             (id, device_id, agent_id, owner_user_id, old_fingerprint, new_fingerprint,
              old_key_version, new_key_version, nonce_metadata, status, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'completed', ?10, ?11)",
            params![
                rotation_id,
                device.id,
                request.agent_id,
                request.owner_user_id,
                device.fingerprint,
                new_fingerprint,
                device.key_version,
                new_key_version,
                new_nonce,
                reason,
                now,
            ],
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&device.id),
                session_id: None,
                queue_id: None,
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "key_rotated",
                result: "completed",
                code: None,
                summary: "Metadados de chave sintética rotacionados; sessões exigem reconexão",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "key_rotate",
            &idempotency_key,
            &request_json,
            "rotation",
            &rotation_id,
        )?;
        let rotation = load_rotation_tx(&transaction, &rotation_id)?;
        transaction.commit()?;
        Ok(rotation)
    }

    pub fn revoke_companion_device(
        &self,
        request: CompanionDeviceActionRequest,
    ) -> Result<CompanionRevocation, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        validate_reference(&request.device_id, 96, "companion_device_invalid")?;
        let reason = validate_text(&request.reason, 512)?;
        let request_json = request_json(&request)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_mode_tx(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_state_tx(&transaction)?;
        if let Some(result_id) = existing_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "device_revoke",
            &idempotency_key,
            &request_json,
            "revocation",
        )? {
            let revocation = load_revocation_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(revocation);
        }
        let device = load_device_by_external_tx(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.device_id,
        )?;
        if device.status == CompanionDeviceStatus::Revoked {
            return Err(DatabaseError::Cognitive("companion_device_revoked"));
        }
        let previous_status = device.status.as_str();
        let now = now_millis();
        transaction.execute(
            "UPDATE companion_devices SET status = 'revoked', revoked_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, device.id],
        )?;
        transaction.execute(
            "UPDATE companion_sessions
             SET status = 'revoked', disconnected_at = ?1, updated_at = ?1
             WHERE device_id = ?2 AND status IN ('connected', 'disconnected')",
            params![now, device.id],
        )?;
        transaction.execute(
            "UPDATE companion_queue
             SET status = 'cancelled', error_code = 'companion_device_revoked',
                 cancelled_at = ?1, updated_at = ?1
             WHERE device_id = ?2 AND status IN ('previewed', 'queued', 'failed')",
            params![now, device.id],
        )?;
        let revocation_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO companion_revocations
             (id, device_id, agent_id, owner_user_id, previous_status, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                revocation_id,
                device.id,
                request.agent_id,
                request.owner_user_id,
                previous_status,
                reason,
                now,
            ],
        )?;
        insert_audit_tx(
            &transaction,
            AuditContext {
                device_id: Some(&device.id),
                session_id: None,
                queue_id: None,
                agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "device_revoked",
                result: "revoked",
                code: None,
                summary: "Dispositivo sintético revogado; sessões e fila foram fechadas",
            },
        )?;
        insert_idempotency_tx(
            &transaction,
            &request.owner_user_id,
            "device_revoke",
            &idempotency_key,
            &request_json,
            "revocation",
            &revocation_id,
        )?;
        let revocation = load_revocation_tx(&transaction, &revocation_id)?;
        transaction.commit()?;
        Ok(revocation)
    }
}

impl CompanionDeviceStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::PairingRequested => "pairing_requested",
            Self::Paired => "paired",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }
}

fn device_status_from_str(value: &str) -> rusqlite::Result<CompanionDeviceStatus> {
    match value {
        "pairing_requested" => Ok(CompanionDeviceStatus::PairingRequested),
        "paired" => Ok(CompanionDeviceStatus::Paired),
        "expired" => Ok(CompanionDeviceStatus::Expired),
        "revoked" => Ok(CompanionDeviceStatus::Revoked),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn session_status_from_str(value: &str) -> rusqlite::Result<CompanionSessionStatus> {
    match value {
        "connected" => Ok(CompanionSessionStatus::Connected),
        "disconnected" => Ok(CompanionSessionStatus::Disconnected),
        "revoked" => Ok(CompanionSessionStatus::Revoked),
        "expired" => Ok(CompanionSessionStatus::Expired),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn queue_status_from_str(value: &str) -> rusqlite::Result<CompanionQueueStatus> {
    match value {
        "previewed" => Ok(CompanionQueueStatus::Previewed),
        "queued" => Ok(CompanionQueueStatus::Queued),
        "cancelled" => Ok(CompanionQueueStatus::Cancelled),
        "failed" => Ok(CompanionQueueStatus::Failed),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn protocol_info(app_version: &str, protocol_version: i64) -> CompanionProtocolInfo {
    CompanionProtocolInfo {
        schema_version: 1,
        protocol_version,
        min_protocol_version: COMPANION_MIN_PROTOCOL_VERSION,
        platform: CompanionPlatform::Android,
        app_version: app_version.to_owned(),
        transport: "tauri_command_fixture".to_owned(),
        network_listener: false,
        standalone_fallback: true,
    }
}

fn companion_device_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CompanionDevice> {
    let protocol_version: i64 = row.get(6)?;
    let app_version: String = row.get(5)?;
    let status = device_status_from_str(&row.get::<_, String>(7)?)?;
    Ok(CompanionDevice {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        owner_user_id: row.get(2)?,
        device_id: row.get(3)?,
        platform: CompanionPlatform::Android,
        app_version: app_version.clone(),
        protocol_version,
        status,
        fingerprint: row.get(8)?,
        pairing_nonce_metadata: row.get(9)?,
        key_version: row.get(10)?,
        pairing_expires_at: row.get(11)?,
        paired_at: row.get(12)?,
        revoked_at: row.get(13)?,
        last_seen_at: row.get(14)?,
        compatible: (COMPANION_MIN_PROTOCOL_VERSION..=COMPANION_PROTOCOL_VERSION)
            .contains(&protocol_version)
            && app_version == COMPANION_FIXTURE_APP_VERSION,
        standalone_fallback: true,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn load_device_connection(
    connection: &Connection,
    device_id: &str,
) -> Result<CompanionDevice, DatabaseError> {
    connection
        .query_row(
            "SELECT id, agent_id, owner_user_id, device_id, platform, app_version,
                    protocol_version, status, fingerprint, pairing_nonce_metadata,
                    key_version, pairing_expires_at, paired_at, revoked_at,
                    last_seen_at, created_at, updated_at
             FROM companion_devices WHERE id = ?1",
            params![device_id],
            companion_device_from_row,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_device_not_found"))
}

fn load_device_tx(
    transaction: &Transaction<'_>,
    device_id: &str,
) -> Result<CompanionDevice, DatabaseError> {
    load_device_connection(transaction, device_id)
}

fn ensure_agent_owner_connection(
    connection: &Connection,
    agent_id: &str,
) -> Result<(), DatabaseError> {
    let valid: bool = connection.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM users u JOIN agents a ON a.owner_user_id = u.id
           WHERE a.id = ?1 AND u.role = 'owner'
         )",
        params![agent_id],
        |row| row.get(0),
    )?;
    if valid {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("companion_agent_invalid"))
    }
}

fn ensure_owner_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    let valid: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM users u JOIN agents a ON a.owner_user_id = u.id
           WHERE u.id = ?1 AND u.role = 'owner'
             AND a.id = ?2 AND a.owner_user_id = ?1
         )",
        params![owner_user_id, agent_id],
        |row| row.get(0),
    )?;
    if valid {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("companion_owner_required"))
    }
}

fn ensure_owner_mode_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    ensure_owner_tx(transaction, agent_id, owner_user_id)?;
    let safe_mode = transaction
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = 'safe_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some_and(|value| value == "true");
    if safe_mode {
        return Err(DatabaseError::Cognitive("companion_blocked_safe_mode"));
    }
    let (mode, suspended): (String, bool) = transaction
        .query_row(
            "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id = ?1",
            params![agent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_agent_invalid"))?;
    if suspended {
        return Err(DatabaseError::Cognitive("companion_blocked_suspended"));
    }
    if mode == "safe" {
        return Err(DatabaseError::Cognitive("companion_blocked_safe_mode"));
    }
    Ok(())
}

fn ensure_not_temporary(temporary_chat: bool) -> Result<(), DatabaseError> {
    if temporary_chat {
        Err(DatabaseError::Cognitive("companion_blocked_temporary"))
    } else {
        Ok(())
    }
}

fn validate_text(value: &str, maximum: usize) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > maximum
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(DatabaseError::Cognitive("companion_text_invalid"));
    }
    Ok(value.to_owned())
}

fn validate_reference(
    value: &str,
    maximum: usize,
    code: &'static str,
) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > maximum
        || value.contains("..")
        || value.contains('\\')
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._/-".contains(character)))
    {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(value.to_owned())
}

fn validate_file_name(value: &str) -> Result<(), DatabaseError> {
    let value = validate_text(value, 192)?;
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
    {
        return Err(DatabaseError::Cognitive("companion_payload_invalid"));
    }
    Ok(())
}

fn validate_mime(value: &str, prefix: &str) -> Result<(), DatabaseError> {
    let value = validate_reference(value, 96, "companion_payload_invalid")?;
    if !value.contains('/') || (!prefix.is_empty() && !value.starts_with(prefix)) {
        return Err(DatabaseError::Cognitive("companion_payload_invalid"));
    }
    Ok(())
}

fn bounded_summary(value: &str) -> String {
    if value.len() <= MAX_COMPANION_SUMMARY_BYTES {
        return value.to_owned();
    }
    let mut result = String::new();
    for character in value.chars() {
        if result.len() + character.len_utf8() + 3 > MAX_COMPANION_SUMMARY_BYTES {
            break;
        }
        result.push(character);
    }
    result.push('…');
    result
}

fn valid_idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        return Err(DatabaseError::Cognitive("companion_idempotency_invalid"));
    }
    Ok(value.to_owned())
}

fn request_json<T: Serialize>(request: &T) -> Result<String, DatabaseError> {
    let value = serde_json::to_string(request).map_err(|_| DatabaseError::Unavailable)?;
    if value.len() > MAX_COMPANION_REQUEST_BYTES {
        Err(DatabaseError::Cognitive("companion_request_oversized"))
    } else {
        Ok(value)
    }
}

fn validate_pairing_fixture(request: &CompanionPairingRequest) -> Result<(), DatabaseError> {
    if request.platform != CompanionPlatform::Android
        || request.device_id != COMPANION_FIXTURE_DEVICE_ID
        || request.fingerprint != COMPANION_FIXTURE_FINGERPRINT
        || request.pairing_nonce_metadata != COMPANION_FIXTURE_PAIRING_NONCE
        || request.app_version != COMPANION_FIXTURE_APP_VERSION
    {
        return Err(DatabaseError::Cognitive("companion_fixture_invalid"));
    }
    validate_reference(&request.device_id, 96, "companion_device_invalid")?;
    validate_reference(&request.fingerprint, 192, "companion_fingerprint_invalid")?;
    validate_reference(
        &request.pairing_nonce_metadata,
        192,
        "companion_nonce_invalid",
    )?;
    ensure_compatible(request.protocol_version, &request.app_version)
}

fn ensure_compatible(protocol_version: i64, app_version: &str) -> Result<(), DatabaseError> {
    if !(COMPANION_MIN_PROTOCOL_VERSION..=COMPANION_PROTOCOL_VERSION).contains(&protocol_version)
        || app_version != COMPANION_FIXTURE_APP_VERSION
    {
        Err(DatabaseError::Cognitive("companion_protocol_incompatible"))
    } else {
        Ok(())
    }
}

fn validate_session_proof(proof: &CompanionSessionProof) -> Result<(), DatabaseError> {
    validate_reference(&proof.session_id, 128, "companion_session_invalid")?;
    validate_reference(&proof.device_id, 96, "companion_device_invalid")?;
    validate_reference(
        &proof.session_nonce_metadata,
        192,
        "companion_nonce_invalid",
    )?;
    validate_reference(&proof.key_fingerprint, 192, "companion_fingerprint_invalid")?;
    validate_reference(
        &proof.message_nonce_metadata,
        192,
        "companion_nonce_invalid",
    )?;
    if proof.replay_counter < 1 {
        return Err(DatabaseError::Cognitive("companion_replay_rejected"));
    }
    ensure_compatible(proof.protocol_version, &proof.app_version)
}

struct AuditContext<'a> {
    device_id: Option<&'a str>,
    session_id: Option<&'a str>,
    queue_id: Option<&'a str>,
    agent_id: &'a str,
    owner_user_id: &'a str,
    event: &'a str,
    result: &'a str,
    code: Option<&'a str>,
    summary: &'a str,
}

fn insert_audit_tx(
    transaction: &Transaction<'_>,
    context: AuditContext<'_>,
) -> Result<(), DatabaseError> {
    if context.summary.is_empty() || context.summary.len() > MAX_COMPANION_SUMMARY_BYTES {
        return Err(DatabaseError::Cognitive("companion_audit_oversized"));
    }
    let details_json = serde_json::to_string(&json!({
        "summary": context.summary,
        "metadataOnly": true,
        "mediaBytesPersisted": false,
    }))
    .map_err(|_| DatabaseError::Unavailable)?;
    if details_json.len() > MAX_COMPANION_METADATA_BYTES {
        return Err(DatabaseError::Cognitive("companion_audit_oversized"));
    }
    let now = now_millis();
    transaction.execute(
        "DELETE FROM companion_audit_log WHERE created_at < ?1",
        params![now - 30 * 24 * 60 * 60 * 1_000_i64],
    )?;
    transaction.execute(
        "INSERT INTO companion_audit_log
         (id, device_id, session_id, queue_id, agent_id, owner_user_id, event, result,
          code, details_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            Uuid::now_v7().to_string(),
            context.device_id,
            context.session_id,
            context.queue_id,
            context.agent_id,
            context.owner_user_id,
            context.event,
            context.result,
            context.code,
            details_json,
            now,
        ],
    )?;
    Ok(())
}

struct HistoryContext<'a> {
    device_id: Option<&'a str>,
    session_id: Option<&'a str>,
    agent_id: &'a str,
    owner_user_id: &'a str,
    direction: &'a str,
    kind: &'a str,
    summary: &'a str,
}

fn insert_history_tx(
    transaction: &Transaction<'_>,
    context: HistoryContext<'_>,
) -> Result<(), DatabaseError> {
    if context.summary.is_empty() || context.summary.len() > MAX_COMPANION_SUMMARY_BYTES {
        return Err(DatabaseError::Cognitive("companion_history_oversized"));
    }
    let metadata_json = serde_json::to_string(&json!({
        "summary": context.summary,
        "metadataOnly": true,
        "mediaBytesPersisted": false,
    }))
    .map_err(|_| DatabaseError::Unavailable)?;
    if metadata_json.len() > MAX_COMPANION_METADATA_BYTES {
        return Err(DatabaseError::Cognitive("companion_history_oversized"));
    }
    transaction.execute(
        "INSERT INTO companion_history
         (id, device_id, session_id, agent_id, owner_user_id, direction, kind, summary,
          metadata_json, metadata_only, media_bytes_persisted, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, 0, ?10)",
        params![
            Uuid::now_v7().to_string(),
            context.device_id,
            context.session_id,
            context.agent_id,
            context.owner_user_id,
            context.direction,
            context.kind,
            bounded_summary(context.summary),
            metadata_json,
            now_millis(),
        ],
    )?;
    transaction.execute(
        "DELETE FROM companion_history WHERE id IN (
           SELECT id FROM companion_history WHERE agent_id = ?1
           ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?2
         )",
        params![context.agent_id, MAX_COMPANION_HISTORY_ROWS],
    )?;
    Ok(())
}

fn existing_idempotency_tx(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    result_kind: &str,
) -> Result<Option<String>, DatabaseError> {
    let existing = transaction
        .query_row(
            "SELECT request_json, result_kind, result_id FROM companion_idempotency
             WHERE owner_user_id = ?1 AND operation = ?2 AND idempotency_key = ?3",
            params![owner_user_id, operation, idempotency_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((existing_request, existing_kind, result_id)) = existing else {
        return Ok(None);
    };
    if existing_request != request_json || existing_kind != result_kind {
        return Err(DatabaseError::Cognitive("companion_idempotency_conflict"));
    }
    Ok(Some(result_id))
}

fn insert_idempotency_tx(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    result_kind: &str,
    result_id: &str,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO companion_idempotency
         (owner_user_id, operation, idempotency_key, request_json, result_kind, result_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            owner_user_id,
            operation,
            idempotency_key,
            request_json,
            result_kind,
            result_id,
            now_millis(),
        ],
    )?;
    Ok(())
}

fn expire_state_tx(transaction: &Transaction<'_>) -> Result<(), DatabaseError> {
    let now = now_millis();
    transaction.execute(
        "UPDATE companion_devices SET status = 'expired', updated_at = ?1
         WHERE status = 'pairing_requested' AND pairing_expires_at IS NOT NULL
           AND pairing_expires_at < ?1",
        params![now],
    )?;
    transaction.execute(
        "UPDATE companion_sessions SET status = 'expired', disconnected_at = ?1, updated_at = ?1
         WHERE status IN ('connected', 'disconnected') AND last_seen_at < ?2",
        params![now, now - COMPANION_SESSION_TTL_MS],
    )?;
    Ok(())
}

fn load_session_connection(
    connection: &Connection,
    session_id: &str,
) -> Result<CompanionSession, DatabaseError> {
    let row = connection
        .query_row(
            "SELECT s.id, d.device_id, s.agent_id, s.owner_user_id, s.status,
                    s.protocol_version, s.app_version, s.negotiated_protocol_version,
                    s.key_fingerprint, s.session_nonce_metadata, s.last_replay_counter,
                    s.connected_at, s.last_seen_at, s.disconnected_at, s.updated_at
             FROM companion_sessions s
             JOIN companion_devices d ON d.id = s.device_id
             WHERE s.id = ?1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    session_status_from_str(&row.get::<_, String>(4)?)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, i64>(14)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_session_not_found"))?;
    let guard = connection
        .query_row(
            "SELECT message_nonce_metadata, replay_counter FROM companion_replay_guards
             WHERE session_id = ?1 ORDER BY replay_counter DESC, created_at DESC LIMIT 1",
            params![session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_session_invalid"))?;
    let protocol = protocol_info(&row.6, row.5);
    let handshake = CompanionProtocolMessage {
        schema_version: 1,
        protocol_version: row.5,
        message_id: format!("session-handshake:{}", row.0),
        device_id: row.1.clone(),
        platform: CompanionPlatform::Android,
        app_version: row.6.clone(),
        kind: CompanionMessageKind::Session,
        session_id: Some(row.0.clone()),
        nonce_metadata: guard.0,
        replay_counter: guard.1,
        payload_kind: "session".to_owned(),
    };
    Ok(CompanionSession {
        id: row.0,
        device_id: row.1,
        agent_id: row.2,
        owner_user_id: row.3,
        status: row.4,
        protocol_version: row.5,
        app_version: row.6,
        negotiated_protocol_version: row.7,
        key_fingerprint: row.8,
        session_nonce_metadata: row.9,
        last_replay_counter: row.10,
        connected_at: row.11,
        last_seen_at: row.12,
        disconnected_at: row.13,
        protocol,
        handshake,
        updated_at: row.14,
    })
}

fn load_session_tx(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<CompanionSession, DatabaseError> {
    load_session_connection(transaction, session_id)
}

fn load_device_by_external_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
    external_device_id: &str,
) -> Result<CompanionDevice, DatabaseError> {
    let internal_id = transaction
        .query_row(
            "SELECT id FROM companion_devices
             WHERE agent_id = ?1 AND owner_user_id = ?2 AND device_id = ?3",
            params![agent_id, owner_user_id, external_device_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_device_not_found"))?;
    load_device_tx(transaction, &internal_id)
}

fn session_device_external_id(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<String, DatabaseError> {
    transaction
        .query_row(
            "SELECT d.device_id FROM companion_sessions s
             JOIN companion_devices d ON d.id = s.device_id WHERE s.id = ?1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_session_not_found"))
}

fn ensure_replay_nonce_available(
    transaction: &Transaction<'_>,
    device_id: &str,
    message_nonce_metadata: &str,
) -> Result<(), DatabaseError> {
    let used: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM companion_replay_guards
           WHERE device_id = ?1 AND message_nonce_metadata = ?2
         )",
        params![device_id, message_nonce_metadata],
        |row| row.get(0),
    )?;
    if used {
        Err(DatabaseError::Cognitive("companion_replay_rejected"))
    } else {
        Ok(())
    }
}

fn ensure_replay_counter_fresh(
    transaction: &Transaction<'_>,
    device_id: &str,
    replay_counter: i64,
) -> Result<(), DatabaseError> {
    let last_counter: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(replay_counter), 0) FROM companion_replay_guards
         WHERE device_id = ?1",
        params![device_id],
        |row| row.get(0),
    )?;
    if replay_counter <= last_counter {
        Err(DatabaseError::Cognitive("companion_replay_rejected"))
    } else {
        Ok(())
    }
}

fn insert_replay_guard_tx(
    transaction: &Transaction<'_>,
    device_id: &str,
    session_id: Option<&str>,
    message_nonce_metadata: &str,
    replay_counter: i64,
    message_kind: &str,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO companion_replay_guards
         (id, device_id, session_id, message_nonce_metadata, replay_counter, message_kind, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::now_v7().to_string(),
            device_id,
            session_id,
            message_nonce_metadata,
            replay_counter,
            message_kind,
            now_millis(),
        ],
    )?;
    Ok(())
}

fn authenticate_session_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
    proof: &CompanionSessionProof,
    message_kind: &str,
) -> Result<CompanionSession, DatabaseError> {
    let session = load_session_tx(transaction, &proof.session_id)?;
    if session.agent_id != agent_id
        || session.owner_user_id != owner_user_id
        || session.device_id != proof.device_id
    {
        return Err(DatabaseError::OwnershipMismatch);
    }
    if matches!(
        session.status,
        CompanionSessionStatus::Revoked | CompanionSessionStatus::Expired
    ) {
        return Err(DatabaseError::Cognitive("companion_session_unavailable"));
    }
    if session.protocol_version != proof.protocol_version
        || session.app_version != proof.app_version
        || session.key_fingerprint != proof.key_fingerprint
        || session.session_nonce_metadata != proof.session_nonce_metadata
    {
        return Err(DatabaseError::Cognitive("companion_authentication_failed"));
    }
    let device =
        load_device_by_external_tx(transaction, agent_id, owner_user_id, &proof.device_id)?;
    if device.status == CompanionDeviceStatus::Revoked {
        return Err(DatabaseError::Cognitive("companion_device_revoked"));
    }
    if device.status != CompanionDeviceStatus::Paired || device.fingerprint != proof.key_fingerprint
    {
        return Err(DatabaseError::Cognitive("companion_authentication_failed"));
    }
    ensure_replay_nonce_available(transaction, &device.id, &proof.message_nonce_metadata)?;
    ensure_replay_counter_fresh(transaction, &device.id, proof.replay_counter)?;
    let now = now_millis();
    transaction.execute(
        "UPDATE companion_sessions
         SET status = 'connected', last_replay_counter = ?1, last_seen_at = ?2,
             disconnected_at = NULL, updated_at = ?2 WHERE id = ?3",
        params![proof.replay_counter, now, proof.session_id],
    )?;
    transaction.execute(
        "UPDATE companion_devices SET last_seen_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, device.id],
    )?;
    insert_replay_guard_tx(
        transaction,
        &device.id,
        Some(&proof.session_id),
        &proof.message_nonce_metadata,
        proof.replay_counter,
        message_kind,
    )?;
    load_session_tx(transaction, &proof.session_id)
}

fn load_queue_connection(
    connection: &Connection,
    queue_id: &str,
) -> Result<CompanionQueueItem, DatabaseError> {
    let row = connection
        .query_row(
            "SELECT q.id, d.device_id, q.session_id, q.agent_id, q.owner_user_id,
                    q.kind, q.status, q.payload_json, q.summary, q.metadata_only,
                    q.media_bytes_persisted, q.approval_required, q.retry_count,
                    q.error_code, q.created_at, q.previewed_at, q.approved_at,
                    q.cancelled_at, q.updated_at
             FROM companion_queue q JOIN companion_devices d ON d.id = q.device_id
             WHERE q.id = ?1",
            params![queue_id],
            |row| {
                let payload_json: String = row.get(7)?;
                let payload = serde_json::from_str::<CompanionQueuePayload>(&payload_json)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(CompanionQueueItem {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    session_id: row.get(2)?,
                    agent_id: row.get(3)?,
                    owner_user_id: row.get(4)?,
                    kind: row.get(5)?,
                    status: queue_status_from_str(&row.get::<_, String>(6)?)?,
                    payload,
                    summary: row.get(8)?,
                    metadata_only: row.get(9)?,
                    media_bytes_persisted: row.get(10)?,
                    approval_required: row.get(11)?,
                    retry_count: row.get(12)?,
                    error_code: row.get(13)?,
                    created_at: row.get(14)?,
                    previewed_at: row.get(15)?,
                    approved_at: row.get(16)?,
                    cancelled_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_queue_not_found"))?;
    if row.payload.kind() != row.kind {
        return Err(DatabaseError::Cognitive("companion_queue_invalid"));
    }
    Ok(row)
}

fn load_queue_tx(
    transaction: &Transaction<'_>,
    queue_id: &str,
) -> Result<CompanionQueueItem, DatabaseError> {
    load_queue_connection(transaction, queue_id)
}

fn ensure_queue_owner(
    item: &CompanionQueueItem,
    session: &CompanionSession,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    if item.agent_id != agent_id
        || item.owner_user_id != owner_user_id
        || item.session_id != session.id
        || item.device_id != session.device_id
    {
        Err(DatabaseError::OwnershipMismatch)
    } else {
        Ok(())
    }
}

fn load_rotation_tx(
    transaction: &Transaction<'_>,
    rotation_id: &str,
) -> Result<CompanionKeyRotation, DatabaseError> {
    transaction
        .query_row(
            "SELECT r.id, d.device_id, r.agent_id, r.owner_user_id, r.old_fingerprint,
                    r.new_fingerprint, r.old_key_version, r.new_key_version, r.nonce_metadata,
                    r.status, r.reason, r.created_at
             FROM companion_key_rotations r JOIN companion_devices d ON d.id = r.device_id
             WHERE r.id = ?1",
            params![rotation_id],
            |row| {
                Ok(CompanionKeyRotation {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    agent_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    old_fingerprint: row.get(4)?,
                    new_fingerprint: row.get(5)?,
                    old_key_version: row.get(6)?,
                    new_key_version: row.get(7)?,
                    nonce_metadata: row.get(8)?,
                    status: row.get(9)?,
                    reason: row.get(10)?,
                    created_at: row.get(11)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_rotation_not_found"))
}

fn load_revocation_tx(
    transaction: &Transaction<'_>,
    revocation_id: &str,
) -> Result<CompanionRevocation, DatabaseError> {
    transaction
        .query_row(
            "SELECT r.id, d.device_id, r.agent_id, r.owner_user_id, r.previous_status,
                    r.reason, r.created_at
             FROM companion_revocations r JOIN companion_devices d ON d.id = r.device_id
             WHERE r.id = ?1",
            params![revocation_id],
            |row| {
                Ok(CompanionRevocation {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    agent_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    previous_status: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("companion_revocation_not_found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Database, ASTRA_ID, OWNER_ID};

    fn database() -> (Database, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("aip-companion-test-{}", Uuid::now_v7()));
        let database = Database::initialize(&path).expect("database should initialize");
        (database, path)
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
    }

    fn pairing_request(idempotency_key: &str) -> CompanionPairingRequest {
        CompanionPairingRequest {
            agent_id: ASTRA_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            device_id: COMPANION_FIXTURE_DEVICE_ID.to_owned(),
            platform: CompanionPlatform::Android,
            app_version: COMPANION_FIXTURE_APP_VERSION.to_owned(),
            protocol_version: COMPANION_PROTOCOL_VERSION,
            fingerprint: COMPANION_FIXTURE_FINGERPRINT.to_owned(),
            pairing_nonce_metadata: COMPANION_FIXTURE_PAIRING_NONCE.to_owned(),
            idempotency_key: idempotency_key.to_owned(),
            temporary_chat: false,
        }
    }

    fn connect_request(device: &CompanionDevice) -> CompanionSessionRequest {
        CompanionSessionRequest {
            agent_id: ASTRA_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            device_id: device.device_id.clone(),
            app_version: device.app_version.clone(),
            protocol_version: device.protocol_version,
            fingerprint: device.fingerprint.clone(),
            pairing_nonce_metadata: device.pairing_nonce_metadata.clone(),
            message_nonce_metadata: "fixture:message/connect-1".to_owned(),
            replay_counter: 1,
            idempotency_key: "session-connect-1".to_owned(),
            temporary_chat: false,
        }
    }

    fn proof(
        session: &CompanionSession,
        replay_counter: i64,
        suffix: &str,
    ) -> CompanionSessionProof {
        CompanionSessionProof {
            session_id: session.id.clone(),
            device_id: session.device_id.clone(),
            session_nonce_metadata: session.session_nonce_metadata.clone(),
            key_fingerprint: session.key_fingerprint.clone(),
            app_version: session.app_version.clone(),
            protocol_version: session.protocol_version,
            message_nonce_metadata: format!("fixture:message/{suffix}"),
            replay_counter,
        }
    }

    #[test]
    fn synthetic_pairing_replay_queue_rotation_and_revocation_are_bounded() {
        let (database, path) = database();
        let requested = database
            .start_companion_pairing(pairing_request("pair-start-1"))
            .expect("pairing should be requested");
        assert_eq!(requested.status, CompanionDeviceStatus::PairingRequested);
        assert!(requested.compatible);

        let paired = database
            .confirm_companion_pairing(CompanionPairingConfirmationRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                device_id: requested.device_id.clone(),
                fingerprint: requested.fingerprint.clone(),
                pairing_nonce_metadata: requested.pairing_nonce_metadata.clone(),
                confirmed: true,
                idempotency_key: "pair-confirm-1".to_owned(),
                temporary_chat: false,
            })
            .expect("pairing should be confirmed");
        let session = database
            .connect_companion_session(connect_request(&paired))
            .expect("session should connect");
        assert!(!session.protocol.network_listener);
        assert!(session.protocol.standalone_fallback);

        let preview = database
            .preview_companion_queue(CompanionQueuePreviewRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                proof: proof(&session, 2, "preview-1"),
                payload: CompanionQueuePayload::Text {
                    text: "mensagem fixture".to_owned(),
                },
                idempotency_key: "queue-preview-1".to_owned(),
                temporary_chat: false,
            })
            .expect("queue preview should be created");
        assert_eq!(preview.status, CompanionQueueStatus::Previewed);
        assert!(preview.metadata_only);
        assert!(!preview.media_bytes_persisted);

        let session = database
            .list_companion_sessions(ASTRA_ID)
            .unwrap()
            .remove(0);
        let queued = database
            .approve_companion_queue(CompanionQueueDecisionRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                proof: proof(&session, 3, "approve-1"),
                queue_id: preview.id.clone(),
                approved: true,
                idempotency_key: "queue-approve-1".to_owned(),
                temporary_chat: false,
            })
            .expect("queue should be approved");
        assert_eq!(queued.status, CompanionQueueStatus::Queued);

        let session = database
            .list_companion_sessions(ASTRA_ID)
            .unwrap()
            .remove(0);
        let cancelled = database
            .cancel_companion_queue(CompanionQueueActionRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                proof: proof(&session, 4, "cancel-1"),
                queue_id: preview.id.clone(),
                idempotency_key: "queue-cancel-1".to_owned(),
                temporary_chat: false,
            })
            .expect("queue should be cancellable");
        assert_eq!(cancelled.status, CompanionQueueStatus::Cancelled);

        let session = database
            .list_companion_sessions(ASTRA_ID)
            .unwrap()
            .remove(0);
        let retried = database
            .retry_companion_queue(CompanionQueueActionRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                proof: proof(&session, 5, "retry-1"),
                queue_id: preview.id.clone(),
                idempotency_key: "queue-retry-1".to_owned(),
                temporary_chat: false,
            })
            .expect("queue should return to preview");
        assert_eq!(retried.status, CompanionQueueStatus::Previewed);
        assert_eq!(retried.retry_count, 1);

        let session = database
            .list_companion_sessions(ASTRA_ID)
            .unwrap()
            .remove(0);
        let replay = database.cancel_companion_queue(CompanionQueueActionRequest {
            agent_id: ASTRA_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            proof: proof(&session, 5, "retry-1"),
            queue_id: preview.id,
            idempotency_key: "queue-cancel-replay".to_owned(),
            temporary_chat: false,
        });
        assert_eq!(
            replay,
            Err(DatabaseError::Cognitive("companion_replay_rejected"))
        );

        let rotation = database
            .rotate_companion_key(CompanionDeviceActionRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                device_id: paired.device_id.clone(),
                reason: "rotação fixture".to_owned(),
                idempotency_key: "key-rotate-1".to_owned(),
                temporary_chat: false,
            })
            .expect("key should rotate");
        assert_eq!(rotation.new_key_version, 2);
        assert_ne!(rotation.old_fingerprint, rotation.new_fingerprint);

        let stale = database.reconnect_companion_session(CompanionReconnectRequest {
            agent_id: ASTRA_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            proof: proof(&session, 6, "stale-key"),
            idempotency_key: "session-reconnect-stale".to_owned(),
            temporary_chat: false,
        });
        assert_eq!(
            stale,
            Err(DatabaseError::Cognitive("companion_authentication_failed"))
        );

        let revocation = database
            .revoke_companion_device(CompanionDeviceActionRequest {
                agent_id: ASTRA_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                device_id: paired.device_id,
                reason: "revogação fixture".to_owned(),
                idempotency_key: "device-revoke-1".to_owned(),
                temporary_chat: false,
            })
            .expect("device should be revoked");
        assert_eq!(revocation.previous_status, "paired");
        assert_eq!(
            database.list_companion_devices(ASTRA_ID).unwrap()[0].status,
            CompanionDeviceStatus::Revoked
        );
        assert!(!database
            .list_companion_history(ASTRA_ID)
            .unwrap()
            .is_empty());
        assert!(!database.list_companion_audit(ASTRA_ID).unwrap().is_empty());
        cleanup(&path);
    }

    #[test]
    fn companion_mutations_fail_closed_for_temporary_chat_and_safe_mode() {
        let (database, path) = database();
        let mut temporary = pairing_request("pair-temporary");
        temporary.temporary_chat = true;
        assert_eq!(
            database.start_companion_pairing(temporary),
            Err(DatabaseError::Cognitive("companion_blocked_temporary"))
        );

        database.set_safe_mode(true).unwrap();
        assert_eq!(
            database.start_companion_pairing(pairing_request("pair-safe-mode")),
            Err(DatabaseError::Cognitive("companion_blocked_safe_mode"))
        );
        cleanup(&path);
    }
}
