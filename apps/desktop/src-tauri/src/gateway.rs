use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError, LUMA_ID, OWNER_ID};

pub const GATEWAY_PROTOCOL_VERSION: i64 = 1;
pub const GATEWAY_MIN_PROTOCOL_VERSION: i64 = 1;
pub const GATEWAY_FIXTURE_AGENT_ID: &str = LUMA_ID;
pub const GATEWAY_FIXTURE_ACCOUNT_ID: &str = "gateway-account-owner";
pub const GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID: &str = "aip-owner-local";
pub const GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA: &str = "fixture:external-account/bielos-owner";
pub const GATEWAY_FIXTURE_CLIENT_ID: &str = "mobile-admin-fixture-01";
pub const GATEWAY_FIXTURE_APP_VERSION: &str = "0.1.0-gateway-fixture";
pub const GATEWAY_FIXTURE_AUTH_PROOF_METADATA: &str = "fixture:auth/mobile-admin-01";
pub const GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH: &str = "sha256:fixture/girlfriend-agent-v1";
pub const GATEWAY_FIXTURE_RECOVERY_TARGET: &str = "fixture:recovery/owner-access";
pub const GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA: &str = "fixture:tunnel/aip-gateway";
pub const GATEWAY_CLOUDFLARE_HOSTNAME_METADATA: &str = "example.invalid";
pub const GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA: &str = "fixture:access/aip-owner";

