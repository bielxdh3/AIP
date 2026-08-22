#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError, OWNER_ID};

pub const AIP_EXTENSION_SDK_VERSION: &str = "aip-extension-sdk/v1";
const EXTENSION_MANIFEST_VERSION: i64 = 1;
const MAX_EXTENSION_ID_BYTES: usize = 96;
const MAX_EXTENSION_CAPABILITIES: usize = 8;
const MAX_EXTENSION_MANIFEST_BYTES: usize = 8_192;
const MAX_PACKAGE_INSTRUCTIONS: usize = 32;
const MAX_PACKAGE_TEXT_BYTES: usize = 4_096;
const MAX_EXECUTION_INPUT_BYTES: usize = 4_096;
const MAX_EXECUTION_OUTPUT_BYTES: usize = 8_192;
const MAX_EXECUTION_STEPS: usize = 32;
const MAX_EXECUTION_MS: i64 = 5_000;
const MAX_EXTENSION_AUDIT_BYTES: usize = 2_048;
const MAX_EXTENSION_AUDIT_ROWS: i64 = 100;
const EXTENSION_AUDIT_RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;
const MAX_EXTENSION_IDEMPOTENCY_REQUEST_BYTES: usize = 16_384;
const MAX_EXTENSION_IDEMPOTENCY_RESULT_BYTES: usize = 32_768;