const MAX_GATEWAY_ACCOUNTS: i64 = 4;
const MAX_GATEWAY_TRANSFERS: i64 = 16;
const MAX_GATEWAY_SESSIONS: i64 = 32;
const MAX_GATEWAY_RECOVERIES: i64 = 32;
const MAX_GATEWAY_AUDIT_ROWS: i64 = 100;
const MAX_GATEWAY_REVOCATIONS: i64 = 32;
const MAX_GATEWAY_REPLAY_GUARDS: i64 = 256;
const MAX_GATEWAY_REFERENCE_BYTES: usize = 192;
const MAX_GATEWAY_TEXT_BYTES: usize = 512;
const MAX_GATEWAY_REQUEST_BYTES: usize = 16_384;
const GATEWAY_SESSION_TTL_MS: i64 = 30 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayAccountStatus {
    MetadataOnly,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayTransferStatus {
    Previewed,
    Approved,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewaySessionStatus {
    Connected,
    Disconnected,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayRecoveryStatus {
    PendingApproval,
    Approved,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayMessageKind {
    Session,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayCloudflareMetadata {
    pub provider: String,
    pub mode: String,
    pub tunnel_id_metadata: String,
    pub hostname_metadata: String,
    pub access_audience_metadata: String,
    pub credential_state: String,
    pub network_listener: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayProtocolInfo {
    pub schema_version: i64,
    pub protocol_version: i64,
    pub min_protocol_version: i64,
    pub transport: String,
    pub network_listener: bool,
    pub cloudflare: GatewayCloudflareMetadata,
    pub standalone_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayProtocolMessage {
    pub schema_version: i64,
    pub protocol_version: i64,
    pub message_id: String,
    pub client_id: String,
    pub kind: GatewayMessageKind,
    pub session_id: String,
    pub nonce_metadata: String,
    pub replay_counter: i64,
    pub payload_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayAccount {
    pub id: String,
    pub owner_user_id: String,
    pub local_account_id: String,
    pub external_account_id_metadata: String,
    pub ownership_scope: String,
    pub status: GatewayAccountStatus,
    pub metadata_only: bool,
    pub external_effect_performed: bool,
    pub standalone_fallback: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTransfer {
    pub id: String,
    pub account_id: String,
    pub source_agent_id: String,
    pub owner_user_id: String,
    pub destination_account_metadata: String,
    pub integrity_hash: String,
    pub status: GatewayTransferStatus,
    pub authorization_status: String,
    pub approval_required: bool,
    pub metadata_only: bool,
    pub external_effect_performed: bool,
    pub standalone_fallback: bool,
    pub created_at: i64,
    pub approved_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySessionProof {
    pub session_id: String,
    pub transfer_id: String,
    pub client_id: String,
    pub session_nonce_metadata: String,
    pub auth_proof_metadata: String,
    pub app_version: String,
    pub protocol_version: i64,
    pub message_nonce_metadata: String,
    pub replay_counter: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySession {
    pub id: String,
    pub account_id: String,
    pub transfer_id: String,
    pub source_agent_id: String,
    pub owner_user_id: String,
    pub client_id: String,
    pub status: GatewaySessionStatus,
    pub protocol_version: i64,
    pub app_version: String,
    pub negotiated_protocol_version: i64,
    pub session_nonce_metadata: String,
    pub auth_proof_metadata: String,
    pub last_replay_counter: i64,
    pub scope: String,
    pub authenticated: bool,
    pub local_loopback_only: bool,
    pub standalone_fallback: bool,
    pub connected_at: i64,
    pub last_seen_at: i64,
    pub disconnected_at: Option<i64>,
    pub protocol: GatewayProtocolInfo,
    pub handshake: GatewayProtocolMessage,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRecovery {
    pub id: String,
    pub account_id: String,
    pub transfer_id: String,
    pub session_id: String,
    pub source_agent_id: String,
    pub owner_user_id: String,
    pub client_id: String,
    pub kind: String,
    pub status: GatewayRecoveryStatus,
    pub target_metadata: String,
    pub approval_required: bool,
    pub metadata_only: bool,
    pub external_effect_performed: bool,
    pub created_at: i64,
    pub approved_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayAuditRecord {
    pub id: String,
    pub account_id: Option<String>,
    pub transfer_id: Option<String>,
    pub session_id: Option<String>,
    pub recovery_id: Option<String>,
    pub source_agent_id: String,
    pub owner_user_id: String,
    pub event: String,
    pub result: String,
    pub code: Option<String>,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRevocation {
    pub id: String,
    pub account_id: String,
    pub transfer_id: Option<String>,
    pub session_id: Option<String>,
    pub owner_user_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub previous_status: String,
    pub reason: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTransferRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub destination_account_metadata: String,
    pub integrity_hash: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTransferApprovalRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub transfer_id: String,
    pub approved: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySessionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub transfer_id: String,
    pub client_id: String,
    pub app_version: String,
    pub protocol_version: i64,
    pub auth_proof_metadata: String,
    pub message_nonce_metadata: String,
    pub replay_counter: i64,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayReconnectRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: GatewaySessionProof,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRecoveryRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: GatewaySessionProof,
    pub recovery_kind: String,
    pub target_metadata: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRecoveryApprovalRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proof: GatewaySessionProof,
    pub recovery_id: String,
    pub approved: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySessionActionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub session_id: String,
    pub reason: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTransferActionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub transfer_id: String,
    pub reason: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

impl GatewayTransferStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Previewed => "previewed",
            Self::Approved => "approved",
            Self::Revoked => "revoked",
        }
    }
}

impl GatewaySessionStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }
}

impl Database {
    pub fn gateway_protocol_info(
        &self,
        agent_id: &str,
    ) -> Result<GatewayProtocolInfo, DatabaseError> {
        let _fixture_account_ids = (GATEWAY_FIXTURE_ACCOUNT_ID, GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID);
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        Ok(gateway_protocol_info())
    }

    pub fn list_gateway_accounts(
        &self,
        agent_id: &str,
    ) -> Result<Vec<GatewayAccount>, DatabaseError> {
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, owner_user_id, local_account_id, external_account_id_metadata,
                    ownership_scope, status, metadata_only, external_effect_performed,
                    standalone_fallback, created_at, updated_at
             FROM gateway_accounts WHERE owner_user_id = ?1
             ORDER BY updated_at DESC, id DESC LIMIT ?2",
        )?;
        let accounts = statement
            .query_map(
                params![OWNER_ID, MAX_GATEWAY_ACCOUNTS],
                gateway_account_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DatabaseError::from);
        accounts
    }

    pub fn list_gateway_transfers(
        &self,
        agent_id: &str,
    ) -> Result<Vec<GatewayTransfer>, DatabaseError> {
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id FROM gateway_transfers
             WHERE source_agent_id = ?1
             ORDER BY updated_at DESC, id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(params![agent_id, MAX_GATEWAY_TRANSFERS], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter()
            .map(|id| load_gateway_transfer_connection(&connection, id))
            .collect()
    }

    pub fn prepare_gateway_transfer(
        &self,
        request: GatewayTransferRequest,
    ) -> Result<GatewayTransfer, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_transfer_request(&request)?;
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "transfer_prepare",
            &idempotency_key,
            &request_json,
            "transfer",
        )? {
            let transfer = load_gateway_transfer_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(transfer);
        }
        let account = load_gateway_account_tx(&transaction, &request.owner_user_id)?;
        if account.status != GatewayAccountStatus::MetadataOnly {
            return Err(DatabaseError::Cognitive("gateway_account_revoked"));
        }
        let active_transfer: Option<String> = transaction
            .query_row(
                "SELECT id FROM gateway_transfers
                 WHERE account_id = ?1 AND status IN ('previewed', 'approved')
                 ORDER BY updated_at DESC LIMIT 1",
                params![account.id],
                |row| row.get(0),
            )
            .optional()?;
        if active_transfer.is_some() {
            return Err(DatabaseError::Cognitive("gateway_transfer_already_active"));
        }
        let now = now_millis();
        let transfer_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO gateway_transfers
             (id, account_id, source_agent_id, owner_user_id, destination_account_metadata,
              integrity_hash, status, authorization_status, approval_required, metadata_only,
              external_effect_performed, standalone_fallback, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'previewed', 'pending_owner_approval',
                     1, 1, 0, 1, ?7, ?7)",
            params![
                transfer_id,
                account.id,
                request.agent_id,
                request.owner_user_id,
                request.destination_account_metadata,
                request.integrity_hash,
                now,
            ],
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&account.id),
                transfer_id: Some(&transfer_id),
                session_id: None,
                recovery_id: None,
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "transfer_prepared",
                result: "pending_owner_approval",
                code: None,
                summary: "Prévia de transferência da agente fixture aguardando aprovação do Owner",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "transfer_prepare",
            &idempotency_key,
            &request_json,
            "transfer",
            &transfer_id,
        )?;
        let transfer = load_gateway_transfer_tx(&transaction, &transfer_id)?;
        transaction.commit()?;
        Ok(transfer)
    }

    pub fn approve_gateway_transfer(
        &self,
        request: GatewayTransferApprovalRequest,
    ) -> Result<GatewayTransfer, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_reference(&request.transfer_id, 128, "gateway_transfer_invalid")?;
        if !request.approved {
            return Err(DatabaseError::Cognitive("gateway_approval_required"));
        }
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "transfer_approve",
            &idempotency_key,
            &request_json,
            "transfer",
        )? {
            let transfer = load_gateway_transfer_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(transfer);
        }
        let transfer = load_gateway_transfer_tx(&transaction, &request.transfer_id)?;
        validate_gateway_transfer_owner(&transfer, &request.agent_id, &request.owner_user_id)?;
        if transfer.status == GatewayTransferStatus::Revoked {
            return Err(DatabaseError::Cognitive("gateway_transfer_revoked"));
        }
        if transfer.status != GatewayTransferStatus::Previewed
            || transfer.integrity_hash != GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH
        {
            return Err(DatabaseError::Cognitive(
                "gateway_transfer_integrity_failed",
            ));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE gateway_transfers
             SET status = 'approved', authorization_status = 'owner_approved',
                 approved_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'previewed'",
            params![now, transfer.id],
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&transfer.account_id),
                transfer_id: Some(&transfer.id),
                session_id: None,
                recovery_id: None,
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "transfer_approved",
                result: "owner_approved",
                code: None,
                summary:
                    "Transferência fixture aprovada pelo Owner; nenhum efeito externo foi executado",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "transfer_approve",
            &idempotency_key,
            &request_json,
            "transfer",
            &transfer.id,
        )?;
        let transfer = load_gateway_transfer_tx(&transaction, &transfer.id)?;
        transaction.commit()?;
        Ok(transfer)
    }

    pub fn list_gateway_sessions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<GatewaySession>, DatabaseError> {
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id FROM gateway_sessions
             WHERE source_agent_id = ?1
             ORDER BY last_seen_at DESC, id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(params![agent_id, MAX_GATEWAY_SESSIONS], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter()
            .map(|id| load_gateway_session_connection(&connection, id))
            .collect()
    }

    pub fn connect_gateway_session(
        &self,
        request: GatewaySessionRequest,
    ) -> Result<GatewaySession, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_session_request(&request)?;
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_gateway_state(&transaction)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "session_connect",
            &idempotency_key,
            &request_json,
            "session",
        )? {
            let session = load_gateway_session_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let transfer = load_gateway_transfer_tx(&transaction, &request.transfer_id)?;
        validate_gateway_transfer_owner(&transfer, &request.agent_id, &request.owner_user_id)?;
        if transfer.status != GatewayTransferStatus::Approved {
            return Err(DatabaseError::Cognitive(
                "gateway_transfer_approval_required",
            ));
        }
        ensure_gateway_replay_nonce_available(
            &transaction,
            &request.client_id,
            &request.message_nonce_metadata,
        )?;
        ensure_gateway_replay_counter_fresh(
            &transaction,
            &request.client_id,
            request.replay_counter,
        )?;
        let now = now_millis();
        transaction.execute(
            "UPDATE gateway_sessions
             SET status = 'disconnected', disconnected_at = ?1, updated_at = ?1
             WHERE client_id = ?2 AND status = 'connected'",
            params![now, request.client_id],
        )?;
        let session_id = Uuid::now_v7().to_string();
        let session_nonce_metadata = format!(
            "fixture:gateway-session/{}/{}",
            request.client_id,
            Uuid::now_v7()
        );
        transaction.execute(
            "INSERT INTO gateway_sessions
             (id, account_id, transfer_id, source_agent_id, owner_user_id, client_id, status,
              protocol_version, app_version, negotiated_protocol_version, session_nonce_metadata,
              auth_proof_metadata, last_replay_counter, scope, authenticated, local_loopback_only,
              standalone_fallback, connected_at, last_seen_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'connected', ?7, ?8, ?7, ?9, ?10, ?11,
                     'administrative_recovery', 1, 1, 1, ?12, ?12, ?12)",
            params![
                session_id,
                transfer.account_id,
                transfer.id,
                request.agent_id,
                request.owner_user_id,
                request.client_id,
                request.protocol_version,
                request.app_version,
                session_nonce_metadata,
                request.auth_proof_metadata,
                request.replay_counter,
                now,
            ],
        )?;
        insert_gateway_replay_guard(
            &transaction,
            &request.client_id,
            Some(&session_id),
            &request.message_nonce_metadata,
            request.replay_counter,
            "session_connect",
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&transfer.account_id),
                transfer_id: Some(&transfer.id),
                session_id: Some(&session_id),
                recovery_id: None,
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "session_connected",
                result: "authenticated",
                code: None,
                summary: "Sessão administrativa fixture autenticada em transporte local sintético",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "session_connect",
            &idempotency_key,
            &request_json,
            "session",
            &session_id,
        )?;
        let session = load_gateway_session_tx(&transaction, &session_id)?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn reconnect_gateway_session(
        &self,
        request: GatewayReconnectRequest,
    ) -> Result<GatewaySession, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_session_proof(&request.proof)?;
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_gateway_state(&transaction)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "session_reconnect",
            &idempotency_key,
            &request_json,
            "session",
        )? {
            let session = load_gateway_session_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let session = authenticate_gateway_session(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            "session_reconnect",
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&session.account_id),
                transfer_id: Some(&session.transfer_id),
                session_id: Some(&session.id),
                recovery_id: None,
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "session_reconnected",
                result: "authenticated",
                code: None,
                summary: "Reconexão administrativa aceita após compatibilidade e replay",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "session_reconnect",
            &idempotency_key,
            &request_json,
            "session",
            &session.id,
        )?;
        let session = load_gateway_session_tx(&transaction, &session.id)?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn list_gateway_recoveries(
        &self,
        agent_id: &str,
    ) -> Result<Vec<GatewayRecovery>, DatabaseError> {
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id FROM gateway_recoveries
             WHERE source_agent_id = ?1
             ORDER BY updated_at DESC, id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(params![agent_id, MAX_GATEWAY_RECOVERIES], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter()
            .map(|id| load_gateway_recovery_connection(&connection, id))
            .collect()
    }

    pub fn request_gateway_recovery(
        &self,
        request: GatewayRecoveryRequest,
    ) -> Result<GatewayRecovery, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_recovery_request(&request)?;
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        expire_gateway_state(&transaction)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "recovery_request",
            &idempotency_key,
            &request_json,
            "recovery",
        )? {
            let recovery = load_gateway_recovery_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(recovery);
        }
        let session = authenticate_gateway_session(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            "recovery_request",
        )?;
        let active_recoveries: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM gateway_recoveries
             WHERE session_id = ?1 AND status IN ('pending_approval', 'approved')",
            params![session.id],
            |row| row.get(0),
        )?;
        if active_recoveries >= 4 {
            return Err(DatabaseError::Cognitive("gateway_recovery_limit"));
        }
        let now = now_millis();
        let recovery_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO gateway_recoveries
             (id, account_id, transfer_id, session_id, source_agent_id, owner_user_id,
              client_id, kind, status, target_metadata, approval_required, metadata_only,
              external_effect_performed, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'mobile_administrative',
                     'pending_approval', ?8, 1, 1, 0, ?9, ?9)",
            params![
                recovery_id,
                session.account_id,
                session.transfer_id,
                session.id,
                request.agent_id,
                request.owner_user_id,
                session.client_id,
                request.target_metadata,
                now,
            ],
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&session.account_id),
                transfer_id: Some(&session.transfer_id),
                session_id: Some(&session.id),
                recovery_id: Some(&recovery_id),
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "recovery_requested",
                result: "pending_owner_approval",
                code: None,
                summary:
                    "Solicitação de recuperação administrativa móvel aguardando aprovação do Owner",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "recovery_request",
            &idempotency_key,
            &request_json,
            "recovery",
            &recovery_id,
        )?;
        let recovery = load_gateway_recovery_tx(&transaction, &recovery_id)?;
        transaction.commit()?;
        Ok(recovery)
    }

    pub fn approve_gateway_recovery(
        &self,
        request: GatewayRecoveryApprovalRequest,
    ) -> Result<GatewayRecovery, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_session_proof(&request.proof)?;
        validate_gateway_reference(&request.recovery_id, 128, "gateway_recovery_invalid")?;
        if !request.approved {
            return Err(DatabaseError::Cognitive("gateway_approval_required"));
        }
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "recovery_approve",
            &idempotency_key,
            &request_json,
            "recovery",
        )? {
            let recovery = load_gateway_recovery_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(recovery);
        }
        let session = authenticate_gateway_session(
            &transaction,
            &request.agent_id,
            &request.owner_user_id,
            &request.proof,
            "recovery_approve",
        )?;
        let recovery = load_gateway_recovery_tx(&transaction, &request.recovery_id)?;
        if recovery.session_id != session.id
            || recovery.transfer_id != session.transfer_id
            || recovery.owner_user_id != request.owner_user_id
        {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if recovery.status != GatewayRecoveryStatus::PendingApproval {
            return Err(DatabaseError::Cognitive("gateway_recovery_state_invalid"));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE gateway_recoveries
             SET status = 'approved', approved_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'pending_approval'",
            params![now, recovery.id],
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&recovery.account_id),
                transfer_id: Some(&recovery.transfer_id),
                session_id: Some(&recovery.session_id),
                recovery_id: Some(&recovery.id),
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "recovery_approved",
                result: "owner_approved",
                code: None,
                summary: "Recuperação administrativa aprovada; nenhum acesso externo foi executado",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "recovery_approve",
            &idempotency_key,
            &request_json,
            "recovery",
            &recovery.id,
        )?;
        let recovery = load_gateway_recovery_tx(&transaction, &recovery.id)?;
        transaction.commit()?;
        Ok(recovery)
    }

    pub fn list_gateway_audit(
        &self,
        agent_id: &str,
    ) -> Result<Vec<GatewayAuditRecord>, DatabaseError> {
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, account_id, transfer_id, session_id, recovery_id,
                    source_agent_id, owner_user_id, event, result, code, details_json, created_at
             FROM gateway_audit_log WHERE source_agent_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let records = statement
            .query_map(params![agent_id, MAX_GATEWAY_AUDIT_ROWS], |row| {
                let details_json: String = row.get(10)?;
                let summary = serde_json::from_str::<Value>(&details_json)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "Evento de gateway".to_owned());
                Ok(GatewayAuditRecord {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    transfer_id: row.get(2)?,
                    session_id: row.get(3)?,
                    recovery_id: row.get(4)?,
                    source_agent_id: row.get(5)?,
                    owner_user_id: row.get(6)?,
                    event: row.get(7)?,
                    result: row.get(8)?,
                    code: row.get(9)?,
                    summary,
                    created_at: row.get(11)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub fn list_gateway_revocations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<GatewayRevocation>, DatabaseError> {
        let connection = self.open()?;
        ensure_gateway_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, account_id, transfer_id, session_id, owner_user_id,
                    target_kind, target_id, previous_status, reason, created_at
             FROM gateway_revocations WHERE owner_user_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let revocations = statement
            .query_map(params![OWNER_ID, MAX_GATEWAY_REVOCATIONS], |row| {
                Ok(GatewayRevocation {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    transfer_id: row.get(2)?,
                    session_id: row.get(3)?,
                    owner_user_id: row.get(4)?,
                    target_kind: row.get(5)?,
                    target_id: row.get(6)?,
                    previous_status: row.get(7)?,
                    reason: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DatabaseError::from);
        revocations
    }

    pub fn revoke_gateway_session(
        &self,
        request: GatewaySessionActionRequest,
    ) -> Result<GatewayRevocation, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_reference(&request.session_id, 128, "gateway_session_invalid")?;
        let reason = validate_gateway_text(&request.reason, MAX_GATEWAY_TEXT_BYTES)?;
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "session_revoke",
            &idempotency_key,
            &request_json,
            "revocation",
        )? {
            let revocation = load_gateway_revocation_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(revocation);
        }
        let session = load_gateway_session_tx(&transaction, &request.session_id)?;
        if session.source_agent_id != request.agent_id
            || session.owner_user_id != request.owner_user_id
        {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if session.status == GatewaySessionStatus::Revoked {
            return Err(DatabaseError::Cognitive("gateway_session_revoked"));
        }
        let previous_status = session.status.as_str().to_owned();
        let now = now_millis();
        transaction.execute(
            "UPDATE gateway_sessions
             SET status = 'revoked', disconnected_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now, session.id],
        )?;
        transaction.execute(
            "UPDATE gateway_recoveries SET status = 'revoked', updated_at = ?1
             WHERE session_id = ?2 AND status IN ('pending_approval', 'approved')",
            params![now, session.id],
        )?;
        let revocation_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO gateway_revocations
             (id, account_id, session_id, owner_user_id, target_kind, target_id,
              previous_status, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, 'session', ?3, ?5, ?6, ?7)",
            params![
                revocation_id,
                session.account_id,
                session.id,
                request.owner_user_id,
                previous_status,
                reason,
                now,
            ],
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&session.account_id),
                transfer_id: Some(&session.transfer_id),
                session_id: Some(&session.id),
                recovery_id: None,
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "session_revoked",
                result: "revoked",
                code: None,
                summary: "Sessão administrativa revogada pelo Owner; recuperação pendente fechada",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "session_revoke",
            &idempotency_key,
            &request_json,
            "revocation",
            &revocation_id,
        )?;
        let revocation = load_gateway_revocation_tx(&transaction, &revocation_id)?;
        transaction.commit()?;
        Ok(revocation)
    }

    pub fn revoke_gateway_transfer(
        &self,
        request: GatewayTransferActionRequest,
    ) -> Result<GatewayRevocation, DatabaseError> {
        ensure_gateway_not_temporary(request.temporary_chat)?;
        validate_gateway_reference(&request.transfer_id, 128, "gateway_transfer_invalid")?;
        let reason = validate_gateway_text(&request.reason, MAX_GATEWAY_TEXT_BYTES)?;
        let request_json = gateway_request_json(&request)?;
        let idempotency_key = valid_gateway_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_gateway_owner_mode(&transaction, &request.agent_id, &request.owner_user_id)?;
        if let Some(result_id) = existing_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "transfer_revoke",
            &idempotency_key,
            &request_json,
            "revocation",
        )? {
            let revocation = load_gateway_revocation_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(revocation);
        }
        let transfer = load_gateway_transfer_tx(&transaction, &request.transfer_id)?;
        validate_gateway_transfer_owner(&transfer, &request.agent_id, &request.owner_user_id)?;
        if transfer.status == GatewayTransferStatus::Revoked {
            return Err(DatabaseError::Cognitive("gateway_transfer_revoked"));
        }
        let previous_status = transfer.status.as_str().to_owned();
        let now = now_millis();
        transaction.execute(
            "UPDATE gateway_transfers
             SET status = 'revoked', authorization_status = 'revoked', updated_at = ?1
             WHERE id = ?2",
            params![now, transfer.id],
        )?;
        transaction.execute(
            "UPDATE gateway_sessions
             SET status = 'revoked', disconnected_at = ?1, updated_at = ?1
             WHERE transfer_id = ?2 AND status IN ('connected', 'disconnected')",
            params![now, transfer.id],
        )?;
        transaction.execute(
            "UPDATE gateway_recoveries SET status = 'revoked', updated_at = ?1
             WHERE transfer_id = ?2 AND status IN ('pending_approval', 'approved')",
            params![now, transfer.id],
        )?;
        let revocation_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO gateway_revocations
             (id, account_id, transfer_id, owner_user_id, target_kind, target_id,
              previous_status, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, 'transfer', ?3, ?5, ?6, ?7)",
            params![
                revocation_id,
                transfer.account_id,
                transfer.id,
                request.owner_user_id,
                previous_status,
                reason,
                now,
            ],
        )?;
        insert_gateway_audit(
            &transaction,
            GatewayAuditContext {
                account_id: Some(&transfer.account_id),
                transfer_id: Some(&transfer.id),
                session_id: None,
                recovery_id: None,
                source_agent_id: &request.agent_id,
                owner_user_id: &request.owner_user_id,
                event: "transfer_revoked",
                result: "revoked",
                code: None,
                summary: "Transferência fixture revogada; sessões e recuperações foram fechadas",
            },
        )?;
        insert_gateway_idempotency(
            &transaction,
            &request.owner_user_id,
            "transfer_revoke",
            &idempotency_key,
            &request_json,
            "revocation",
            &revocation_id,
        )?;
        let revocation = load_gateway_revocation_tx(&transaction, &revocation_id)?;
        transaction.commit()?;
        Ok(revocation)
    }
}

impl GatewayAccountStatus {
    fn from_str(value: &str) -> rusqlite::Result<Self> {
        match value {
            "metadata_only" => Ok(Self::MetadataOnly),
            "revoked" => Ok(Self::Revoked),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }
}

fn gateway_transfer_status_from_str(value: &str) -> rusqlite::Result<GatewayTransferStatus> {
    match value {
        "previewed" => Ok(GatewayTransferStatus::Previewed),
        "approved" => Ok(GatewayTransferStatus::Approved),
        "revoked" => Ok(GatewayTransferStatus::Revoked),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn gateway_session_status_from_str(value: &str) -> rusqlite::Result<GatewaySessionStatus> {
    match value {
        "connected" => Ok(GatewaySessionStatus::Connected),
        "disconnected" => Ok(GatewaySessionStatus::Disconnected),
        "revoked" => Ok(GatewaySessionStatus::Revoked),
        "expired" => Ok(GatewaySessionStatus::Expired),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn gateway_recovery_status_from_str(value: &str) -> rusqlite::Result<GatewayRecoveryStatus> {
    match value {
        "pending_approval" => Ok(GatewayRecoveryStatus::PendingApproval),
        "approved" => Ok(GatewayRecoveryStatus::Approved),
        "revoked" => Ok(GatewayRecoveryStatus::Revoked),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn gateway_protocol_info() -> GatewayProtocolInfo {
    GatewayProtocolInfo {
        schema_version: 1,
        protocol_version: GATEWAY_PROTOCOL_VERSION,
        min_protocol_version: GATEWAY_MIN_PROTOCOL_VERSION,
        transport: "local_loopback_fixture".to_owned(),
        network_listener: false,
        cloudflare: GatewayCloudflareMetadata {
            provider: "cloudflare_tunnel_access".to_owned(),
            mode: "metadata_only".to_owned(),
            tunnel_id_metadata: GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA.to_owned(),
            hostname_metadata: GATEWAY_CLOUDFLARE_HOSTNAME_METADATA.to_owned(),
            access_audience_metadata: GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA.to_owned(),
            credential_state: "absent".to_owned(),
            network_listener: false,
        },
        standalone_fallback: true,
    }
}

fn ensure_gateway_agent(connection: &Connection, agent_id: &str) -> Result<(), DatabaseError> {
    if agent_id != GATEWAY_FIXTURE_AGENT_ID {
        return Err(DatabaseError::Cognitive("gateway_fixture_agent_invalid"));
    }
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
        Err(DatabaseError::Cognitive("gateway_agent_invalid"))
    }
}

fn ensure_gateway_owner(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    if agent_id != GATEWAY_FIXTURE_AGENT_ID {
        return Err(DatabaseError::Cognitive("gateway_fixture_agent_invalid"));
    }
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
        Err(DatabaseError::Cognitive("gateway_owner_required"))
    }
}

fn ensure_gateway_owner_mode(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    ensure_gateway_owner(transaction, agent_id, owner_user_id)?;
    let safe_mode = transaction
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = 'safe_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some_and(|value| value == "true");
    if safe_mode {
        return Err(DatabaseError::Cognitive("gateway_blocked_safe_mode"));
    }
    let (mode, suspended): (String, bool) = transaction
        .query_row(
            "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id = ?1",
            params![agent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_agent_invalid"))?;
    if suspended {
        return Err(DatabaseError::Cognitive("gateway_blocked_suspended"));
    }
    if mode == "safe" {
        return Err(DatabaseError::Cognitive("gateway_blocked_safe_mode"));
    }
    Ok(())
}

fn ensure_gateway_not_temporary(temporary_chat: bool) -> Result<(), DatabaseError> {
    if temporary_chat {
        Err(DatabaseError::Cognitive("gateway_blocked_temporary"))
    } else {
        Ok(())
    }
}

fn validate_gateway_text(value: &str, maximum: usize) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > maximum
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(DatabaseError::Cognitive("gateway_text_invalid"));
    }
    Ok(value.to_owned())
}

fn validate_gateway_reference(
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

fn valid_gateway_idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        return Err(DatabaseError::Cognitive("gateway_idempotency_invalid"));
    }
    Ok(value.to_owned())
}

fn gateway_request_json<T: Serialize>(request: &T) -> Result<String, DatabaseError> {
    let value = serde_json::to_string(request).map_err(|_| DatabaseError::Unavailable)?;
    if value.len() > MAX_GATEWAY_REQUEST_BYTES {
        Err(DatabaseError::Cognitive("gateway_request_oversized"))
    } else {
        Ok(value)
    }
}

fn ensure_gateway_compatible(
    protocol_version: i64,
    app_version: &str,
) -> Result<(), DatabaseError> {
    if !(GATEWAY_MIN_PROTOCOL_VERSION..=GATEWAY_PROTOCOL_VERSION).contains(&protocol_version)
        || app_version != GATEWAY_FIXTURE_APP_VERSION
    {
        Err(DatabaseError::Cognitive("gateway_protocol_incompatible"))
    } else {
        Ok(())
    }
}

fn validate_gateway_transfer_request(
    request: &GatewayTransferRequest,
) -> Result<(), DatabaseError> {
    if request.agent_id != GATEWAY_FIXTURE_AGENT_ID
        || request.destination_account_metadata != GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA
        || request.integrity_hash != GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH
    {
        return Err(DatabaseError::Cognitive("gateway_transfer_fixture_invalid"));
    }
    validate_gateway_reference(
        &request.destination_account_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_account_invalid",
    )?;
    validate_gateway_reference(&request.integrity_hash, 256, "gateway_integrity_invalid")
        .map(|_| ())
}

fn validate_gateway_session_request(request: &GatewaySessionRequest) -> Result<(), DatabaseError> {
    validate_gateway_reference(&request.transfer_id, 128, "gateway_transfer_invalid")?;
    validate_gateway_reference(&request.client_id, 96, "gateway_client_invalid")?;
    validate_gateway_reference(
        &request.auth_proof_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_authentication_invalid",
    )?;
    validate_gateway_reference(
        &request.message_nonce_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_nonce_invalid",
    )?;
    if request.client_id != GATEWAY_FIXTURE_CLIENT_ID
        || request.auth_proof_metadata != GATEWAY_FIXTURE_AUTH_PROOF_METADATA
        || request.replay_counter < 1
    {
        return Err(DatabaseError::Cognitive("gateway_authentication_invalid"));
    }
    ensure_gateway_compatible(request.protocol_version, &request.app_version)
}

fn validate_gateway_session_proof(proof: &GatewaySessionProof) -> Result<(), DatabaseError> {
    validate_gateway_reference(&proof.session_id, 128, "gateway_session_invalid")?;
    validate_gateway_reference(&proof.transfer_id, 128, "gateway_transfer_invalid")?;
    validate_gateway_reference(&proof.client_id, 96, "gateway_client_invalid")?;
    validate_gateway_reference(
        &proof.session_nonce_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_nonce_invalid",
    )?;
    validate_gateway_reference(
        &proof.auth_proof_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_authentication_invalid",
    )?;
    validate_gateway_reference(
        &proof.message_nonce_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_nonce_invalid",
    )?;
    if proof.client_id != GATEWAY_FIXTURE_CLIENT_ID
        || proof.auth_proof_metadata != GATEWAY_FIXTURE_AUTH_PROOF_METADATA
        || proof.replay_counter < 1
    {
        return Err(DatabaseError::Cognitive("gateway_authentication_invalid"));
    }
    ensure_gateway_compatible(proof.protocol_version, &proof.app_version)
}

fn validate_gateway_recovery_request(
    request: &GatewayRecoveryRequest,
) -> Result<(), DatabaseError> {
    validate_gateway_session_proof(&request.proof)?;
    validate_gateway_reference(&request.recovery_kind, 64, "gateway_recovery_invalid")?;
    validate_gateway_reference(
        &request.target_metadata,
        MAX_GATEWAY_REFERENCE_BYTES,
        "gateway_recovery_invalid",
    )?;
    if request.recovery_kind != "mobile_administrative"
        || request.target_metadata != GATEWAY_FIXTURE_RECOVERY_TARGET
    {
        return Err(DatabaseError::Cognitive("gateway_recovery_fixture_invalid"));
    }
    Ok(())
}

fn validate_gateway_transfer_owner(
    transfer: &GatewayTransfer,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    if transfer.source_agent_id != agent_id || transfer.owner_user_id != owner_user_id {
        Err(DatabaseError::OwnershipMismatch)
    } else {
        Ok(())
    }
}

struct GatewayAuditContext<'a> {
    account_id: Option<&'a str>,
    transfer_id: Option<&'a str>,
    session_id: Option<&'a str>,
    recovery_id: Option<&'a str>,
    source_agent_id: &'a str,
    owner_user_id: &'a str,
    event: &'a str,
    result: &'a str,
    code: Option<&'a str>,
    summary: &'a str,
}

fn insert_gateway_audit(
    transaction: &Transaction<'_>,
    context: GatewayAuditContext<'_>,
) -> Result<(), DatabaseError> {
    if context.summary.is_empty() || context.summary.len() > MAX_GATEWAY_TEXT_BYTES {
        return Err(DatabaseError::Cognitive("gateway_audit_oversized"));
    }
    let details_json = serde_json::to_string(&json!({
        "summary": context.summary,
        "metadataOnly": true,
        "externalEffectPerformed": false,
        "networkListener": false,
    }))
    .map_err(|_| DatabaseError::Unavailable)?;
    if details_json.len() > 2_048 {
        return Err(DatabaseError::Cognitive("gateway_audit_oversized"));
    }
    let now = now_millis();
    transaction.execute(
        "DELETE FROM gateway_audit_log WHERE created_at < ?1",
        params![now - 30 * 24 * 60 * 60 * 1_000_i64],
    )?;
    transaction.execute(
        "INSERT INTO gateway_audit_log
         (id, account_id, transfer_id, session_id, recovery_id, source_agent_id,
          owner_user_id, event, result, code, details_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            Uuid::now_v7().to_string(),
            context.account_id,
            context.transfer_id,
            context.session_id,
            context.recovery_id,
            context.source_agent_id,
            context.owner_user_id,
            context.event,
            context.result,
            context.code,
            details_json,
            now,
        ],
    )?;
    transaction.execute(
        "DELETE FROM gateway_audit_log WHERE id IN (
           SELECT id FROM gateway_audit_log WHERE source_agent_id = ?1
           ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?2
         )",
        params![context.source_agent_id, MAX_GATEWAY_AUDIT_ROWS],
    )?;
    Ok(())
}

fn existing_gateway_idempotency(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    result_kind: &str,
) -> Result<Option<String>, DatabaseError> {
    let existing = transaction
        .query_row(
            "SELECT request_json, result_kind, result_id FROM gateway_idempotency
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
        return Err(DatabaseError::Cognitive("gateway_idempotency_conflict"));
    }
    Ok(Some(result_id))
}

fn insert_gateway_idempotency(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    result_kind: &str,
    result_id: &str,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO gateway_idempotency
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
    transaction.execute(
        "DELETE FROM gateway_idempotency WHERE rowid IN (
           SELECT rowid FROM gateway_idempotency WHERE owner_user_id = ?1
           ORDER BY created_at DESC, rowid DESC LIMIT -1 OFFSET 256
         )",
        params![owner_user_id],
    )?;
    Ok(())
}

fn expire_gateway_state(transaction: &Transaction<'_>) -> Result<(), DatabaseError> {
    let now = now_millis();
    transaction.execute(
        "UPDATE gateway_sessions SET status = 'expired', disconnected_at = ?1, updated_at = ?1
         WHERE status IN ('connected', 'disconnected') AND last_seen_at < ?2",
        params![now, now - GATEWAY_SESSION_TTL_MS],
    )?;
    transaction.execute(
        "DELETE FROM gateway_replay_guards WHERE id IN (
           SELECT id FROM gateway_replay_guards WHERE client_id = client_id
           ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?1
         )",
        params![MAX_GATEWAY_REPLAY_GUARDS],
    )?;
    Ok(())
}