const EXTENSION_OPERATION_CREATE: &str = "create";
const EXTENSION_OPERATION_REVIEW: &str = "review";
const EXTENSION_OPERATION_ACTIVATE: &str = "activate";
const EXTENSION_OPERATION_UPDATE: &str = "update";
const EXTENSION_OPERATION_ROLLBACK: &str = "rollback";
const EXTENSION_OPERATION_DISABLE: &str = "disable";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionCapability {
    AgentContext,
    ToolCatalog,
    OwnerReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSandboxPolicy {
    MetadataOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionAdmissionPolicy {
    LocalFixtureOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ExtensionInstruction {
    EmitText {
        text: Option<String>,
        echo_input: Option<bool>,
    },
    ReadAgentContext,
    ListToolCatalog,
    Yield,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPackage {
    pub format: String,
    pub entrypoint: String,
    pub instructions: Vec<ExtensionInstruction>,
    pub integrity_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSourceKind {
    AdministratorSelected,
    AgentCreated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionCatalogScope {
    PrivateLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionLifecycle {
    ReviewRequired,
    Approved,
    Active,
    Disabled,
    Rejected,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionReviewStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionProposalStatus {
    Pending,
    Approved,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionPermissionStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifest {
    pub extension_id: String,
    pub manifest_version: i64,
    pub extension_version: String,
    pub sdk_version: String,
    pub name: String,
    pub sandbox_policy: ExtensionSandboxPolicy,
    pub admission_policy: ExtensionAdmissionPolicy,
    pub capabilities: Vec<ExtensionCapability>,
    pub local_fixture_ref: Option<String>,
    pub untrusted: bool,
    #[serde(default)]
    pub package: Option<ExtensionPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionExecutionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub extension_id: String,
    pub revision: i64,
    pub package_hash: String,
    pub input: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionExecutionResult {
    pub execution_id: String,
    pub status: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub steps: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionExecutionCancellationRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub execution_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionHostContext {
    pub agent_id: String,
    pub tool_ids: Vec<String>,
}

pub fn extension_package_hash(package: &ExtensionPackage) -> Result<String, DatabaseError> {
    let mut payload = serde_json::to_value(package).map_err(|_| DatabaseError::Unavailable)?;
    payload
        .as_object_mut()
        .ok_or(DatabaseError::Cognitive("extension_package_invalid"))?
        .remove("integritySha256");
    let payload = serde_json::to_vec(&payload).map_err(|_| DatabaseError::Unavailable)?;
    Ok(format!("{:x}", Sha256::digest(payload)))
}

fn validate_package(package: &ExtensionPackage) -> Result<(), DatabaseError> {
    if package.format != "aip-extension-package/v1"
        || package.entrypoint != "main"
        || package.instructions.is_empty()
        || package.instructions.len() > MAX_PACKAGE_INSTRUCTIONS
        || package.integrity_sha256.len() != 64
        || !package
            .integrity_sha256
            .chars()
            .all(|c| c.is_ascii_hexdigit())
        || extension_package_hash(package)? != package.integrity_sha256
    {
        return Err(DatabaseError::Cognitive("extension_package_invalid"));
    }
    let mut seen = HashSet::new();
    for instruction in &package.instructions {
        let encoded = serde_json::to_string(instruction).map_err(|_| DatabaseError::Unavailable)?;
        if !seen.insert(encoded) {
            return Err(DatabaseError::Cognitive("extension_instruction_duplicate"));
        }
        if let ExtensionInstruction::EmitText { text, echo_input } = instruction {
            if text.is_none() && *echo_input != Some(true) {
                return Err(DatabaseError::Cognitive("extension_instruction_invalid"));
            }
            if text
                .as_deref()
                .is_some_and(|v| v.len() > MAX_PACKAGE_TEXT_BYTES)
            {
                return Err(DatabaseError::Cognitive("extension_instruction_oversized"));
            }
        }
    }
    Ok(())
}

pub fn interpret_extension_package(
    package: &ExtensionPackage,
    input: &str,
    capabilities: &[ExtensionCapability],
    host: &ExtensionHostContext,
) -> Result<String, DatabaseError> {
    validate_package(package)?;
    if input.len() > MAX_EXECUTION_INPUT_BYTES {
        return Err(DatabaseError::Cognitive("extension_input_oversized"));
    }
    let mut output = String::new();
    for instruction in &package.instructions {
        match instruction {
            ExtensionInstruction::EmitText { text, echo_input } => {
                output.push_str(text.as_deref().unwrap_or(if *echo_input == Some(true) {
                    input
                } else {
                    ""
                }))
            }
            ExtensionInstruction::ReadAgentContext => {
                if !capabilities.contains(&ExtensionCapability::AgentContext) {
                    return Err(DatabaseError::Cognitive("extension_capability_denied"));
                }
                output.push_str("agent_id:");
                output.push_str(&host.agent_id);
            }
            ExtensionInstruction::ListToolCatalog => {
                if !capabilities.contains(&ExtensionCapability::ToolCatalog) {
                    return Err(DatabaseError::Cognitive("extension_capability_denied"));
                }
                output.push_str("tool_ids:");
                output.push_str(&host.tool_ids.join(","));
            }
            ExtensionInstruction::Yield => {}
        }
        if output.len() > MAX_EXECUTION_OUTPUT_BYTES {
            return Err(DatabaseError::Cognitive("extension_output_oversized"));
        }
    }
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPermissionRequest {
    pub capability: ExtensionCapability,
    pub status: ExtensionPermissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCatalogEntry {
    pub extension_id: String,
    pub catalog_scope: ExtensionCatalogScope,
    pub source_kind: ExtensionSourceKind,
    pub lifecycle: ExtensionLifecycle,
    pub review_status: ExtensionReviewStatus,
    pub manifest: ExtensionManifest,
    pub current_revision: i64,
    pub active_revision: Option<i64>,
    pub approved_capabilities: Vec<ExtensionCapability>,
    pub compatible: bool,
    pub untrusted: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionProposal {
    pub id: String,
    pub extension_id: String,
    pub revision: i64,
    pub source_kind: ExtensionSourceKind,
    pub proposer_agent_id: Option<String>,
    pub status: ExtensionProposalStatus,
    pub review_status: ExtensionReviewStatus,
    pub manifest: ExtensionManifest,
    pub requested_capabilities: Vec<ExtensionCapability>,
    pub approved_capabilities: Vec<ExtensionCapability>,
    pub permissions: Vec<ExtensionPermissionRequest>,
    pub compatible: bool,
    pub review_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionAuditRecord {
    pub id: String,
    pub extension_id: Option<String>,
    pub proposal_id: Option<String>,
    pub revision: Option<i64>,
    pub agent_id: String,
    pub event: String,
    pub result: String,
    pub code: Option<String>,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionProposalRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub source_kind: ExtensionSourceKind,
    pub proposer_agent_id: Option<String>,
    pub manifest: ExtensionManifest,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionAgentProposalRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub manifest: ExtensionManifest,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionUpdateRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub extension_id: String,
    pub source_kind: ExtensionSourceKind,
    pub proposer_agent_id: Option<String>,
    pub manifest: ExtensionManifest,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionReviewRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub proposal_id: String,
    pub approved: bool,
    pub approved_capabilities: Vec<ExtensionCapability>,
    pub reason: Option<String>,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionActivationRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub extension_id: String,
    pub proposal_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRollbackRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub extension_id: String,
    pub target_revision: i64,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDisableRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub extension_id: String,
    pub reason: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone)]
struct CatalogRow {
    extension_id: String,
    catalog_scope: ExtensionCatalogScope,
    source_kind: ExtensionSourceKind,
    lifecycle: ExtensionLifecycle,
    current_revision: i64,
    active_revision: Option<i64>,
    untrusted: bool,
}

#[derive(Debug, Serialize)]
struct ProposalInsert<'a> {
    manifest: &'a ExtensionManifest,
    revision: i64,
    owner_id: &'a str,
    source_kind: &'a ExtensionSourceKind,
    proposer_agent_id: Option<&'a str>,
    idempotency_key: &'a str,
    now: i64,
}

#[derive(Debug, Serialize, Deserialize)]
enum StoredExtensionResult {
    Proposal(ExtensionProposal),
    Catalog(ExtensionCatalogEntry),
}

#[derive(Debug, Serialize)]
struct AuditDetails<'a> {
    summary: &'a str,
}

struct AuditContext<'a> {
    extension_id: Option<&'a str>,
    proposal_id: Option<&'a str>,
    revision: Option<i64>,
    agent_id: &'a str,
    owner_id: &'a str,
    event: &'a str,
    result: &'a str,
    code: Option<&'a str>,
    summary: &'a str,
}

impl Database {
    pub fn list_extension_catalog(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ExtensionCatalogEntry>, DatabaseError> {
        let connection = self.open()?;
        ensure_owner_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT extension_id FROM extension_catalog
             WHERE owner_user_id = ?1 ORDER BY updated_at DESC, extension_id",
        )?;
        let ids = statement
            .query_map(params![OWNER_ID], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_catalog_entry_connection(&connection, &id))
            .collect()
    }

    pub fn list_extension_proposals(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ExtensionProposal>, DatabaseError> {
        let connection = self.open()?;
        ensure_owner_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id FROM extension_proposals
             WHERE owner_user_id = ?1 ORDER BY updated_at DESC, id DESC LIMIT 64",
        )?;
        let ids = statement
            .query_map(params![OWNER_ID], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_proposal_connection(&connection, &id))
            .collect()
    }

    pub fn list_extension_audit(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ExtensionAuditRecord>, DatabaseError> {
        let connection = self.open()?;
        ensure_owner_agent(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, extension_id, proposal_id, revision, agent_id, event, result, code,
                    details_json, created_at
             FROM extension_audit_log
             WHERE owner_user_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let records = statement
            .query_map(
                params![OWNER_ID, MAX_EXTENSION_AUDIT_ROWS],
                map_audit_record,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from);
        records
    }

    pub fn create_extension_proposal(
        &self,
        mut request: ExtensionProposalRequest,
    ) -> Result<ExtensionProposal, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.idempotency_key = valid_idempotency(&request.idempotency_key)?;
        request.manifest = normalize_manifest(request.manifest)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let owner_id = ensure_owner_actor_tx(&transaction, &request.owner_user_id)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, &owner_id)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let proposer_agent_id = validate_source(
            &transaction,
            &request.source_kind,
            request.proposer_agent_id.as_deref(),
            &request.agent_id,
            &owner_id,
        )?;
        let request_json = request_fingerprint(EXTENSION_OPERATION_CREATE, &request)?;
        if let Some(proposal) = replay_proposal_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_CREATE,
            &request.idempotency_key,
            &request_json,
        )? {
            return Ok(proposal);
        }
        if transaction.query_row(
            "SELECT EXISTS(
                   SELECT 1 FROM extension_catalog
                   WHERE extension_id = ?1 AND owner_user_id = ?2
                 )",
            params![request.manifest.extension_id, owner_id],
            |row| row.get::<_, bool>(0),
        )? {
            return Err(DatabaseError::Cognitive("extension_already_exists"));
        }
        let now = now_millis();
        let extension_id = request.manifest.extension_id.clone();
        insert_catalog_tx(
            &transaction,
            &extension_id,
            &owner_id,
            &request.source_kind,
            ExtensionLifecycle::ReviewRequired,
            proposer_agent_id.as_deref(),
            now,
        )?;
        insert_manifest_tx(&transaction, &request.manifest, 1, now)?;
        let proposal_id = insert_proposal_tx(
            &transaction,
            ProposalInsert {
                manifest: &request.manifest,
                revision: 1,
                owner_id: &owner_id,
                source_kind: &request.source_kind,
                proposer_agent_id: proposer_agent_id.as_deref(),
                idempotency_key: &request.idempotency_key,
                now,
            },
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                extension_id: Some(&extension_id),
                proposal_id: Some(&proposal_id),
                revision: Some(1),
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                event: "proposal_created",
                result: "pending_review",
                code: None,
                summary: "Proposta de extensão local criada para revisão do Owner.",
            },
        )?;
        let proposal = load_proposal_tx(&transaction, &proposal_id)?;
        record_idempotency_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_CREATE,
            &request.idempotency_key,
            &request_json,
            &StoredExtensionResult::Proposal(proposal.clone()),
            now,
        )?;
        transaction.commit()?;
        Ok(proposal)
    }

    pub fn create_agent_extension_proposal(
        &self,
        request: ExtensionAgentProposalRequest,
    ) -> Result<ExtensionProposal, DatabaseError> {
        let agent_id = request.agent_id.clone();
        self.create_extension_proposal(ExtensionProposalRequest {
            agent_id: agent_id.clone(),
            owner_user_id: request.owner_user_id,
            source_kind: ExtensionSourceKind::AgentCreated,
            proposer_agent_id: Some(agent_id),
            manifest: request.manifest,
            idempotency_key: request.idempotency_key,
            temporary_chat: request.temporary_chat,
        })
    }

    pub fn update_extension(
        &self,
        mut request: ExtensionUpdateRequest,
    ) -> Result<ExtensionProposal, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.idempotency_key = valid_idempotency(&request.idempotency_key)?;
        if request.extension_id != request.manifest.extension_id {
            return Err(DatabaseError::Cognitive("extension_identity_invalid"));
        }
        request.manifest = normalize_manifest(request.manifest)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let owner_id = ensure_owner_actor_tx(&transaction, &request.owner_user_id)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, &owner_id)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let proposer_agent_id = validate_source(
            &transaction,
            &request.source_kind,
            request.proposer_agent_id.as_deref(),
            &request.agent_id,
            &owner_id,
        )?;
        let request_json = request_fingerprint(EXTENSION_OPERATION_UPDATE, &request)?;
        if let Some(proposal) = replay_proposal_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_UPDATE,
            &request.idempotency_key,
            &request_json,
        )? {
            return Ok(proposal);
        }
        let current = load_catalog_row_tx(&transaction, &request.extension_id)?;
        if current.source_kind != request.source_kind {
            return Err(DatabaseError::Cognitive("extension_source_invalid"));
        }
        let next_revision: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1
             FROM extension_manifest_revisions WHERE extension_id = ?1",
            params![request.extension_id],
            |row| row.get(0),
        )?;
        if next_revision > i64::from(i32::MAX) {
            return Err(DatabaseError::Cognitive("extension_revision_invalid"));
        }
        let now = now_millis();
        insert_manifest_tx(&transaction, &request.manifest, next_revision, now)?;
        let proposal_id = insert_proposal_tx(
            &transaction,
            ProposalInsert {
                manifest: &request.manifest,
                revision: next_revision,
                owner_id: &owner_id,
                source_kind: &request.source_kind,
                proposer_agent_id: proposer_agent_id.as_deref(),
                idempotency_key: &request.idempotency_key,
                now,
            },
        )?;
        transaction.execute(
            "UPDATE extension_catalog
             SET current_revision = ?1, active_revision = NULL, lifecycle = 'disabled', updated_at = ?2
             WHERE extension_id = ?3 AND owner_user_id = ?4",
            params![next_revision, now, request.extension_id, owner_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                extension_id: Some(&request.extension_id),
                proposal_id: Some(&proposal_id),
                revision: Some(next_revision),
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                event: "update_created",
                result: "disabled_pending_review",
                code: Some("extension_update_requires_review"),
                summary:
                    "Atualização registrada; a extensão permanece desativada até nova revisão.",
            },
        )?;
        let proposal = load_proposal_tx(&transaction, &proposal_id)?;
        record_idempotency_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_UPDATE,
            &request.idempotency_key,
            &request_json,
            &StoredExtensionResult::Proposal(proposal.clone()),
            now,
        )?;
        transaction.commit()?;
        Ok(proposal)
    }

    pub fn review_extension_proposal(
        &self,
        mut request: ExtensionReviewRequest,
    ) -> Result<ExtensionProposal, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.idempotency_key = valid_idempotency(&request.idempotency_key)?;
        request.approved_capabilities = normalize_capabilities(request.approved_capabilities)?;
        request.reason = request
            .reason
            .map(|value| validate_text(&value, 512))
            .transpose()?;
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let owner_id = ensure_owner_actor_tx(&transaction, &request.owner_user_id)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, &owner_id)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let request_json = request_fingerprint(EXTENSION_OPERATION_REVIEW, &request)?;
        if let Some(proposal) = replay_proposal_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_REVIEW,
            &request.idempotency_key,
            &request_json,
        )? {
            return Ok(proposal);
        }
        let proposal = load_proposal_tx(&transaction, &request.proposal_id)?;
        let manifest = proposal.manifest.clone();
        if proposal.status != ExtensionProposalStatus::Pending {
            return Err(DatabaseError::Cognitive("extension_proposal_invalid"));
        }
        if proposal.source_kind == ExtensionSourceKind::AgentCreated
            && proposal.proposer_agent_id.as_deref() == Some(request.agent_id.as_str())
        {
            return Err(DatabaseError::Cognitive("extension_proposal_self_review"));
        }
        if !proposal.compatible {
            return Err(DatabaseError::Cognitive("extension_sdk_incompatible"));
        }
        let approved_capabilities = request.approved_capabilities.clone();
        if !request.approved && !approved_capabilities.is_empty() {
            return Err(DatabaseError::Cognitive("extension_permission_invalid"));
        }
        if request.approved
            && approved_capabilities
                .iter()
                .any(|capability| !manifest.capabilities.contains(capability))
        {
            return Err(DatabaseError::Cognitive("extension_permission_invalid"));
        }
        let reason = request.reason.clone();
        if !request.approved && reason.is_none() {
            return Err(DatabaseError::Cognitive("extension_review_reason_required"));
        }
        let now = now_millis();
        let status = if request.approved {
            ExtensionProposalStatus::Approved
        } else {
            ExtensionProposalStatus::Rejected
        };
        let review_status = if request.approved {
            ExtensionReviewStatus::Approved
        } else {
            ExtensionReviewStatus::Rejected
        };
        let approved_json = if request.approved {
            Some(serialize_capabilities(&approved_capabilities)?)
        } else {
            None
        };
        transaction.execute(
            "UPDATE extension_proposals
             SET status = ?1, approved_capabilities_json = ?2, review_reason = ?3, updated_at = ?4
             WHERE id = ?5 AND owner_user_id = ?6",
            params![
                proposal_status_kind(&status),
                approved_json,
                reason,
                now,
                request.proposal_id,
                owner_id,
            ],
        )?;
        transaction.execute(
            "UPDATE extension_manifest_revisions
             SET review_status = ?1, updated_at = ?2
             WHERE extension_id = ?3 AND revision = ?4",
            params![
                review_status_kind(&review_status),
                now,
                proposal.extension_id,
                proposal.revision,
            ],
        )?;
        for capability in &manifest.capabilities {
            let permission_status =
                if request.approved && approved_capabilities.contains(capability) {
                    ExtensionPermissionStatus::Approved
                } else {
                    ExtensionPermissionStatus::Denied
                };
            transaction.execute(
                "UPDATE extension_permission_requests
                 SET status = ?1, updated_at = ?2
                 WHERE proposal_id = ?3 AND capability = ?4",
                params![
                    permission_status_kind(&permission_status),
                    now,
                    request.proposal_id,
                    capability_kind(capability),
                ],
            )?;
        }
        transaction.execute(
            "UPDATE extension_catalog SET lifecycle = ?1, updated_at = ?2
             WHERE extension_id = ?3 AND owner_user_id = ?4 AND current_revision = ?5",
            params![
                if request.approved {
                    "approved"
                } else {
                    "rejected"
                },
                now,
                proposal.extension_id,
                owner_id,
                proposal.revision,
            ],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                extension_id: Some(&proposal.extension_id),
                proposal_id: Some(&proposal.id),
                revision: Some(proposal.revision),
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                event: if request.approved {
                    "proposal_approved"
                } else {
                    "proposal_rejected"
                },
                result: if request.approved {
                    "approved"
                } else {
                    "rejected"
                },
                code: None,
                summary: if request.approved {
                    "Proposta aprovada pelo Owner; a ativação ainda exige ação explícita."
                } else {
                    "Proposta rejeitada pelo Owner."
                },
            },
        )?;
        let reviewed = load_proposal_tx(&transaction, &request.proposal_id)?;
        record_idempotency_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_REVIEW,
            &request.idempotency_key,
            &request_json,
            &StoredExtensionResult::Proposal(reviewed.clone()),
            now,
        )?;
        transaction.commit()?;
        Ok(reviewed)
    }

    pub fn activate_extension(
        &self,
        mut request: ExtensionActivationRequest,
    ) -> Result<ExtensionCatalogEntry, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let owner_id = ensure_owner_actor_tx(&transaction, &request.owner_user_id)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, &owner_id)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let request_json = request_fingerprint(EXTENSION_OPERATION_ACTIVATE, &request)?;
        if let Some(entry) = replay_catalog_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_ACTIVATE,
            &request.idempotency_key,
            &request_json,
        )? {
            return Ok(entry);
        }
        let catalog = load_catalog_row_tx(&transaction, &request.extension_id)?;
        let proposal = load_proposal_tx(&transaction, &request.proposal_id)?;
        if proposal.extension_id != request.extension_id
            || proposal.revision != catalog.current_revision
        {
            return Err(DatabaseError::Cognitive("extension_update_requires_review"));
        }
        if proposal.status != ExtensionProposalStatus::Approved
            || proposal.review_status != ExtensionReviewStatus::Approved
        {
            return Err(DatabaseError::Cognitive("extension_review_required"));
        }
        if !proposal.compatible {
            return Err(DatabaseError::Cognitive("extension_sdk_incompatible"));
        }
        if proposal
            .permissions
            .iter()
            .any(|permission| permission.status == ExtensionPermissionStatus::Pending)
        {
            return Err(DatabaseError::Cognitive("extension_permission_required"));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE extension_catalog
             SET lifecycle = 'active', active_revision = ?1, updated_at = ?2
             WHERE extension_id = ?3 AND owner_user_id = ?4 AND current_revision = ?1",
            params![proposal.revision, now, request.extension_id, owner_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                extension_id: Some(&request.extension_id),
                proposal_id: Some(&request.proposal_id),
                revision: Some(proposal.revision),
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                event: "extension_activated",
                result: "active",
                code: None,
                summary: "Extensão metadata-only ativada explicitamente pelo Owner.",
            },
        )?;
        let entry = load_catalog_entry_tx(&transaction, &request.extension_id)?;
        record_idempotency_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_ACTIVATE,
            &request.idempotency_key,
            &request_json,
            &StoredExtensionResult::Catalog(entry.clone()),
            now,
        )?;
        transaction.commit()?;
        Ok(entry)
    }

    pub fn rollback_extension(
        &self,
        mut request: ExtensionRollbackRequest,
    ) -> Result<ExtensionCatalogEntry, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.idempotency_key = valid_idempotency(&request.idempotency_key)?;
        if request.target_revision < 1 {
            return Err(DatabaseError::Cognitive("extension_revision_invalid"));
        }
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let owner_id = ensure_owner_actor_tx(&transaction, &request.owner_user_id)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, &owner_id)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let request_json = request_fingerprint(EXTENSION_OPERATION_ROLLBACK, &request)?;
        if let Some(entry) = replay_catalog_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_ROLLBACK,
            &request.idempotency_key,
            &request_json,
        )? {
            return Ok(entry);
        }
        let catalog = load_catalog_row_tx(&transaction, &request.extension_id)?;
        if request.target_revision >= catalog.current_revision {
            return Err(DatabaseError::Cognitive("extension_rollback_unavailable"));
        }
        let proposal_id = transaction
            .query_row(
                "SELECT id FROM extension_proposals
                 WHERE extension_id = ?1 AND revision = ?2 AND owner_user_id = ?3
                   AND status = 'approved'
                 LIMIT 1",
                params![request.extension_id, request.target_revision, owner_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(DatabaseError::Cognitive("extension_rollback_unavailable"))?;
        let proposal = load_proposal_tx(&transaction, &proposal_id)?;
        if proposal.review_status != ExtensionReviewStatus::Approved || !proposal.compatible {
            return Err(DatabaseError::Cognitive("extension_rollback_unavailable"));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE extension_catalog
             SET current_revision = ?1, active_revision = ?1, lifecycle = 'active', updated_at = ?2
             WHERE extension_id = ?3 AND owner_user_id = ?4",
            params![request.target_revision, now, request.extension_id, owner_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                extension_id: Some(&request.extension_id),
                proposal_id: Some(&proposal_id),
                revision: Some(request.target_revision),
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                event: "extension_rolled_back",
                result: "active_prior_approved_manifest",
                code: None,
                summary: "Extensão restaurada para um manifest aprovado anteriormente.",
            },
        )?;
        let entry = load_catalog_entry_tx(&transaction, &request.extension_id)?;
        record_idempotency_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_ROLLBACK,
            &request.idempotency_key,
            &request_json,
            &StoredExtensionResult::Catalog(entry.clone()),
            now,
        )?;
        transaction.commit()?;
        Ok(entry)
    }

    pub fn disable_extension(
        &self,
        mut request: ExtensionDisableRequest,
    ) -> Result<ExtensionCatalogEntry, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        request.idempotency_key = valid_idempotency(&request.idempotency_key)?;
        request.reason = validate_text(&request.reason, 512)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let owner_id = ensure_owner_actor_tx(&transaction, &request.owner_user_id)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, &owner_id)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let request_json = request_fingerprint(EXTENSION_OPERATION_DISABLE, &request)?;
        if let Some(entry) = replay_catalog_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_DISABLE,
            &request.idempotency_key,
            &request_json,
        )? {
            return Ok(entry);
        }
        load_catalog_row_tx(&transaction, &request.extension_id)?;
        let now = now_millis();
        transaction.execute(
            "UPDATE extension_catalog
             SET lifecycle = 'disabled', active_revision = NULL, updated_at = ?1
             WHERE extension_id = ?2 AND owner_user_id = ?3",
            params![now, request.extension_id, owner_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                extension_id: Some(&request.extension_id),
                proposal_id: None,
                revision: None,
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                event: "extension_disabled",
                result: "disabled",
                code: None,
                summary: &format!("Extensão desativada pelo Owner: {}", request.reason),
            },
        )?;
        let entry = load_catalog_entry_tx(&transaction, &request.extension_id)?;
        record_idempotency_tx(
            &transaction,
            &owner_id,
            EXTENSION_OPERATION_DISABLE,
            &request.idempotency_key,
            &request_json,
            &StoredExtensionResult::Catalog(entry.clone()),
            now,
        )?;
        transaction.commit()?;
        Ok(entry)
    }
}