fn load_gateway_account_connection(
    connection: &Connection,
    owner_user_id: &str,
) -> Result<GatewayAccount, DatabaseError> {
    connection
        .query_row(
            "SELECT id, owner_user_id, local_account_id, external_account_id_metadata,
                    ownership_scope, status, metadata_only, external_effect_performed,
                    standalone_fallback, created_at, updated_at
             FROM gateway_accounts WHERE owner_user_id = ?1",
            params![owner_user_id],
            gateway_account_from_row,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_account_not_found"))
}

fn load_gateway_account_tx(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
) -> Result<GatewayAccount, DatabaseError> {
    load_gateway_account_connection(transaction, owner_user_id)
}

fn gateway_account_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GatewayAccount> {
    Ok(GatewayAccount {
        id: row.get(0)?,
        owner_user_id: row.get(1)?,
        local_account_id: row.get(2)?,
        external_account_id_metadata: row.get(3)?,
        ownership_scope: row.get(4)?,
        status: GatewayAccountStatus::from_str(&row.get::<_, String>(5)?)?,
        metadata_only: row.get(6)?,
        external_effect_performed: row.get(7)?,
        standalone_fallback: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn load_gateway_transfer_connection(
    connection: &Connection,
    transfer_id: &str,
) -> Result<GatewayTransfer, DatabaseError> {
    connection
        .query_row(
            "SELECT id, account_id, source_agent_id, owner_user_id,
                    destination_account_metadata, integrity_hash, status, authorization_status,
                    approval_required, metadata_only, external_effect_performed,
                    standalone_fallback, created_at, approved_at, updated_at
             FROM gateway_transfers WHERE id = ?1",
            params![transfer_id],
            gateway_transfer_from_row,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_transfer_not_found"))
}

fn load_gateway_transfer_tx(
    transaction: &Transaction<'_>,
    transfer_id: &str,
) -> Result<GatewayTransfer, DatabaseError> {
    load_gateway_transfer_connection(transaction, transfer_id)
}

fn gateway_transfer_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GatewayTransfer> {
    Ok(GatewayTransfer {
        id: row.get(0)?,
        account_id: row.get(1)?,
        source_agent_id: row.get(2)?,
        owner_user_id: row.get(3)?,
        destination_account_metadata: row.get(4)?,
        integrity_hash: row.get(5)?,
        status: gateway_transfer_status_from_str(&row.get::<_, String>(6)?)?,
        authorization_status: row.get(7)?,
        approval_required: row.get(8)?,
        metadata_only: row.get(9)?,
        external_effect_performed: row.get(10)?,
        standalone_fallback: row.get(11)?,
        created_at: row.get(12)?,
        approved_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn load_gateway_session_connection(
    connection: &Connection,
    session_id: &str,
) -> Result<GatewaySession, DatabaseError> {
    let row = connection
        .query_row(
            "SELECT id, account_id, transfer_id, source_agent_id, owner_user_id, client_id,
                    status, protocol_version, app_version, negotiated_protocol_version,
                    session_nonce_metadata, auth_proof_metadata, last_replay_counter, scope,
                    authenticated, local_loopback_only, standalone_fallback, connected_at,
                    last_seen_at, disconnected_at, updated_at
             FROM gateway_sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    gateway_session_status_from_str(&row.get::<_, String>(6)?)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, bool>(14)?,
                    row.get::<_, bool>(15)?,
                    row.get::<_, bool>(16)?,
                    row.get::<_, i64>(17)?,
                    row.get::<_, i64>(18)?,
                    row.get::<_, Option<i64>>(19)?,
                    row.get::<_, i64>(20)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_session_not_found"))?;
    let guard = connection
        .query_row(
            "SELECT message_nonce_metadata, replay_counter FROM gateway_replay_guards
             WHERE session_id = ?1 ORDER BY replay_counter DESC, created_at DESC LIMIT 1",
            params![session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_session_invalid"))?;
    let protocol = gateway_protocol_info();
    let handshake = GatewayProtocolMessage {
        schema_version: 1,
        protocol_version: row.7,
        message_id: format!("gateway-handshake:{}", row.0),
        client_id: row.5.clone(),
        kind: GatewayMessageKind::Session,
        session_id: row.0.clone(),
        nonce_metadata: guard.0,
        replay_counter: guard.1,
        payload_kind: "authenticated_session".to_owned(),
    };
    Ok(GatewaySession {
        id: row.0,
        account_id: row.1,
        transfer_id: row.2,
        source_agent_id: row.3,
        owner_user_id: row.4,
        client_id: row.5,
        status: row.6,
        protocol_version: row.7,
        app_version: row.8,
        negotiated_protocol_version: row.9,
        session_nonce_metadata: row.10,
        auth_proof_metadata: row.11,
        last_replay_counter: row.12,
        scope: row.13,
        authenticated: row.14,
        local_loopback_only: row.15,
        standalone_fallback: row.16,
        connected_at: row.17,
        last_seen_at: row.18,
        disconnected_at: row.19,
        protocol,
        handshake,
        updated_at: row.20,
    })
}

fn load_gateway_session_tx(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<GatewaySession, DatabaseError> {
    load_gateway_session_connection(transaction, session_id)
}

fn load_gateway_recovery_connection(
    connection: &Connection,
    recovery_id: &str,
) -> Result<GatewayRecovery, DatabaseError> {
    connection
        .query_row(
            "SELECT id, account_id, transfer_id, session_id, source_agent_id, owner_user_id,
                    client_id, kind, status, target_metadata, approval_required, metadata_only,
                    external_effect_performed, created_at, approved_at, updated_at
             FROM gateway_recoveries WHERE id = ?1",
            params![recovery_id],
            |row| {
                Ok(GatewayRecovery {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    transfer_id: row.get(2)?,
                    session_id: row.get(3)?,
                    source_agent_id: row.get(4)?,
                    owner_user_id: row.get(5)?,
                    client_id: row.get(6)?,
                    kind: row.get(7)?,
                    status: gateway_recovery_status_from_str(&row.get::<_, String>(8)?)?,
                    target_metadata: row.get(9)?,
                    approval_required: row.get(10)?,
                    metadata_only: row.get(11)?,
                    external_effect_performed: row.get(12)?,
                    created_at: row.get(13)?,
                    approved_at: row.get(14)?,
                    updated_at: row.get(15)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_recovery_not_found"))
}

fn load_gateway_recovery_tx(
    transaction: &Transaction<'_>,
    recovery_id: &str,
) -> Result<GatewayRecovery, DatabaseError> {
    load_gateway_recovery_connection(transaction, recovery_id)
}

fn load_gateway_revocation_tx(
    transaction: &Transaction<'_>,
    revocation_id: &str,
) -> Result<GatewayRevocation, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, account_id, transfer_id, session_id, owner_user_id,
                    target_kind, target_id, previous_status, reason, created_at
             FROM gateway_revocations WHERE id = ?1",
            params![revocation_id],
            |row| {
                Ok(GatewayRevocation {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    transfer_id: row.get(2)?,
                    session_id: row.get(3)?,
                    owner_user_id: row.get(4)?,
                    target_kind: row.get(5)?,
                    target_id: row.get(6)?,
                    previous_status: row.get(7)?,
                    reason: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("gateway_revocation_not_found"))
}

fn ensure_gateway_replay_nonce_available(
    transaction: &Transaction<'_>,
    client_id: &str,
    message_nonce_metadata: &str,
) -> Result<(), DatabaseError> {
    let used: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM gateway_replay_guards
           WHERE client_id = ?1 AND message_nonce_metadata = ?2
         )",
        params![client_id, message_nonce_metadata],
        |row| row.get(0),
    )?;
    if used {
        Err(DatabaseError::Cognitive("gateway_replay_rejected"))
    } else {
        Ok(())
    }
}

fn ensure_gateway_replay_counter_fresh(
    transaction: &Transaction<'_>,
    client_id: &str,
    replay_counter: i64,
) -> Result<(), DatabaseError> {
    let last_counter: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(replay_counter), 0) FROM gateway_replay_guards
         WHERE client_id = ?1",
        params![client_id],
        |row| row.get(0),
    )?;
    if replay_counter <= last_counter {
        Err(DatabaseError::Cognitive("gateway_replay_rejected"))
    } else {
        Ok(())
    }
}

fn insert_gateway_replay_guard(
    transaction: &Transaction<'_>,
    client_id: &str,
    session_id: Option<&str>,
    message_nonce_metadata: &str,
    replay_counter: i64,
    message_kind: &str,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO gateway_replay_guards
         (id, client_id, session_id, message_nonce_metadata, replay_counter, message_kind, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::now_v7().to_string(),
            client_id,
            session_id,
            message_nonce_metadata,
            replay_counter,
            message_kind,
            now_millis(),
        ],
    )?;
    Ok(())
}

fn authenticate_gateway_session(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
    proof: &GatewaySessionProof,
    message_kind: &str,
) -> Result<GatewaySession, DatabaseError> {
    let session = load_gateway_session_tx(transaction, &proof.session_id)?;
    if session.source_agent_id != agent_id
        || session.owner_user_id != owner_user_id
        || session.transfer_id != proof.transfer_id
        || session.client_id != proof.client_id
    {
        return Err(DatabaseError::OwnershipMismatch);
    }
    if matches!(
        session.status,
        GatewaySessionStatus::Revoked | GatewaySessionStatus::Expired
    ) {
        return Err(DatabaseError::Cognitive("gateway_session_unavailable"));
    }
    if session.app_version != proof.app_version
        || session.protocol_version != proof.protocol_version
        || session.session_nonce_metadata != proof.session_nonce_metadata
        || session.auth_proof_metadata != proof.auth_proof_metadata
    {
        return Err(DatabaseError::Cognitive("gateway_authentication_failed"));
    }
    let transfer = load_gateway_transfer_tx(transaction, &session.transfer_id)?;
    validate_gateway_transfer_owner(&transfer, agent_id, owner_user_id)?;
    if transfer.status != GatewayTransferStatus::Approved {
        return Err(DatabaseError::Cognitive(
            "gateway_transfer_approval_required",
        ));
    }
    ensure_gateway_replay_nonce_available(
        transaction,
        &session.client_id,
        &proof.message_nonce_metadata,
    )?;
    ensure_gateway_replay_counter_fresh(transaction, &session.client_id, proof.replay_counter)?;
    let now = now_millis();
    transaction.execute(
        "UPDATE gateway_sessions
         SET status = 'connected', last_replay_counter = ?1, last_seen_at = ?2,
             disconnected_at = NULL, updated_at = ?2 WHERE id = ?3",
        params![proof.replay_counter, now, proof.session_id],
    )?;
    insert_gateway_replay_guard(
        transaction,
        &session.client_id,
        Some(&session.id),
        &proof.message_nonce_metadata,
        proof.replay_counter,
        message_kind,
    )?;
    load_gateway_session_tx(transaction, &proof.session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> (Database, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("aip-gateway-test-{}", Uuid::now_v7()));
        let database = Database::initialize(&path).expect("database should initialize");
        (database, path)
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
    }

    fn transfer_request(key: &str) -> GatewayTransferRequest {
        GatewayTransferRequest {
            agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            destination_account_metadata: GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA.to_owned(),
            integrity_hash: GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH.to_owned(),
            idempotency_key: key.to_owned(),
            temporary_chat: false,
        }
    }

    fn session_request(transfer_id: &str, key: &str) -> GatewaySessionRequest {
        GatewaySessionRequest {
            agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            transfer_id: transfer_id.to_owned(),
            client_id: GATEWAY_FIXTURE_CLIENT_ID.to_owned(),
            app_version: GATEWAY_FIXTURE_APP_VERSION.to_owned(),
            protocol_version: GATEWAY_PROTOCOL_VERSION,
            auth_proof_metadata: GATEWAY_FIXTURE_AUTH_PROOF_METADATA.to_owned(),
            message_nonce_metadata: format!("fixture:gateway-message/{key}"),
            replay_counter: 1,
            idempotency_key: key.to_owned(),
            temporary_chat: false,
        }
    }

    fn proof(session: &GatewaySession, counter: i64, suffix: &str) -> GatewaySessionProof {
        GatewaySessionProof {
            session_id: session.id.clone(),
            transfer_id: session.transfer_id.clone(),
            client_id: session.client_id.clone(),
            session_nonce_metadata: session.session_nonce_metadata.clone(),
            auth_proof_metadata: session.auth_proof_metadata.clone(),
            app_version: session.app_version.clone(),
            protocol_version: session.protocol_version,
            message_nonce_metadata: format!("fixture:gateway-message/{suffix}"),
            replay_counter: counter,
        }
    }

    #[test]
    fn gateway_transfer_session_recovery_and_revocation_are_local_and_bounded() {
        let (database, path) = database();
        let protocol = database
            .gateway_protocol_info(GATEWAY_FIXTURE_AGENT_ID)
            .unwrap();
        assert!(!protocol.network_listener);
        assert_eq!(protocol.transport, "local_loopback_fixture");
        assert_eq!(protocol.cloudflare.credential_state, "absent");
        assert!(protocol.standalone_fallback);
        let accounts = database
            .list_gateway_accounts(GATEWAY_FIXTURE_AGENT_ID)
            .unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, GATEWAY_FIXTURE_ACCOUNT_ID);
        assert_eq!(
            accounts[0].local_account_id,
            GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID
        );
        assert!(accounts[0].metadata_only);
        assert!(!accounts[0].external_effect_performed);

        let preview = database
            .prepare_gateway_transfer(transfer_request("transfer-prepare-1"))
            .unwrap();
        assert_eq!(preview.status, GatewayTransferStatus::Previewed);
        assert!(preview.approval_required);
        assert!(preview.metadata_only);
        assert!(!preview.external_effect_performed);

        let transfer = database
            .approve_gateway_transfer(GatewayTransferApprovalRequest {
                agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                transfer_id: preview.id.clone(),
                approved: true,
                idempotency_key: "transfer-approve-1".to_owned(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(transfer.status, GatewayTransferStatus::Approved);
        assert_eq!(transfer.authorization_status, "owner_approved");

        let session = database
            .connect_gateway_session(session_request(&transfer.id, "session-connect-1"))
            .unwrap();
        assert!(session.authenticated);
        assert!(session.local_loopback_only);
        assert_eq!(session.scope, "administrative_recovery");

        let recovery = database
            .request_gateway_recovery(GatewayRecoveryRequest {
                agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                proof: proof(&session, 2, "recovery-request-1"),
                recovery_kind: "mobile_administrative".to_owned(),
                target_metadata: GATEWAY_FIXTURE_RECOVERY_TARGET.to_owned(),
                idempotency_key: "recovery-request-1".to_owned(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(recovery.status, GatewayRecoveryStatus::PendingApproval);
        assert!(recovery.approval_required);
        assert!(!recovery.external_effect_performed);

        let approved_recovery = database
            .approve_gateway_recovery(GatewayRecoveryApprovalRequest {
                agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                proof: proof(&session, 3, "recovery-approve-1"),
                recovery_id: recovery.id.clone(),
                approved: true,
                idempotency_key: "recovery-approve-1".to_owned(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(approved_recovery.status, GatewayRecoveryStatus::Approved);

        let replay = database.approve_gateway_recovery(GatewayRecoveryApprovalRequest {
            agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
            owner_user_id: OWNER_ID.to_owned(),
            proof: proof(&session, 3, "recovery-approve-1"),
            recovery_id: recovery.id,
            approved: true,
            idempotency_key: "recovery-approve-replay".to_owned(),
            temporary_chat: false,
        });
        assert_eq!(
            replay,
            Err(DatabaseError::Cognitive("gateway_replay_rejected"))
        );

        let revocation = database
            .revoke_gateway_session(GatewaySessionActionRequest {
                agent_id: GATEWAY_FIXTURE_AGENT_ID.to_owned(),
                owner_user_id: OWNER_ID.to_owned(),
                session_id: session.id,
                reason: "revogação fixture local".to_owned(),
                idempotency_key: "session-revoke-1".to_owned(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(revocation.target_kind, "session");
        assert!(!database
            .list_gateway_audit(GATEWAY_FIXTURE_AGENT_ID)
            .unwrap()
            .is_empty());
        assert!(!database
            .list_gateway_revocations(GATEWAY_FIXTURE_AGENT_ID)
            .unwrap()
            .is_empty());
        cleanup(&path);
    }

    #[test]
    fn gateway_mutations_fail_closed_for_invalid_fixture_temporary_chat_and_safe_mode() {
        let (database, path) = database();
        let mut invalid = transfer_request("transfer-invalid");
        invalid.integrity_hash = "sha256:wrong".to_owned();
        assert_eq!(
            database.prepare_gateway_transfer(invalid),
            Err(DatabaseError::Cognitive("gateway_transfer_fixture_invalid"))
        );

        let mut temporary = transfer_request("transfer-temporary");
        temporary.temporary_chat = true;
        assert_eq!(
            database.prepare_gateway_transfer(temporary),
            Err(DatabaseError::Cognitive("gateway_blocked_temporary"))
        );

        database.set_safe_mode(true).unwrap();
        assert_eq!(
            database.prepare_gateway_transfer(transfer_request("transfer-safe-mode")),
            Err(DatabaseError::Cognitive("gateway_blocked_safe_mode"))
        );
        cleanup(&path);
    }
}