impl Database {
    pub fn execute_extension(
        &self,
        request: ExtensionExecutionRequest,
    ) -> Result<ExtensionExecutionResult, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        if request.owner_user_id != OWNER_ID
            || request.input.len() > MAX_EXECUTION_INPUT_BYTES
            || request.idempotency_key.len() > 128
        {
            return Err(DatabaseError::Cognitive("extension_execution_denied"));
        }
        static ACTIVE: OnceLock<Mutex<()>> = OnceLock::new();
        let _active = match ACTIVE.get_or_init(|| Mutex::new(())).lock() {
            Ok(guard) => guard,
            Err(_) => return Err(DatabaseError::Cognitive("extension_execution_busy")),
        };
        let mut connection = self.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_owner_agent_tx(&transaction, &request.agent_id, OWNER_ID)?;
        ensure_extensions_enabled_tx(&transaction)?;
        let agent_state: (String, bool) = transaction.query_row(
            "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id=?1",
            params![request.agent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if agent_state.1 || agent_state.0 == "safe" {
            return Err(DatabaseError::Cognitive(
                "extension_execution_blocked_suspended",
            ));
        }
        let catalog = load_catalog_row_tx(&transaction, &request.extension_id)?;
        if catalog.lifecycle != ExtensionLifecycle::Active
            || catalog.active_revision != Some(request.revision)
            || !catalog.untrusted
        {
            return Err(DatabaseError::Cognitive("extension_execution_denied"));
        }
        let (manifest, _, _) =
            load_manifest_tx(&transaction, &request.extension_id, request.revision)?;
        let package = manifest
            .package
            .as_ref()
            .ok_or(DatabaseError::Cognitive("extension_package_required"))?;
        if extension_package_hash(package)? != request.package_hash.to_ascii_lowercase() {
            return Err(DatabaseError::Cognitive("extension_integrity_mismatch"));
        }
        let approved: Vec<ExtensionCapability> = transaction
            .query_row("SELECT approved_capabilities_json FROM extension_proposals WHERE extension_id=?1 AND revision=?2 AND status='approved'", params![request.extension_id, request.revision], |r| r.get::<_, Option<String>>(0))?
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default();
        let id = Uuid::now_v7().to_string();
        let now = now_millis();
        let request_json =
            serde_json::to_string(&request).map_err(|_| DatabaseError::Unavailable)?;
        let request_hash = format!("{:x}", Sha256::digest(request_json.as_bytes()));
        if let Some((old_hash, old_result)) = transaction
            .query_row("SELECT request_hash,result_json FROM extension_execution_idempotency WHERE owner_user_id=?1 AND idempotency_key=?2", params![OWNER_ID, request.idempotency_key], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .optional()?
        {
            if old_hash != request_hash { return Err(DatabaseError::Cognitive("idempotency_conflict")); }
            return serde_json::from_str(&old_result).map_err(|_| DatabaseError::Unavailable);
        }
        transaction.execute("INSERT INTO extension_executions(id,owner_user_id,agent_id,extension_id,revision,package_hash,input,status,steps,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)", params![id, OWNER_ID, request.agent_id, request.extension_id, request.revision, request.package_hash.to_ascii_lowercase(), request.input, "running", 0_i64, now, now])?;
        transaction.commit()?;
        drop(connection);

        let tool_ids = self
            .list_tool_catalog()?
            .into_iter()
            .map(|tool| tool.tool_id)
            .take(64)
            .collect::<Vec<_>>();
        let host = ExtensionHostContext {
            agent_id: request.agent_id.clone(),
            tool_ids,
        };
        validate_host_context(&host)?;

        let started = now_millis();
        let mut output = String::new();
        let mut error = None;
        let mut status = "succeeded";
        let mut steps = 0_i64;
        for instruction in &package.instructions {
            steps += 1;
            if steps as usize > MAX_EXECUTION_STEPS || now_millis() - started > MAX_EXECUTION_MS {
                status = "terminated";
                error = Some("extension_execution_limit".to_string());
                break;
            }
            let check = self.open()?.query_row(
                "SELECT cancellation_requested FROM extension_executions WHERE id=?1",
                params![id],
                |r| r.get::<_, bool>(0),
            );
            if check.unwrap_or(true) {
                status = "terminated";
                error = Some("extension_execution_cancelled".to_string());
                break;
            }
            let single = ExtensionPackage {
                format: package.format.clone(),
                entrypoint: package.entrypoint.clone(),
                instructions: vec![instruction.clone()],
                integrity_sha256: extension_package_hash(&ExtensionPackage {
                    format: package.format.clone(),
                    entrypoint: package.entrypoint.clone(),
                    instructions: vec![instruction.clone()],
                    integrity_sha256: String::new(),
                })?,
            };
            match interpret_extension_package(&single, &request.input, &approved, &host) {
                Ok(part) => {
                    output.push_str(&part);
                    if output.len() > MAX_EXECUTION_OUTPUT_BYTES {
                        status = "failed";
                        error = Some("extension_output_oversized".into());
                        break;
                    }
                }
                Err(e) => {
                    status = "failed";
                    error = Some(e.code().into());
                    break;
                }
            }
        }
        let result = ExtensionExecutionResult {
            execution_id: id.clone(),
            status: status.into(),
            output: (status == "succeeded").then_some(output),
            error,
            steps,
        };
        let result_json = serde_json::to_string(&result).map_err(|_| DatabaseError::Unavailable)?;
        let mut final_connection = self.open()?;
        let final_tx =
            final_connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        final_tx.execute("UPDATE extension_executions SET status=?1,output=?2,error=?3,steps=?4,updated_at=?5 WHERE id=?6 AND status='running'", params![result.status, result.output, result.error, result.steps, now_millis(), id])?;
        final_tx.execute("INSERT INTO extension_execution_idempotency(owner_user_id,idempotency_key,request_hash,result_json,created_at) VALUES(?,?,?,?,?)", params![OWNER_ID, request.idempotency_key, request_hash, result_json, now_millis()])?;
        final_tx.commit()?;
        Ok(result)
    }

    pub fn cancel_extension_execution(
        &self,
        request: ExtensionExecutionCancellationRequest,
    ) -> Result<(), DatabaseError> {
        if request.owner_user_id != OWNER_ID {
            return Err(DatabaseError::OwnershipMismatch);
        }
        let connection = self.open()?;
        let changed = connection.execute("UPDATE extension_executions SET cancellation_requested=1,status='terminated',updated_at=?1 WHERE id=?2 AND owner_user_id=?3 AND agent_id=?4 AND status='running'", params![now_millis(), request.execution_id, OWNER_ID, request.agent_id])?;
        if changed == 0 {
            return Err(DatabaseError::Cognitive("extension_execution_not_found"));
        }
        Ok(())
    }
}

fn validate_host_context(host: &ExtensionHostContext) -> Result<(), DatabaseError> {
    if host.agent_id.is_empty()
        || host.agent_id.len() > 96
        || !host
            .agent_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
        || host.tool_ids.len() > 64
        || host.tool_ids.iter().any(|tool_id| {
            tool_id.is_empty()
                || tool_id.len() > 128
                || tool_id
                    .chars()
                    .any(|character| character == '\0' || character.is_control())
        })
    {
        return Err(DatabaseError::Cognitive("extension_host_context_invalid"));
    }
    Ok(())
}

fn insert_catalog_tx(
    transaction: &Transaction<'_>,
    extension_id: &str,
    owner_id: &str,
    source_kind: &ExtensionSourceKind,
    lifecycle: ExtensionLifecycle,
    created_by_agent_id: Option<&str>,
    now: i64,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO extension_catalog
         (extension_id, owner_user_id, catalog_scope, source_kind, lifecycle,
          current_revision, active_revision, untrusted, created_by_agent_id, created_at, updated_at)
         VALUES (?1, ?2, 'private_local', ?3, ?4, 1, NULL, 1, ?5, ?6, ?6)",
        params![
            extension_id,
            owner_id,
            source_kind_kind(source_kind),
            lifecycle_kind(&lifecycle),
            created_by_agent_id,
            now,
        ],
    )?;
    Ok(())
}

fn insert_manifest_tx(
    transaction: &Transaction<'_>,
    manifest: &ExtensionManifest,
    revision: i64,
    now: i64,
) -> Result<(), DatabaseError> {
    let capabilities_json = serialize_capabilities(&manifest.capabilities)?;
    let manifest_json = serde_json::to_string(manifest).map_err(|_| DatabaseError::Unavailable)?;
    if manifest_json.len() > MAX_EXTENSION_MANIFEST_BYTES {
        return Err(DatabaseError::Cognitive("extension_manifest_oversized"));
    }
    let compatible = manifest_is_compatible(manifest);
    transaction.execute(
        "INSERT INTO extension_manifest_revisions
         (extension_id, revision, manifest_version, extension_version, sdk_version, name,
          sandbox_policy, admission_policy, capabilities_json, local_fixture_ref, manifest_json,
          compatible, review_status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'pending', ?13, ?13)",
        params![
            manifest.extension_id,
            revision,
            manifest.manifest_version,
            manifest.extension_version,
            manifest.sdk_version,
            manifest.name,
            sandbox_policy_kind(&manifest.sandbox_policy),
            admission_policy_kind(&manifest.admission_policy),
            capabilities_json,
            manifest.local_fixture_ref,
            manifest_json,
            compatible,
            now,
        ],
    )?;
    Ok(())
}

fn insert_proposal_tx(
    transaction: &Transaction<'_>,
    input: ProposalInsert<'_>,
) -> Result<String, DatabaseError> {
    let proposal_id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO extension_proposals
         (id, extension_id, revision, owner_user_id, source_kind, proposer_agent_id,
          status, approved_capabilities_json, review_reason, idempotency_key, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', NULL, NULL, ?7, ?8, ?8)",
        params![
            proposal_id,
            input.manifest.extension_id,
            input.revision,
            input.owner_id,
            source_kind_kind(input.source_kind),
            input.proposer_agent_id,
            input.idempotency_key,
            input.now,
        ],
    )?;
    for capability in &input.manifest.capabilities {
        transaction.execute(
            "INSERT INTO extension_permission_requests
             (proposal_id, capability, status, created_at, updated_at)
             VALUES (?1, ?2, 'pending', ?3, ?3)",
            params![proposal_id, capability_kind(capability), input.now],
        )?;
    }
    Ok(proposal_id)
}

fn load_catalog_entry_connection(
    connection: &Connection,
    extension_id: &str,
) -> Result<ExtensionCatalogEntry, DatabaseError> {
    let transaction = connection.unchecked_transaction()?;
    let entry = load_catalog_entry_tx(&transaction, extension_id)?;
    transaction.rollback()?;
    Ok(entry)
}

fn load_catalog_entry_tx(
    transaction: &Transaction<'_>,
    extension_id: &str,
) -> Result<ExtensionCatalogEntry, DatabaseError> {
    let row = load_catalog_row_tx(transaction, extension_id)?;
    let (manifest, compatible, review_status) =
        load_manifest_tx(transaction, extension_id, row.current_revision)?;
    let approved_capabilities = transaction
        .query_row(
            "SELECT approved_capabilities_json FROM extension_proposals
             WHERE extension_id = ?1 AND revision = ?2 AND status = 'approved'
             ORDER BY updated_at DESC LIMIT 1",
            params![extension_id, row.current_revision],
            |query_row| query_row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .map(|value| parse_capabilities(&value))
        .transpose()?
        .unwrap_or_default();
    Ok(ExtensionCatalogEntry {
        extension_id: row.extension_id,
        catalog_scope: row.catalog_scope,
        source_kind: row.source_kind,
        lifecycle: row.lifecycle,
        review_status,
        manifest,
        current_revision: row.current_revision,
        active_revision: row.active_revision,
        approved_capabilities,
        compatible,
        untrusted: row.untrusted,
        updated_at: query_updated_at(transaction, extension_id)?,
    })
}

fn query_updated_at(
    transaction: &Transaction<'_>,
    extension_id: &str,
) -> Result<i64, DatabaseError> {
    Ok(transaction.query_row(
        "SELECT updated_at FROM extension_catalog WHERE extension_id = ?1",
        params![extension_id],
        |row| row.get(0),
    )?)
}

fn load_catalog_row_tx(
    transaction: &Transaction<'_>,
    extension_id: &str,
) -> Result<CatalogRow, DatabaseError> {
    transaction
        .query_row(
            "SELECT extension_id, catalog_scope, source_kind, lifecycle, current_revision,
                    active_revision, untrusted
             FROM extension_catalog WHERE extension_id = ?1 AND owner_user_id = ?2",
            params![extension_id, OWNER_ID],
            |row| {
                Ok(CatalogRow {
                    extension_id: row.get(0)?,
                    catalog_scope: catalog_scope_from_str(&row.get::<_, String>(1)?)?,
                    source_kind: source_kind_from_str(&row.get::<_, String>(2)?)?,
                    lifecycle: lifecycle_from_str(&row.get::<_, String>(3)?)?,
                    current_revision: row.get(4)?,
                    active_revision: row.get(5)?,
                    untrusted: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("extension_not_found"))
}

fn load_manifest_tx(
    transaction: &Transaction<'_>,
    extension_id: &str,
    revision: i64,
) -> Result<(ExtensionManifest, bool, ExtensionReviewStatus), DatabaseError> {
    transaction
        .query_row(
            "SELECT manifest_json, review_status
             FROM extension_manifest_revisions
             WHERE extension_id = ?1 AND revision = ?2",
            params![extension_id, revision],
            |row| {
                let manifest_json = row.get::<_, String>(0)?;
                let manifest = serde_json::from_str::<ExtensionManifest>(&manifest_json)
                    .map_err(|_| invalid_query())?;
                let compatible = manifest_is_compatible(&manifest);
                Ok((
                    manifest,
                    compatible,
                    review_status_from_str(&row.get::<_, String>(1)?)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("extension_revision_not_found"))
}

fn load_proposal_connection(
    connection: &Connection,
    proposal_id: &str,
) -> Result<ExtensionProposal, DatabaseError> {
    let transaction = connection.unchecked_transaction()?;
    let proposal = load_proposal_tx(&transaction, proposal_id)?;
    transaction.rollback()?;
    Ok(proposal)
}

fn load_proposal_tx(
    transaction: &Transaction<'_>,
    proposal_id: &str,
) -> Result<ExtensionProposal, DatabaseError> {
    let (
        id,
        extension_id,
        revision,
        source_kind,
        proposer_agent_id,
        status,
        approved_json,
        reason,
        created_at,
        updated_at,
    ) = transaction
        .query_row(
            "SELECT id, extension_id, revision, source_kind, proposer_agent_id, status,
                    approved_capabilities_json, review_reason, created_at, updated_at
             FROM extension_proposals WHERE id = ?1 AND owner_user_id = ?2",
            params![proposal_id, OWNER_ID],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    source_kind_from_str(&row.get::<_, String>(3)?)?,
                    row.get::<_, Option<String>>(4)?,
                    proposal_status_from_str(&row.get::<_, String>(5)?)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("extension_proposal_not_found"))?;
    let (manifest, compatible, review_status) =
        load_manifest_tx(transaction, &extension_id, revision)?;
    let mut statement = transaction.prepare(
        "SELECT capability, status FROM extension_permission_requests
         WHERE proposal_id = ?1 ORDER BY capability",
    )?;
    let permissions = statement
        .query_map(params![proposal_id], |row| {
            Ok(ExtensionPermissionRequest {
                capability: capability_from_str(&row.get::<_, String>(0)?)?,
                status: permission_status_from_str(&row.get::<_, String>(1)?)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let approved_capabilities = approved_json
        .map(|value| parse_capabilities(&value))
        .transpose()?
        .unwrap_or_default();
    Ok(ExtensionProposal {
        id,
        extension_id,
        revision,
        source_kind,
        proposer_agent_id,
        status,
        review_status,
        requested_capabilities: manifest.capabilities.clone(),
        manifest,
        approved_capabilities,
        permissions,
        compatible,
        review_reason: reason,
        created_at,
        updated_at,
    })
}

fn map_audit_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExtensionAuditRecord> {
    let details = serde_json::from_str::<AuditDetailsOwned>(&row.get::<_, String>(8)?)
        .map_err(|_| invalid_query())?;
    Ok(ExtensionAuditRecord {
        id: row.get(0)?,
        extension_id: row.get(1)?,
        proposal_id: row.get(2)?,
        revision: row.get(3)?,
        agent_id: row.get(4)?,
        event: row.get(5)?,
        result: row.get(6)?,
        code: row.get(7)?,
        summary: details.summary,
        created_at: row.get(9)?,
    })
}

#[derive(Debug, Deserialize)]
struct AuditDetailsOwned {
    summary: String,
}

fn audit_tx(transaction: &Transaction<'_>, context: AuditContext<'_>) -> Result<(), DatabaseError> {
    let details_json = serde_json::to_string(&AuditDetails {
        summary: context.summary,
    })
    .map_err(|_| DatabaseError::Unavailable)?;
    if details_json.len() > MAX_EXTENSION_AUDIT_BYTES {
        return Err(DatabaseError::Cognitive("extension_audit_oversized"));
    }
    transaction.execute(
        "DELETE FROM extension_audit_log WHERE created_at < ?1",
        params![now_millis() - EXTENSION_AUDIT_RETENTION_MS],
    )?;
    transaction.execute(
        "INSERT INTO extension_audit_log
         (id, extension_id, proposal_id, revision, agent_id, owner_user_id, event, result, code,
          details_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            Uuid::now_v7().to_string(),
            context.extension_id,
            context.proposal_id,
            context.revision,
            context.agent_id,
            context.owner_id,
            context.event,
            context.result,
            context.code,
            details_json,
            now_millis(),
        ],
    )?;
    Ok(())
}

fn request_fingerprint<T: Serialize>(
    operation: &str,
    request: &T,
) -> Result<String, DatabaseError> {
    let request_json =
        serde_json::to_string(&(operation, request)).map_err(|_| DatabaseError::Unavailable)?;
    if request_json.len() > MAX_EXTENSION_IDEMPOTENCY_REQUEST_BYTES {
        return Err(DatabaseError::Cognitive("extension_request_oversized"));
    }
    Ok(request_json)
}

fn replay_extension_tx(
    transaction: &Transaction<'_>,
    owner_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
) -> Result<Option<StoredExtensionResult>, DatabaseError> {
    let stored = transaction
        .query_row(
            "SELECT request_json, result_kind, result_json
             FROM extension_idempotency
             WHERE owner_user_id = ?1 AND operation = ?2 AND idempotency_key = ?3",
            params![owner_id, operation, idempotency_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((stored_request_json, result_kind, result_json)) = stored else {
        return Ok(None);
    };
    if stored_request_json != request_json {
        return Err(DatabaseError::Cognitive("idempotency_conflict"));
    }
    let result = match result_kind.as_str() {
        "proposal" => serde_json::from_str::<ExtensionProposal>(&result_json)
            .map(StoredExtensionResult::Proposal),
        "catalog" => serde_json::from_str::<ExtensionCatalogEntry>(&result_json)
            .map(StoredExtensionResult::Catalog),
        _ => return Err(DatabaseError::Cognitive("extension_idempotency_invalid")),
    }
    .map_err(|_| DatabaseError::Cognitive("extension_idempotency_invalid"))?;
    Ok(Some(result))
}

fn replay_proposal_tx(
    transaction: &Transaction<'_>,
    owner_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
) -> Result<Option<ExtensionProposal>, DatabaseError> {
    match replay_extension_tx(
        transaction,
        owner_id,
        operation,
        idempotency_key,
        request_json,
    )? {
        None => Ok(None),
        Some(StoredExtensionResult::Proposal(proposal)) => Ok(Some(proposal)),
        Some(StoredExtensionResult::Catalog(_)) => {
            Err(DatabaseError::Cognitive("idempotency_conflict"))
        }
    }
}

fn replay_catalog_tx(
    transaction: &Transaction<'_>,
    owner_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
) -> Result<Option<ExtensionCatalogEntry>, DatabaseError> {
    match replay_extension_tx(
        transaction,
        owner_id,
        operation,
        idempotency_key,
        request_json,
    )? {
        None => Ok(None),
        Some(StoredExtensionResult::Catalog(entry)) => Ok(Some(entry)),
        Some(StoredExtensionResult::Proposal(_)) => {
            Err(DatabaseError::Cognitive("idempotency_conflict"))
        }
    }
}

fn record_idempotency_tx(
    transaction: &Transaction<'_>,
    owner_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    result: &StoredExtensionResult,
    now: i64,
) -> Result<(), DatabaseError> {
    let (result_kind, result_json) = match result {
        StoredExtensionResult::Proposal(proposal) => (
            "proposal",
            serde_json::to_string(proposal).map_err(|_| DatabaseError::Unavailable)?,
        ),
        StoredExtensionResult::Catalog(entry) => (
            "catalog",
            serde_json::to_string(entry).map_err(|_| DatabaseError::Unavailable)?,
        ),
    };
    if result_json.len() > MAX_EXTENSION_IDEMPOTENCY_RESULT_BYTES {
        return Err(DatabaseError::Cognitive("extension_result_oversized"));
    }
    transaction.execute(
        "INSERT INTO extension_idempotency
         (owner_user_id, operation, idempotency_key, request_json, result_kind, result_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            owner_id,
            operation,
            idempotency_key,
            request_json,
            result_kind,
            result_json,
            now,
        ],
    )?;
    Ok(())
}

fn ensure_owner_agent(connection: &Connection, agent_id: &str) -> Result<String, DatabaseError> {
    connection
        .query_row(
            "SELECT owner_user_id FROM agents WHERE id = ?1",
            params![agent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("agent_not_found"))
        .and_then(|owner| {
            if owner == OWNER_ID {
                Ok(owner)
            } else {
                Err(DatabaseError::OwnershipMismatch)
            }
        })
}

fn ensure_owner_agent_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_id: &str,
) -> Result<(), DatabaseError> {
    transaction
        .query_row(
            "SELECT owner_user_id FROM agents WHERE id = ?1",
            params![agent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("agent_not_found"))
        .and_then(|owner| {
            if owner == owner_id {
                Ok(())
            } else {
                Err(DatabaseError::OwnershipMismatch)
            }
        })
}

fn ensure_owner_actor_tx(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
) -> Result<String, DatabaseError> {
    if owner_user_id != OWNER_ID {
        return Err(DatabaseError::Cognitive("extension_owner_required"));
    }
    transaction
        .query_row(
            "SELECT id FROM users WHERE id = ?1 AND role = 'owner'",
            params![owner_user_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("extension_owner_required"))
}

fn ensure_not_temporary(temporary_chat: bool) -> Result<(), DatabaseError> {
    if temporary_chat {
        Err(DatabaseError::Cognitive("extensions_blocked_temporary"))
    } else {
        Ok(())
    }
}

fn ensure_extensions_enabled_tx(transaction: &Transaction<'_>) -> Result<(), DatabaseError> {
    let safe_mode = transaction
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = 'safe_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some_and(|value| value == "true");
    if safe_mode {
        Err(DatabaseError::Cognitive("extensions_blocked_safe_mode"))
    } else {
        Ok(())
    }
}

fn validate_source(
    transaction: &Transaction<'_>,
    source_kind: &ExtensionSourceKind,
    proposer_agent_id: Option<&str>,
    requesting_agent_id: &str,
    owner_id: &str,
) -> Result<Option<String>, DatabaseError> {
    match source_kind {
        ExtensionSourceKind::AdministratorSelected => {
            if proposer_agent_id.is_some() {
                Err(DatabaseError::Cognitive("extension_source_invalid"))
            } else {
                Ok(None)
            }
        }
        ExtensionSourceKind::AgentCreated => {
            let proposer_agent_id =
                proposer_agent_id.ok_or(DatabaseError::Cognitive("extension_source_invalid"))?;
            if proposer_agent_id != requesting_agent_id {
                return Err(DatabaseError::Cognitive("extension_source_invalid"));
            }
            ensure_owner_agent_tx(transaction, proposer_agent_id, owner_id)?;
            Ok(Some(proposer_agent_id.to_string()))
        }
    }
}

fn normalize_manifest(mut manifest: ExtensionManifest) -> Result<ExtensionManifest, DatabaseError> {
    validate_extension_id(&manifest.extension_id)?;
    if manifest.manifest_version != EXTENSION_MANIFEST_VERSION {
        return Err(DatabaseError::Cognitive("extension_manifest_invalid"));
    }
    if manifest.sdk_version != AIP_EXTENSION_SDK_VERSION {
        return Err(DatabaseError::Cognitive("extension_sdk_incompatible"));
    }
    validate_extension_version(&manifest.extension_version)?;
    manifest.name = validate_text(&manifest.name, 160)?;
    if manifest.sandbox_policy != ExtensionSandboxPolicy::MetadataOnly {
        return Err(DatabaseError::Cognitive("extension_sandbox_invalid"));
    }
    if manifest.admission_policy != ExtensionAdmissionPolicy::LocalFixtureOnly {
        return Err(DatabaseError::Cognitive("extension_admission_denied"));
    }
    if !manifest.untrusted {
        return Err(DatabaseError::Cognitive("extension_untrusted_required"));
    }
    manifest.capabilities = normalize_capabilities(manifest.capabilities)?;
    if let Some(local_fixture_ref) = manifest.local_fixture_ref.as_mut() {
        *local_fixture_ref = validate_fixture_ref(local_fixture_ref)?;
    }
    // Metadata-only manifests remain valid; executable admission requires an explicitly
    // untrusted, integrity-checked closed package.
    if let Some(package) = manifest.package.as_ref() {
        validate_package(package)?;
    }
    let manifest_json = serde_json::to_string(&manifest).map_err(|_| DatabaseError::Unavailable)?;
    if manifest_json.len() > MAX_EXTENSION_MANIFEST_BYTES {
        return Err(DatabaseError::Cognitive("extension_manifest_oversized"));
    }
    Ok(manifest)
}

fn manifest_is_compatible(manifest: &ExtensionManifest) -> bool {
    manifest.manifest_version == EXTENSION_MANIFEST_VERSION
        && manifest.sdk_version == AIP_EXTENSION_SDK_VERSION
        && manifest.sandbox_policy == ExtensionSandboxPolicy::MetadataOnly
        && manifest.admission_policy == ExtensionAdmissionPolicy::LocalFixtureOnly
        && manifest.untrusted
        && validate_extension_id(&manifest.extension_id).is_ok()
        && validate_extension_version(&manifest.extension_version).is_ok()
        && validate_text(&manifest.name, 160).is_ok()
        && normalize_capabilities(manifest.capabilities.clone()).is_ok()
        && manifest
            .local_fixture_ref
            .as_deref()
            .is_none_or(|value| validate_fixture_ref(value).is_ok())
        && manifest
            .package
            .as_ref()
            .is_none_or(|package| validate_package(package).is_ok())
}

fn validate_extension_id(value: &str) -> Result<(), DatabaseError> {
    if value.is_empty()
        || value.len() > MAX_EXTENSION_ID_BYTES
        || value.chars().any(|character| {
            !(character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || ".-_".contains(character))
        })
        || value.starts_with('.')
        || value.ends_with('.')
    {
        Err(DatabaseError::Cognitive("extension_id_invalid"))
    } else {
        Ok(())
    }
}

fn validate_extension_version(value: &str) -> Result<(), DatabaseError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || part.len() > 6
                || part.parse::<u32>().is_err()
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        Err(DatabaseError::Cognitive("extension_version_invalid"))
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
            .any(|character| character == '\0' || character.is_control())
    {
        Err(DatabaseError::Cognitive("extension_text_invalid"))
    } else {
        Ok(value.to_string())
    }
}

fn validate_fixture_ref(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    let prefix = "fixture:extension/";
    if value.len() <= prefix.len()
        || value.len() > 160
        || !value.starts_with(prefix)
        || value.contains("..")
        || value.contains('\\')
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._/-".contains(character)))
    {
        Err(DatabaseError::Cognitive("extension_fixture_invalid"))
    } else {
        Ok(value.to_string())
    }
}

fn normalize_capabilities(
    mut capabilities: Vec<ExtensionCapability>,
) -> Result<Vec<ExtensionCapability>, DatabaseError> {
    if capabilities.len() > MAX_EXTENSION_CAPABILITIES {
        return Err(DatabaseError::Cognitive("extension_capability_invalid"));
    }
    capabilities.sort_by(|left, right| capability_kind(left).cmp(capability_kind(right)));
    if capabilities.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(DatabaseError::Cognitive("extension_capability_invalid"));
    }
    Ok(capabilities)
}

fn serialize_capabilities(capabilities: &[ExtensionCapability]) -> Result<String, DatabaseError> {
    serde_json::to_string(capabilities).map_err(|_| DatabaseError::Unavailable)
}

fn parse_capabilities(value: &str) -> Result<Vec<ExtensionCapability>, DatabaseError> {
    let capabilities = serde_json::from_str::<Vec<ExtensionCapability>>(value)
        .map_err(|_| DatabaseError::Cognitive("extension_capability_invalid"))?;
    normalize_capabilities(capabilities)
}

fn valid_idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        Err(DatabaseError::Cognitive("invalid_idempotency_key"))
    } else {
        Ok(value.to_string())
    }
}

fn invalid_query() -> rusqlite::Error {
    rusqlite::Error::InvalidQuery
}

fn capability_kind(value: &ExtensionCapability) -> &'static str {
    match value {
        ExtensionCapability::AgentContext => "agent_context",
        ExtensionCapability::ToolCatalog => "tool_catalog",
        ExtensionCapability::OwnerReview => "owner_review",
    }
}

fn capability_from_str(value: &str) -> rusqlite::Result<ExtensionCapability> {
    match value {
        "agent_context" => Ok(ExtensionCapability::AgentContext),
        "tool_catalog" => Ok(ExtensionCapability::ToolCatalog),
        "owner_review" => Ok(ExtensionCapability::OwnerReview),
        _ => Err(invalid_query()),
    }
}

fn sandbox_policy_kind(value: &ExtensionSandboxPolicy) -> &'static str {
    match value {
        ExtensionSandboxPolicy::MetadataOnly => "metadata_only",
    }
}

fn admission_policy_kind(value: &ExtensionAdmissionPolicy) -> &'static str {
    match value {
        ExtensionAdmissionPolicy::LocalFixtureOnly => "local_fixture_only",
    }
}

fn source_kind_kind(value: &ExtensionSourceKind) -> &'static str {
    match value {
        ExtensionSourceKind::AdministratorSelected => "administrator_selected",
        ExtensionSourceKind::AgentCreated => "agent_created",
    }
}

fn source_kind_from_str(value: &str) -> rusqlite::Result<ExtensionSourceKind> {
    match value {
        "administrator_selected" => Ok(ExtensionSourceKind::AdministratorSelected),
        "agent_created" => Ok(ExtensionSourceKind::AgentCreated),
        _ => Err(invalid_query()),
    }
}

fn catalog_scope_from_str(value: &str) -> rusqlite::Result<ExtensionCatalogScope> {
    match value {
        "private_local" => Ok(ExtensionCatalogScope::PrivateLocal),
        _ => Err(invalid_query()),
    }
}

fn lifecycle_kind(value: &ExtensionLifecycle) -> &'static str {
    match value {
        ExtensionLifecycle::ReviewRequired => "review_required",
        ExtensionLifecycle::Approved => "approved",
        ExtensionLifecycle::Active => "active",
        ExtensionLifecycle::Disabled => "disabled",
        ExtensionLifecycle::Rejected => "rejected",
        ExtensionLifecycle::RecoveryRequired => "recovery_required",
    }
}

fn lifecycle_from_str(value: &str) -> rusqlite::Result<ExtensionLifecycle> {
    match value {
        "review_required" => Ok(ExtensionLifecycle::ReviewRequired),
        "approved" => Ok(ExtensionLifecycle::Approved),
        "active" => Ok(ExtensionLifecycle::Active),
        "disabled" => Ok(ExtensionLifecycle::Disabled),
        "rejected" => Ok(ExtensionLifecycle::Rejected),
        "recovery_required" => Ok(ExtensionLifecycle::RecoveryRequired),
        _ => Err(invalid_query()),
    }
}

fn review_status_kind(value: &ExtensionReviewStatus) -> &'static str {
    match value {
        ExtensionReviewStatus::Pending => "pending",
        ExtensionReviewStatus::Approved => "approved",
        ExtensionReviewStatus::Rejected => "rejected",
    }
}

fn review_status_from_str(value: &str) -> rusqlite::Result<ExtensionReviewStatus> {
    match value {
        "pending" => Ok(ExtensionReviewStatus::Pending),
        "approved" => Ok(ExtensionReviewStatus::Approved),
        "rejected" => Ok(ExtensionReviewStatus::Rejected),
        _ => Err(invalid_query()),
    }
}

fn proposal_status_kind(value: &ExtensionProposalStatus) -> &'static str {
    match value {
        ExtensionProposalStatus::Pending => "pending",
        ExtensionProposalStatus::Approved => "approved",
        ExtensionProposalStatus::Rejected => "rejected",
        ExtensionProposalStatus::Withdrawn => "withdrawn",
    }
}

fn proposal_status_from_str(value: &str) -> rusqlite::Result<ExtensionProposalStatus> {
    match value {
        "pending" => Ok(ExtensionProposalStatus::Pending),
        "approved" => Ok(ExtensionProposalStatus::Approved),
        "rejected" => Ok(ExtensionProposalStatus::Rejected),
        "withdrawn" => Ok(ExtensionProposalStatus::Withdrawn),
        _ => Err(invalid_query()),
    }
}

fn permission_status_kind(value: &ExtensionPermissionStatus) -> &'static str {
    match value {
        ExtensionPermissionStatus::Pending => "pending",
        ExtensionPermissionStatus::Approved => "approved",
        ExtensionPermissionStatus::Denied => "denied",
    }
}

fn permission_status_from_str(value: &str) -> rusqlite::Result<ExtensionPermissionStatus> {
    match value {
        "pending" => Ok(ExtensionPermissionStatus::Pending),
        "approved" => Ok(ExtensionPermissionStatus::Approved),
        "denied" => Ok(ExtensionPermissionStatus::Denied),
        _ => Err(invalid_query()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use uuid::Uuid;

    use super::*;
    use crate::database::{ASTRA_ID, LUMA_ID};

    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("aip-extensions-test-{}", Uuid::now_v7()))
            .join("aip.sqlite3")
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path.parent().expect("test path should have a parent"));
    }

    fn manifest(version: &str, capabilities: Vec<ExtensionCapability>) -> ExtensionManifest {
        ExtensionManifest {
            extension_id: "fixture.notes".into(),
            manifest_version: 1,
            extension_version: version.into(),
            sdk_version: AIP_EXTENSION_SDK_VERSION.into(),
            name: "Notas locais fixture".into(),
            sandbox_policy: ExtensionSandboxPolicy::MetadataOnly,
            admission_policy: ExtensionAdmissionPolicy::LocalFixtureOnly,
            capabilities,
            local_fixture_ref: Some("fixture:extension/notes".into()),
            untrusted: true,
            package: None,
        }
    }

    fn administrator_proposal(database: &Database, version: &str) -> ExtensionProposal {
        database
            .create_extension_proposal(ExtensionProposalRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                source_kind: ExtensionSourceKind::AdministratorSelected,
                proposer_agent_id: None,
                manifest: manifest(version, vec![ExtensionCapability::ToolCatalog]),
                idempotency_key: format!("admin-proposal-{version}"),
                temporary_chat: false,
            })
            .expect("administrator proposal should be created")
    }

    fn approve(database: &Database, proposal: &ExtensionProposal) -> ExtensionProposal {
        database
            .review_extension_proposal(ExtensionReviewRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                proposal_id: proposal.id.clone(),
                approved: true,
                approved_capabilities: proposal.requested_capabilities.clone(),
                reason: Some("Revisado pelo Owner".into()),
                idempotency_key: format!("review-{}", proposal.id),
                temporary_chat: false,
            })
            .expect("proposal should be approved")
    }

    fn package(instructions: Vec<ExtensionInstruction>) -> ExtensionPackage {
        let mut package = ExtensionPackage {
            format: "aip-extension-package/v1".into(),
            entrypoint: "main".into(),
            instructions,
            integrity_sha256: String::new(),
        };
        package.integrity_sha256 = extension_package_hash(&package).expect("package hash");
        package
    }

    fn runtime_manifest() -> ExtensionManifest {
        let mut result = manifest(
            "1.0.0",
            vec![
                ExtensionCapability::AgentContext,
                ExtensionCapability::ToolCatalog,
            ],
        );
        result.package = Some(package(vec![
            ExtensionInstruction::ReadAgentContext,
            ExtensionInstruction::ListToolCatalog,
            ExtensionInstruction::EmitText {
                text: Some("|ok|".into()),
                echo_input: None,
            },
        ]));
        result
    }

    fn activate_runtime(database: &Database) -> (ExtensionProposal, String) {
        let proposal = database
            .create_extension_proposal(ExtensionProposalRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                source_kind: ExtensionSourceKind::AdministratorSelected,
                proposer_agent_id: None,
                manifest: runtime_manifest(),
                idempotency_key: "runtime-proposal".into(),
                temporary_chat: false,
            })
            .expect("runtime proposal");
        let approved = approve(database, &proposal);
        database
            .activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: proposal.extension_id.clone(),
                proposal_id: approved.id,
                idempotency_key: "runtime-activate".into(),
                temporary_chat: false,
            })
            .expect("runtime activation");
        let hash = runtime_manifest()
            .package
            .expect("package")
            .integrity_sha256;
        (proposal, hash)
    }

    #[test]
    fn executable_package_runs_with_bounded_host_context_and_replays() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database");
        let (proposal, hash) = activate_runtime(&database);
        let expected_tool = database
            .list_tool_catalog()
            .expect("tool catalog")
            .first()
            .expect("seeded tool")
            .tool_id
            .clone();
        let request = ExtensionExecutionRequest {
            agent_id: ASTRA_ID.into(),
            owner_user_id: OWNER_ID.into(),
            extension_id: proposal.extension_id,
            revision: 1,
            package_hash: hash,
            input: "private input must not enter host context".into(),
            idempotency_key: "runtime-execute".into(),
            temporary_chat: false,
        };
        let first = database
            .execute_extension(request.clone())
            .expect("execution");
        assert_eq!(first.status, "succeeded");
        assert!(first
            .output
            .as_deref()
            .is_some_and(|value| value.contains("agent_id:agt_astra_provisional")));
        assert!(first
            .output
            .as_deref()
            .is_some_and(|value| { value.contains(&format!("tool_ids:{expected_tool}")) }));
        assert!(!first
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("private input"));
        assert_eq!(database.execute_extension(request.clone()).unwrap(), first);
        assert_eq!(
            database.execute_extension(ExtensionExecutionRequest {
                input: "conflict".into(),
                ..request
            }),
            Err(DatabaseError::Cognitive("idempotency_conflict"))
        );
        cleanup(&path);
    }

    #[test]
    fn package_validation_and_execution_gates_fail_closed() {
        let host = ExtensionHostContext {
            agent_id: ASTRA_ID.into(),
            tool_ids: vec!["tool.one".into()],
        };
        let valid = package(vec![ExtensionInstruction::EmitText {
            text: Some("ok".into()),
            echo_input: None,
        }]);
        assert_eq!(
            interpret_extension_package(&valid, "", &[], &host).unwrap(),
            "ok"
        );
        let mut tampered = valid.clone();
        tampered.instructions.push(ExtensionInstruction::Yield);
        assert_eq!(
            interpret_extension_package(&tampered, "", &[], &host),
            Err(DatabaseError::Cognitive("extension_package_invalid"))
        );
        let duplicate = package(vec![
            ExtensionInstruction::Yield,
            ExtensionInstruction::Yield,
        ]);
        assert_eq!(
            interpret_extension_package(&duplicate, "", &[], &host),
            Err(DatabaseError::Cognitive("extension_instruction_duplicate"))
        );
        let oversized = package(vec![ExtensionInstruction::EmitText {
            text: Some("x".repeat(MAX_PACKAGE_TEXT_BYTES + 1)),
            echo_input: None,
        }]);
        assert_eq!(
            interpret_extension_package(&oversized, "", &[], &host),
            Err(DatabaseError::Cognitive("extension_instruction_oversized"))
        );
        let denied = package(vec![ExtensionInstruction::ReadAgentContext]);
        assert_eq!(
            interpret_extension_package(&denied, "", &[], &host),
            Err(DatabaseError::Cognitive("extension_capability_denied"))
        );
        assert!(serde_json::from_str::<ExtensionInstruction>(r#"{"op":"unknown"}"#).is_err());
    }

    #[test]
    fn metadata_only_and_execution_modes_are_denied() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database");
        let proposal = administrator_proposal(&database, "1.0.0");
        approve(&database, &proposal);
        database
            .activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: proposal.extension_id.clone(),
                proposal_id: proposal.id,
                idempotency_key: "metadata-active".into(),
                temporary_chat: false,
            })
            .expect("activate");
        assert_eq!(
            database.execute_extension(ExtensionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: "fixture.notes".into(),
                revision: 1,
                package_hash: "0".repeat(64),
                input: String::new(),
                idempotency_key: "metadata-execute".into(),
                temporary_chat: false
            }),
            Err(DatabaseError::Cognitive("extension_package_required"))
        );
        database.set_safe_mode(true).unwrap();
        assert_eq!(
            database.execute_extension(ExtensionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: "fixture.notes".into(),
                revision: 1,
                package_hash: "0".repeat(64),
                input: String::new(),
                idempotency_key: "safe-execute".into(),
                temporary_chat: false
            }),
            Err(DatabaseError::Cognitive("extensions_blocked_safe_mode"))
        );
        assert_eq!(
            database.execute_extension(ExtensionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: "fixture.notes".into(),
                revision: 1,
                package_hash: "0".repeat(64),
                input: String::new(),
                idempotency_key: "temporary-execute".into(),
                temporary_chat: true
            }),
            Err(DatabaseError::Cognitive("extensions_blocked_temporary"))
        );
        cleanup(&path);
    }

    #[test]
    fn agent_proposals_remain_pending_until_owner_approval_and_activation() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        let proposal = database
            .create_agent_extension_proposal(ExtensionAgentProposalRequest {
                agent_id: LUMA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                manifest: manifest("1.0.0", vec![ExtensionCapability::OwnerReview]),
                idempotency_key: "agent-proposal-1".into(),
                temporary_chat: false,
            })
            .expect("agent proposal should be created");
        assert_eq!(proposal.source_kind, ExtensionSourceKind::AgentCreated);
        assert_eq!(proposal.status, ExtensionProposalStatus::Pending);
        assert_eq!(
            database.activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: proposal.extension_id.clone(),
                proposal_id: proposal.id.clone(),
                idempotency_key: "activate-before-review".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("extension_review_required"))
        );
        let approved = approve(&database, &proposal);
        assert_eq!(approved.status, ExtensionProposalStatus::Approved);
        let active = database
            .activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: proposal.extension_id.clone(),
                proposal_id: proposal.id.clone(),
                idempotency_key: "activate-after-review".into(),
                temporary_chat: false,
            })
            .expect("approved extension should activate explicitly");
        assert_eq!(active.lifecycle, ExtensionLifecycle::Active);
        assert!(active.untrusted);
        assert!(database
            .list_extension_audit(ASTRA_ID)
            .unwrap()
            .iter()
            .any(|record| record.event == "extension_activated"));
        cleanup(&path);
    }

    #[test]
    fn updates_disable_until_review_and_rollback_restores_prior_approved_revision() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        let first = administrator_proposal(&database, "1.0.0");
        approve(&database, &first);
        database
            .activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: first.extension_id.clone(),
                proposal_id: first.id.clone(),
                idempotency_key: "activate-v1".into(),
                temporary_chat: false,
            })
            .unwrap();
        let update = database
            .update_extension(ExtensionUpdateRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: first.extension_id.clone(),
                source_kind: ExtensionSourceKind::AdministratorSelected,
                proposer_agent_id: None,
                manifest: manifest(
                    "2.0.0",
                    vec![
                        ExtensionCapability::ToolCatalog,
                        ExtensionCapability::AgentContext,
                    ],
                ),
                idempotency_key: "update-v2".into(),
                temporary_chat: false,
            })
            .unwrap();
        let catalog = database.list_extension_catalog(ASTRA_ID).unwrap();
        assert_eq!(catalog[0].lifecycle, ExtensionLifecycle::Disabled);
        assert_eq!(catalog[0].active_revision, None);
        assert_eq!(
            database.activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: first.extension_id.clone(),
                proposal_id: update.id.clone(),
                idempotency_key: "activate-v2-before-review".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("extension_review_required"))
        );
        approve(&database, &update);
        database
            .activate_extension(ExtensionActivationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: first.extension_id.clone(),
                proposal_id: update.id,
                idempotency_key: "activate-v2".into(),
                temporary_chat: false,
            })
            .unwrap();
        let rolled_back = database
            .rollback_extension(ExtensionRollbackRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                extension_id: first.extension_id.clone(),
                target_revision: 1,
                idempotency_key: "rollback-v1".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(rolled_back.lifecycle, ExtensionLifecycle::Active);
        assert_eq!(rolled_back.current_revision, 1);
        assert_eq!(rolled_back.manifest.extension_version, "1.0.0");
        cleanup(&path);
    }

    #[test]
    fn temporary_chat_safe_mode_and_invalid_manifest_fail_closed() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        assert_eq!(
            database.create_extension_proposal(ExtensionProposalRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                source_kind: ExtensionSourceKind::AdministratorSelected,
                proposer_agent_id: None,
                manifest: manifest("1.0.0", vec![]),
                idempotency_key: "temporary-extension".into(),
                temporary_chat: true,
            }),
            Err(DatabaseError::Cognitive("extensions_blocked_temporary"))
        );
        assert_eq!(
            database.create_extension_proposal(ExtensionProposalRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                source_kind: ExtensionSourceKind::AdministratorSelected,
                proposer_agent_id: None,
                manifest: ExtensionManifest {
                    sdk_version: "future-sdk".into(),
                    ..manifest("1.0.0", vec![])
                },
                idempotency_key: "future-extension".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("extension_sdk_incompatible"))
        );
        let proposal = administrator_proposal(&database, "1.0.0");
        database.set_safe_mode(true).unwrap();
        assert_eq!(
            database.review_extension_proposal(ExtensionReviewRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                proposal_id: proposal.id,
                approved: true,
                approved_capabilities: vec![ExtensionCapability::ToolCatalog],
                reason: None,
                idempotency_key: "safe-review".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("extensions_blocked_safe_mode"))
        );
        cleanup(&path);
    }
}
