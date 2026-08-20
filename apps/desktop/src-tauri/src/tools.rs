use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError, OWNER_ID};

const TOOL_MANIFEST_VERSION: i64 = 1;
const MAX_SESSION_PERMISSIONS: usize = 12;
const MAX_ACTIONS_PER_SESSION: i64 = 64;
const MAX_SCOPE_BYTES: usize = 96;
const MAX_REFERENCE_BYTES: usize = 160;
const MAX_TEXT_BYTES: usize = 2_048;
const MAX_INPUT_BYTES: usize = 8_192;
const MAX_OUTPUT_BYTES: usize = 4_096;
const MAX_PREVIEW_BYTES: usize = 8_192;
const MAX_AUDIT_BYTES: usize = 4_096;
const MAX_AUDIT_ROWS: i64 = 100;
const MAX_WORKSPACE_ROOTS: i64 = 64;
const MAX_TOOL_CATALOG_ROWS: i64 = 16;
const AUDIT_RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolClassification {
    ReadOnly,
    StateChanging,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolAdapterKind {
    #[serde(rename = "workspace_mock")]
    Workspace,
    #[serde(rename = "calendar_mock")]
    Calendar,
    #[serde(rename = "messaging_mock")]
    Messaging,
    #[serde(rename = "workspace_local")]
    WorkspaceLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    Preview,
    ExecuteReadOnly,
    ExecuteStateChanging,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSessionStatus {
    Active,
    Cancelled,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolActionStatus {
    Previewed,
    Approved,
    Confirmed,
    DryRun,
    Executed,
    Cancelled,
    Failed,
    Compensated,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    DryRun,
    Simulated,
    Cancelled,
    Compensated,
    Executed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRoot {
    pub id: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRootRequest {
    pub path: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRootIdRequest {
    pub root_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolManifest {
    pub tool_id: String,
    pub manifest_version: i64,
    pub name: String,
    pub classification: ToolClassification,
    pub adapter_kind: ToolAdapterKind,
    pub scope_kind: String,
    pub requires_second_confirmation: bool,
    pub capabilities: Vec<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSessionPermission {
    pub tool_id: String,
    pub permission: ToolPermission,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSessionRequest {
    pub agent_id: String,
    pub scope_ref: String,
    pub permissions: Vec<ToolSessionPermission>,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSession {
    pub id: String,
    pub agent_id: String,
    pub scope_ref: String,
    pub status: ToolSessionStatus,
    pub permissions: Vec<ToolSessionPermission>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolFileMove {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub source_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ToolActionInput {
    WorkspaceInspect {
        relative_paths: Vec<String>,
    },
    WorkspaceOrganize {
        moves: Vec<ToolFileMove>,
    },
    CalendarList {
        date: String,
    },
    CalendarCreate {
        title: String,
        date: String,
        start: String,
        end: String,
    },
    MessagingPreview {
        recipient: String,
        body: String,
    },
    MessagingSend {
        recipient: String,
        body: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolActionPreviewRequest {
    pub agent_id: String,
    pub session_id: String,
    pub tool_id: String,
    pub input: ToolActionInput,
    pub dry_run: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolActionDecisionRequest {
    pub agent_id: String,
    pub action_id: String,
    pub approved: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolActionConfirmationRequest {
    pub agent_id: String,
    pub action_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolActionExecutionRequest {
    pub agent_id: String,
    pub action_id: String,
    pub dry_run: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolActionCancellationRequest {
    pub agent_id: String,
    pub action_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSessionCancellationRequest {
    pub agent_id: String,
    pub session_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionResult {
    pub status: ToolResultStatus,
    pub output: String,
    pub changed: bool,
    pub untrusted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCompensation {
    pub kind: String,
    pub available: bool,
    pub description: String,
    #[serde(default)]
    pub moves: Option<Vec<ToolCompensationMove>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCompensationMove {
    pub from: String,
    pub to: String,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAction {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub tool_id: String,
    pub classification: ToolClassification,
    pub input: ToolActionInput,
    pub summary: String,
    pub affected_resources: Vec<String>,
    pub exact_effect: String,
    pub status: ToolActionStatus,
    pub dry_run: bool,
    pub requires_owner_approval: bool,
    pub requires_second_confirmation: bool,
    pub owner_approved: bool,
    pub second_confirmed: bool,
    pub result: Option<ToolExecutionResult>,
    pub compensation: Option<ToolCompensation>,
    pub code: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAuditRecord {
    pub id: String,
    pub action_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: String,
    pub tool_id: Option<String>,
    pub event: String,
    pub result: String,
    pub code: Option<String>,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
struct AuditDetails<'a> {
    summary: &'a str,
}

#[derive(Debug)]
struct ToolPreviewPlan {
    input: ToolActionInput,
    summary: String,
    affected_resources: Vec<String>,
    exact_effect: String,
}

impl Database {
    pub fn add_workspace_root(
        &self,
        request: WorkspaceRootRequest,
    ) -> Result<WorkspaceRoot, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let key = valid_idempotency(&request.idempotency_key)?;
        let path = validate_workspace_root(Path::new(&request.path))?;
        let mut connection = self.open()?;
        let owner_id = OWNER_ID.to_string();
        let transaction = connection.transaction()?;
        let existing: Option<(String, String)> = transaction
            .query_row(
                "SELECT id, path FROM workspace_roots WHERE owner_user_id = ?1 AND idempotency_key = ?2",
                params![owner_id, key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((id, existing_path)) = existing {
            if existing_path != path.to_string_lossy() {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            let root = load_workspace_root_tx(&transaction, &id)?;
            transaction.commit()?;
            return Ok(root);
        }
        let now = now_millis();
        let root_count: i64 = transaction.query_row(
            "SELECT count(*) FROM workspace_roots WHERE owner_user_id = ?1",
            params![owner_id],
            |row| row.get(0),
        )?;
        if root_count >= MAX_WORKSPACE_ROOTS {
            return Err(DatabaseError::Cognitive("workspace_root_limit"));
        }
        let root_id = format!("wrt_{}", Uuid::now_v7());
        transaction.execute(
            "INSERT INTO workspace_roots
             (id, owner_user_id, path, idempotency_key, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
            params![
                root_id,
                owner_id,
                path.to_string_lossy().to_string(),
                key,
                now
            ],
        )?;
        let root = load_workspace_root_tx(&transaction, &root_id)?;
        transaction.commit()?;
        Ok(root)
    }

    pub fn list_workspace_roots(&self) -> Result<Vec<WorkspaceRoot>, DatabaseError> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id, enabled, created_at, updated_at FROM workspace_roots
             WHERE owner_user_id = ?1 ORDER BY updated_at DESC, id LIMIT ?2",
        )?;
        let roots = statement
            .query_map(params![OWNER_ID, MAX_WORKSPACE_ROOTS], map_workspace_root)
            .map_err(DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from);
        roots
    }

    pub fn remove_workspace_root(
        &self,
        request: WorkspaceRootIdRequest,
    ) -> Result<WorkspaceRoot, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _ = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let root = load_workspace_root_tx(&transaction, &request.root_id)?;
        transaction.execute(
            "UPDATE workspace_roots SET enabled = 0, updated_at = ?1
             WHERE id = ?2 AND owner_user_id = ?3",
            params![now_millis(), request.root_id, OWNER_ID],
        )?;
        let updated = load_workspace_root_tx(&transaction, &root.id)?;
        transaction.commit()?;
        Ok(updated)
    }

    pub fn list_tool_catalog(&self) -> Result<Vec<ToolManifest>, DatabaseError> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT tool_id, manifest_version, name, classification, adapter_kind,
                    scope_kind, requires_second_confirmation, capabilities_json, updated_at
             FROM tool_catalog ORDER BY tool_id LIMIT ?1",
        )?;
        let manifests = statement
            .query_map(params![MAX_TOOL_CATALOG_ROWS], map_tool_manifest)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from);
        manifests
    }

    pub fn create_tool_session(
        &self,
        request: ToolSessionRequest,
    ) -> Result<ToolSession, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let scope_ref = validate_scope(&request.scope_ref)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_owner_tx(&transaction, &request.agent_id)?;
        ensure_tools_enabled_tx(&transaction)?;
        let permissions = normalize_permissions(&transaction, &scope_ref, &request.permissions)?;
        let request_json = serde_json::to_string(&json!({
            "scopeRef": scope_ref,
            "permissions": permissions,
        }))
        .map_err(|_| DatabaseError::Unavailable)?;
        if let Some((session_id, existing_json)) = transaction
            .query_row(
                "SELECT id, request_json FROM tool_sessions
                 WHERE owner_user_id = ?1 AND idempotency_key = ?2",
                params![owner_id, idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if existing_json != request_json {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            let session = load_tool_session_tx(&transaction, &session_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let now = now_millis();
        let session_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO tool_sessions
             (id, agent_id, owner_user_id, scope_ref, status, idempotency_key,
              request_json, temporary_chat, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, 0, ?7, ?7)",
            params![
                session_id,
                request.agent_id,
                owner_id,
                scope_ref,
                idempotency_key,
                request_json,
                now,
            ],
        )?;
        for permission in &permissions {
            transaction.execute(
                "INSERT INTO tool_session_permissions (session_id, tool_id, permission)
                 VALUES (?1, ?2, ?3)",
                params![
                    session_id,
                    permission.tool_id,
                    permission_kind(&permission.permission),
                ],
            )?;
        }
        audit_tx(
            &transaction,
            AuditContext {
                action_id: None,
                session_id: Some(&session_id),
                agent_id: &request.agent_id,
                owner_id: &owner_id,
                tool_id: None,
                event: "session_created",
                result: "accepted",
                code: None,
                summary: "Sessão local de ferramentas criada.",
            },
        )?;
        let session = load_tool_session_tx(&transaction, &session_id)?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn list_tool_sessions(&self, agent_id: &str) -> Result<Vec<ToolSession>, DatabaseError> {
        let connection = self.open()?;
        ensure_owner(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id FROM tool_sessions
             WHERE agent_id = ?1 ORDER BY updated_at DESC, id DESC LIMIT 32",
        )?;
        let ids = statement
            .query_map(params![agent_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids.into_iter()
            .map(|id| load_tool_session_connection(&connection, &id))
            .collect()
    }

    pub fn preview_tool_action(
        &self,
        request: ToolActionPreviewRequest,
    ) -> Result<ToolAction, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        ensure_tools_enabled_tx(&transaction)?;
        let session = load_tool_session_tx(&transaction, &request.session_id)?;
        ensure_session_agent(&session, &request.agent_id)?;
        ensure_session_active(&session)?;
        ensure_permission(
            &transaction,
            &session.id,
            &request.tool_id,
            &ToolPermission::Preview,
        )?;
        let manifest = load_tool_manifest_tx(&transaction, &request.tool_id)?;
        let plan = validate_action_input(
            &transaction,
            &manifest,
            &session.scope_ref,
            &request.input,
            true,
        )?;
        let input_json =
            serde_json::to_string(&plan.input).map_err(|_| DatabaseError::Unavailable)?;
        if input_json.len() > MAX_INPUT_BYTES {
            return Err(DatabaseError::Cognitive("tool_input_invalid"));
        }
        if let Some(action_id) = transaction
            .query_row(
                "SELECT id FROM tool_actions
                 WHERE session_id = ?1 AND idempotency_key = ?2",
                params![session.id, idempotency_key],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            let existing = load_tool_action_tx(&transaction, &action_id)?;
            if existing.tool_id != request.tool_id
                || existing.input != plan.input
                || existing.dry_run != request.dry_run
            {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            transaction.commit()?;
            return Ok(existing);
        }
        let affected_resources_json = serde_json::to_string(&plan.affected_resources)
            .map_err(|_| DatabaseError::Unavailable)?;
        if affected_resources_json.len() > MAX_PREVIEW_BYTES {
            return Err(DatabaseError::Cognitive("tool_input_invalid"));
        }
        let now = now_millis();
        let action_id = Uuid::now_v7().to_string();
        transaction.execute(
            "SELECT 1 FROM tool_actions WHERE session_id = ?1 LIMIT 1",
            params![session.id],
        )?;
        let action_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM tool_actions WHERE session_id = ?1",
            params![session.id],
            |row| row.get(0),
        )?;
        if action_count >= MAX_ACTIONS_PER_SESSION {
            return Err(DatabaseError::Cognitive("tool_session_limit"));
        }
        transaction.execute(
            "INSERT INTO tool_actions
             (id, session_id, agent_id, owner_user_id, tool_id, classification, input_json,
              summary, affected_resources_json, exact_effect, status, dry_run,
              requires_second_confirmation, owner_approved, second_confirmed,
              result_json, compensation_json, error_code, idempotency_key,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'previewed', ?11,
                     ?12, 0, 0, NULL, NULL, NULL, ?13, ?14, ?14)",
            params![
                action_id,
                session.id,
                session.agent_id,
                OWNER_ID,
                manifest.tool_id,
                classification_kind(&manifest.classification),
                input_json,
                plan.summary,
                affected_resources_json,
                plan.exact_effect,
                request.dry_run,
                manifest.requires_second_confirmation,
                idempotency_key,
                now,
            ],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                action_id: Some(&action_id),
                session_id: Some(&session.id),
                agent_id: &session.agent_id,
                owner_id: OWNER_ID,
                tool_id: Some(&manifest.tool_id),
                event: "action_previewed",
                result: "previewed",
                code: None,
                summary: &plan.summary,
            },
        )?;
        let action = load_tool_action_tx(&transaction, &action_id)?;
        transaction.commit()?;
        Ok(action)
    }

    pub fn decide_tool_action(
        &self,
        request: ToolActionDecisionRequest,
    ) -> Result<ToolAction, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        let action =
            load_tool_action_for_agent_tx(&transaction, &request.action_id, &request.agent_id)?;
        let session = load_tool_session_tx(&transaction, &action.session_id)?;
        ensure_session_active(&session)?;
        let manifest = load_tool_manifest_tx(&transaction, &action.tool_id)?;
        if manifest.classification == ToolClassification::ReadOnly {
            return Err(DatabaseError::Cognitive("tool_approval_not_required"));
        }
        if action.status != ToolActionStatus::Previewed
            && action.status != ToolActionStatus::Approved
            && action.status != ToolActionStatus::Confirmed
        {
            if action.owner_approved && request.approved {
                transaction.commit()?;
                return Ok(action);
            }
            return Err(DatabaseError::Cognitive("tool_action_not_approvable"));
        }
        if !request.approved {
            let now = now_millis();
            let result_json = serde_json::to_string(&ToolExecutionResult {
                status: ToolResultStatus::Cancelled,
                output: "Ação recusada pelo Proprietário; nenhum efeito externo foi aplicado."
                    .into(),
                changed: false,
                untrusted: true,
            })
            .map_err(|_| DatabaseError::Unavailable)?;
            transaction.execute(
                "UPDATE tool_actions
                 SET status = 'rejected', error_code = 'tool_action_rejected',
                     result_json = ?1, updated_at = ?2
                 WHERE id = ?3 AND agent_id = ?4",
                params![result_json, now, request.action_id, request.agent_id],
            )?;
            audit_tx(
                &transaction,
                AuditContext {
                    action_id: Some(&request.action_id),
                    session_id: Some(&action.session_id),
                    agent_id: &request.agent_id,
                    owner_id: OWNER_ID,
                    tool_id: Some(&action.tool_id),
                    event: "action_rejected",
                    result: "rejected",
                    code: Some("tool_action_rejected"),
                    summary: "Ação recusada pelo Proprietário.",
                },
            )?;
        } else {
            ensure_tools_enabled_tx(&transaction)?;
            if action.owner_approved {
                transaction.commit()?;
                return Ok(action);
            }
            let now = now_millis();
            transaction.execute(
                "UPDATE tool_actions
                 SET status = 'approved', owner_approved = 1, approved_at = ?1, updated_at = ?1
                 WHERE id = ?2 AND agent_id = ?3",
                params![now, request.action_id, request.agent_id],
            )?;
            audit_tx(
                &transaction,
                AuditContext {
                    action_id: Some(&request.action_id),
                    session_id: Some(&action.session_id),
                    agent_id: &request.agent_id,
                    owner_id: OWNER_ID,
                    tool_id: Some(&action.tool_id),
                    event: "action_approved",
                    result: "approved",
                    code: None,
                    summary: "Ação aprovada pelo Proprietário.",
                },
            )?;
        }
        let result = load_tool_action_tx(&transaction, &request.action_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn confirm_tool_action(
        &self,
        request: ToolActionConfirmationRequest,
    ) -> Result<ToolAction, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        ensure_tools_enabled_tx(&transaction)?;
        let action =
            load_tool_action_for_agent_tx(&transaction, &request.action_id, &request.agent_id)?;
        let session = load_tool_session_tx(&transaction, &action.session_id)?;
        ensure_session_active(&session)?;
        let manifest = load_tool_manifest_tx(&transaction, &action.tool_id)?;
        if !manifest.requires_second_confirmation {
            return Err(DatabaseError::Cognitive("tool_confirmation_not_required"));
        }
        if !action.owner_approved {
            return Err(DatabaseError::Cognitive("tool_approval_required"));
        }
        if action.second_confirmed {
            transaction.commit()?;
            return Ok(action);
        }
        if action.status != ToolActionStatus::Approved {
            return Err(DatabaseError::Cognitive("tool_action_not_confirmable"));
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE tool_actions
             SET status = 'confirmed', second_confirmed = 1, confirmed_at = ?1, updated_at = ?1
             WHERE id = ?2 AND agent_id = ?3",
            params![now, request.action_id, request.agent_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                action_id: Some(&request.action_id),
                session_id: Some(&action.session_id),
                agent_id: &request.agent_id,
                owner_id: OWNER_ID,
                tool_id: Some(&action.tool_id),
                event: "action_confirmed",
                result: "confirmed",
                code: None,
                summary: "Segunda confirmação do Proprietário registrada.",
            },
        )?;
        let result = load_tool_action_tx(&transaction, &request.action_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn execute_tool_action(
        &self,
        request: ToolActionExecutionRequest,
    ) -> Result<ToolAction, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        ensure_tools_enabled_tx(&transaction)?;
        let action =
            load_tool_action_for_agent_tx(&transaction, &request.action_id, &request.agent_id)?;
        if action.dry_run != request.dry_run {
            return Err(DatabaseError::Cognitive("tool_action_invalid"));
        }
        let session = load_tool_session_tx(&transaction, &action.session_id)?;
        ensure_session_active(&session)?;
        let manifest = load_tool_manifest_tx(&transaction, &action.tool_id)?;
        if action.status == ToolActionStatus::Executed
            || action.status == ToolActionStatus::DryRun
            || action.status == ToolActionStatus::Compensated
        {
            transaction.commit()?;
            return Ok(action);
        }
        if action.status == ToolActionStatus::Cancelled
            || action.status == ToolActionStatus::Rejected
        {
            return Err(DatabaseError::Cognitive("tool_action_not_executable"));
        }
        if manifest.classification == ToolClassification::ReadOnly {
            ensure_permission(
                &transaction,
                &session.id,
                &manifest.tool_id,
                &ToolPermission::ExecuteReadOnly,
            )?;
        } else {
            ensure_permission(
                &transaction,
                &session.id,
                &manifest.tool_id,
                &ToolPermission::ExecuteStateChanging,
            )?;
            if !action.owner_approved {
                return Err(DatabaseError::Cognitive("tool_approval_required"));
            }
            if manifest.requires_second_confirmation && !action.second_confirmed {
                return Err(DatabaseError::Cognitive("tool_confirmation_required"));
            }
        }
        let plan = validate_action_input(
            &transaction,
            &manifest,
            &session.scope_ref,
            &action.input,
            false,
        )?;
        let result = match execute_adapter(&transaction, &manifest, &plan, action.dry_run) {
            Ok(result) => result,
            Err(error) => {
                let failure = ToolExecutionResult {
                    status: ToolResultStatus::Failed,
                    output: "A operação local falhou; o estado parcial foi tratado sem sobrescrever destinos inesperados.".into(),
                    changed: error == DatabaseError::Cognitive("workspace_move_partial"),
                    untrusted: true,
                };
                let result_json =
                    serde_json::to_string(&failure).map_err(|_| DatabaseError::Unavailable)?;
                transaction.execute(
                    "UPDATE tool_actions SET status = 'failed', result_json = ?1,
                     error_code = ?2, updated_at = ?3 WHERE id = ?4 AND agent_id = ?5",
                    params![
                        result_json,
                        error.code(),
                        now_millis(),
                        request.action_id,
                        request.agent_id
                    ],
                )?;
                audit_tx(
                    &transaction,
                    AuditContext {
                        action_id: Some(&request.action_id),
                        session_id: Some(&action.session_id),
                        agent_id: &request.agent_id,
                        owner_id: OWNER_ID,
                        tool_id: Some(&action.tool_id),
                        event: "action_failed",
                        result: "failed",
                        code: Some(error.code()),
                        summary: "Operação local falhou e foi registrada.",
                    },
                )?;
                transaction.commit()?;
                return Err(error);
            }
        };
        let result_json = serde_json::to_string(&result).map_err(|_| DatabaseError::Unavailable)?;
        if result_json.len() > MAX_OUTPUT_BYTES {
            return Err(DatabaseError::Cognitive("tool_output_oversized"));
        }
        let compensation = if manifest.classification == ToolClassification::StateChanging
            && !action.dry_run
        {
            Some(ToolCompensation {
                kind: if manifest.adapter_kind == ToolAdapterKind::WorkspaceLocal {
                    "workspace_move"
                } else {
                    "mock_noop"
                }
                .into(),
                available: true,
                description: if manifest.adapter_kind == ToolAdapterKind::WorkspaceLocal {
                    "Reverter somente os movimentos ainda compatíveis com a prévia aprovada.".into()
                } else {
                    "O mock não alterou sistemas externos; nenhuma compensação externa é necessária.".into()
                },
                moves: if manifest.adapter_kind == ToolAdapterKind::WorkspaceLocal {
                    Some(local_compensation_moves(
                        &transaction,
                        &session.scope_ref,
                        &action.input,
                    )?)
                } else {
                    None
                },
            })
        } else {
            None
        };
        let compensation_json = compensation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| DatabaseError::Unavailable)?;
        let status = if action.dry_run {
            "dry_run"
        } else {
            "executed"
        };
        let now = now_millis();
        transaction.execute(
            "UPDATE tool_actions
             SET status = ?1, result_json = ?2, compensation_json = ?3,
                 error_code = NULL, executed_at = ?4, updated_at = ?4
             WHERE id = ?5 AND agent_id = ?6",
            params![
                status,
                result_json,
                compensation_json,
                now,
                request.action_id,
                request.agent_id
            ],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                action_id: Some(&request.action_id),
                session_id: Some(&action.session_id),
                agent_id: &request.agent_id,
                owner_id: OWNER_ID,
                tool_id: Some(&action.tool_id),
                event: if action.dry_run {
                    "action_dry_run"
                } else {
                    "action_executed"
                },
                result: status,
                code: None,
                summary: &plan.summary,
            },
        )?;
        let result = load_tool_action_tx(&transaction, &request.action_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn cancel_tool_action(
        &self,
        request: ToolActionCancellationRequest,
    ) -> Result<ToolAction, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        let action =
            load_tool_action_for_agent_tx(&transaction, &request.action_id, &request.agent_id)?;
        let session = load_tool_session_tx(&transaction, &action.session_id)?;
        ensure_session_active(&session)?;
        if action.status == ToolActionStatus::Cancelled
            || action.status == ToolActionStatus::Rejected
        {
            transaction.commit()?;
            return Ok(action);
        }
        if action.status == ToolActionStatus::Executed
            || action.status == ToolActionStatus::DryRun
            || action.status == ToolActionStatus::Compensated
        {
            return Err(DatabaseError::Cognitive("tool_action_already_completed"));
        }
        let result_json = serde_json::to_string(&ToolExecutionResult {
            status: ToolResultStatus::Cancelled,
            output: "Ação cancelada; nenhum efeito externo foi aplicado.".into(),
            changed: false,
            untrusted: true,
        })
        .map_err(|_| DatabaseError::Unavailable)?;
        let now = now_millis();
        transaction.execute(
            "UPDATE tool_actions
             SET status = 'cancelled', error_code = 'tool_action_cancelled',
                 result_json = ?1, updated_at = ?2
             WHERE id = ?3 AND agent_id = ?4",
            params![result_json, now, request.action_id, request.agent_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                action_id: Some(&request.action_id),
                session_id: Some(&action.session_id),
                agent_id: &request.agent_id,
                owner_id: OWNER_ID,
                tool_id: Some(&action.tool_id),
                event: "action_cancelled",
                result: "cancelled",
                code: Some("tool_action_cancelled"),
                summary: "Ação cancelada pelo Proprietário.",
            },
        )?;
        let result = load_tool_action_tx(&transaction, &request.action_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn compensate_tool_action(
        &self,
        request: ToolActionCancellationRequest,
    ) -> Result<ToolAction, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        let action =
            load_tool_action_for_agent_tx(&transaction, &request.action_id, &request.agent_id)?;
        if action.status == ToolActionStatus::Compensated {
            transaction.commit()?;
            return Ok(action);
        }
        if action.status != ToolActionStatus::Executed || action.compensation.is_none() {
            return Err(DatabaseError::Cognitive("tool_compensation_unavailable"));
        }
        let session = load_tool_session_tx(&transaction, &action.session_id)?;
        let manifest = load_tool_manifest_tx(&transaction, &action.tool_id)?;
        if manifest.adapter_kind == ToolAdapterKind::WorkspaceLocal {
            let ToolActionInput::WorkspaceOrganize { moves } = &action.input else {
                return Err(DatabaseError::Cognitive("tool_compensation_unavailable"));
            };
            let root = workspace_root_path(&transaction, &session.scope_ref)?;
            let recorded = action
                .compensation
                .as_ref()
                .and_then(|compensation| compensation.moves.as_ref())
                .ok_or(DatabaseError::Cognitive(
                    "workspace_compensation_unavailable",
                ))?;
            if recorded.len() != moves.len() {
                return Err(DatabaseError::Cognitive(
                    "workspace_compensation_unavailable",
                ));
            }
            let mut reverse = Vec::with_capacity(moves.len());
            for (movement, identity) in moves.iter().zip(recorded) {
                let destination = safe_child(&root, &movement.to, true)?;
                let source = safe_child(&root, &movement.from, false)?;
                let current_identity = capture_file_identity(&destination)?;
                if current_identity != identity.identity {
                    record_compensation_failure(
                        &transaction,
                        &action,
                        &request,
                        "workspace_compensation_unavailable",
                    )?;
                    let result = load_tool_action_tx(&transaction, &request.action_id)?;
                    transaction.commit()?;
                    return Ok(result);
                }
                reverse.push((destination, source));
            }
            for (destination, source) in reverse {
                if fs::rename(destination, source).is_err() {
                    record_compensation_failure(
                        &transaction,
                        &action,
                        &request,
                        "workspace_compensation_failed",
                    )?;
                    let result = load_tool_action_tx(&transaction, &request.action_id)?;
                    transaction.commit()?;
                    return Ok(result);
                }
            }
            let result_json = serde_json::to_string(&ToolExecutionResult {
                status: ToolResultStatus::Compensated,
                output: "Movimentos locais revertidos com segurança.".into(),
                changed: true,
                untrusted: true,
            })
            .map_err(|_| DatabaseError::Unavailable)?;
            let now = now_millis();
            transaction.execute(
                "UPDATE tool_actions SET status = 'compensated', result_json = ?1,
                 updated_at = ?2 WHERE id = ?3 AND agent_id = ?4",
                params![result_json, now, request.action_id, request.agent_id],
            )?;
            audit_tx(
                &transaction,
                AuditContext {
                    action_id: Some(&request.action_id),
                    session_id: Some(&action.session_id),
                    agent_id: &request.agent_id,
                    owner_id: OWNER_ID,
                    tool_id: Some(&action.tool_id),
                    event: "action_compensated",
                    result: "compensated",
                    code: None,
                    summary: "Movimentos locais compensados.",
                },
            )?;
            let result = load_tool_action_tx(&transaction, &request.action_id)?;
            transaction.commit()?;
            return Ok(result);
        }
        let result_json = serde_json::to_string(&ToolExecutionResult {
            status: ToolResultStatus::Compensated,
            output: "Compensação registrada; o mock não teve efeito externo para desfazer.".into(),
            changed: false,
            untrusted: true,
        })
        .map_err(|_| DatabaseError::Unavailable)?;
        let now = now_millis();
        transaction.execute(
            "UPDATE tool_actions
             SET status = 'compensated', result_json = ?1, updated_at = ?2
             WHERE id = ?3 AND agent_id = ?4",
            params![result_json, now, request.action_id, request.agent_id],
        )?;
        audit_tx(
            &transaction,
            AuditContext {
                action_id: Some(&request.action_id),
                session_id: Some(&action.session_id),
                agent_id: &request.agent_id,
                owner_id: OWNER_ID,
                tool_id: Some(&action.tool_id),
                event: "action_compensated",
                result: "compensated",
                code: None,
                summary: "Compensação local registrada.",
            },
        )?;
        let result = load_tool_action_tx(&transaction, &request.action_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn cancel_tool_session(
        &self,
        request: ToolSessionCancellationRequest,
    ) -> Result<ToolSession, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let _idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_owner_tx(&transaction, &request.agent_id)?;
        let session = load_tool_session_tx(&transaction, &request.session_id)?;
        ensure_session_agent(&session, &request.agent_id)?;
        if session.status == ToolSessionStatus::Active {
            let now = now_millis();
            transaction.execute(
                "UPDATE tool_sessions SET status = 'cancelled', updated_at = ?1
                 WHERE id = ?2 AND agent_id = ?3",
                params![now, request.session_id, request.agent_id],
            )?;
            transaction.execute(
                "UPDATE tool_actions
                 SET status = 'cancelled', error_code = 'tool_session_cancelled', updated_at = ?1
                 WHERE session_id = ?2
                   AND status IN ('previewed', 'approved', 'confirmed')",
                params![now, request.session_id],
            )?;
            audit_tx(
                &transaction,
                AuditContext {
                    action_id: None,
                    session_id: Some(&request.session_id),
                    agent_id: &request.agent_id,
                    owner_id: OWNER_ID,
                    tool_id: None,
                    event: "session_cancelled",
                    result: "cancelled",
                    code: Some("tool_session_cancelled"),
                    summary: "Sessão de ferramentas cancelada pelo Proprietário.",
                },
            )?;
        }
        let result = load_tool_session_tx(&transaction, &request.session_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn list_tool_audit(&self, agent_id: &str) -> Result<Vec<ToolAuditRecord>, DatabaseError> {
        let connection = self.open()?;
        ensure_owner(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, action_id, session_id, agent_id, tool_id, event, result, code,
                    details_json, created_at
             FROM tool_audit_log
             WHERE agent_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let records = statement
            .query_map(params![agent_id, MAX_AUDIT_ROWS], map_tool_audit_record)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from);
        records
    }
}

fn map_workspace_root(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRoot> {
    Ok(WorkspaceRoot {
        id: row.get(0)?,
        enabled: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn load_workspace_root_tx(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<WorkspaceRoot, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, enabled, created_at, updated_at FROM workspace_roots
             WHERE id = ?1 AND owner_user_id = ?2",
            params![root_id, OWNER_ID],
            map_workspace_root,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("workspace_root_not_found"))
}

fn workspace_root_path(
    transaction: &Transaction<'_>,
    scope_ref: &str,
) -> Result<PathBuf, DatabaseError> {
    let root_id = scope_ref
        .strip_prefix("workspace_root:")
        .ok_or(DatabaseError::Cognitive("tool_scope_invalid"))?;
    let path: String = transaction
        .query_row(
            "SELECT path FROM workspace_roots
             WHERE id = ?1 AND owner_user_id = ?2 AND enabled = 1",
            params![root_id, OWNER_ID],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("workspace_root_unavailable"))?;
    validate_workspace_root(Path::new(&path))
}

fn validate_workspace_root(path: &Path) -> Result<PathBuf, DatabaseError> {
    let original = fs::symlink_metadata(path)
        .map_err(|_| DatabaseError::Cognitive("workspace_root_unavailable"))?;
    if original.file_type().is_symlink() {
        return Err(DatabaseError::Cognitive("workspace_root_invalid"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if original.file_attributes() & 0x400 != 0 {
            return Err(DatabaseError::Cognitive("workspace_root_invalid"));
        }
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| DatabaseError::Cognitive("workspace_root_unavailable"))?;
    if is_broad_workspace_root(&canonical) {
        return Err(DatabaseError::Cognitive("workspace_root_invalid"));
    }
    validate_existing_components(&canonical)?;
    let metadata = fs::symlink_metadata(&canonical)
        .map_err(|_| DatabaseError::Cognitive("workspace_root_unavailable"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || canonical.parent().is_none() {
        return Err(DatabaseError::Cognitive("workspace_root_invalid"));
    }
    Ok(canonical)
}

fn is_broad_workspace_root(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if normalized == "/" || normalized.ends_with(":/") {
        return true;
    }
    [
        "/home",
        "/tmp",
        "/var",
        "/etc",
        "/usr",
        "/opt",
        "/users",
        "/windows",
        "/windows/system32",
        "/program files",
        "/program files (x86)",
        "/programdata",
    ]
    .iter()
    .any(|suffix| normalized == *suffix || normalized.ends_with(suffix))
}

fn validate_existing_components(path: &Path) -> Result<(), DatabaseError> {
    let mut current = Some(path);
    while let Some(component) = current {
        let metadata = fs::symlink_metadata(component)
            .map_err(|_| DatabaseError::Cognitive("workspace_root_unavailable"))?;
        if metadata.file_type().is_symlink() {
            return Err(DatabaseError::Cognitive("workspace_root_invalid"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(DatabaseError::Cognitive("workspace_root_invalid"));
            }
        }
        current = component.parent();
    }
    Ok(())
}

fn reject_link_or_reparse(path: &Path, error: &'static str) -> Result<(), DatabaseError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?;
    if metadata.file_type().is_symlink() {
        return Err(DatabaseError::Cognitive(error));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(DatabaseError::Cognitive(error));
        }
    }
    Ok(())
}

fn validate_child_components(
    root: &Path,
    path: &Path,
    error: &'static str,
) -> Result<(), DatabaseError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| DatabaseError::Cognitive("workspace_path_invalid"))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(_) => reject_link_or_reparse(&current, error)?,
            Err(error_value) if error_value.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err(DatabaseError::Cognitive("workspace_path_unavailable")),
        }
    }
    Ok(())
}

fn relative_components(value: &str) -> Result<PathBuf, DatabaseError> {
    let normalized = validate_relative_path(value)?;
    let path = PathBuf::from(&normalized);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(DatabaseError::Cognitive("tool_scope_invalid"));
    }
    Ok(path)
}

fn safe_child(root: &Path, relative: &str, must_exist: bool) -> Result<PathBuf, DatabaseError> {
    let root = validate_workspace_root(root)?;
    let relative = relative_components(relative)?;
    let candidate = root.join(relative);
    let components_end = if must_exist {
        candidate.as_path()
    } else {
        candidate
            .parent()
            .ok_or(DatabaseError::Cognitive("workspace_path_invalid"))?
    };
    validate_child_components(&root, components_end, "workspace_path_invalid")?;
    if must_exist {
        let metadata = fs::symlink_metadata(&candidate)
            .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?;
        if metadata.file_type().is_symlink() {
            return Err(DatabaseError::Cognitive("workspace_path_invalid"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(DatabaseError::Cognitive("workspace_path_invalid"));
            }
        }
        let canonical = fs::canonicalize(&candidate)
            .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?;
        if !canonical.starts_with(root) {
            return Err(DatabaseError::Cognitive("workspace_path_invalid"));
        }
        Ok(canonical)
    } else {
        if candidate.exists() {
            return Err(DatabaseError::Cognitive("workspace_destination_exists"));
        }
        let parent = candidate
            .parent()
            .ok_or(DatabaseError::Cognitive("workspace_path_invalid"))?;
        let parent = fs::canonicalize(parent)
            .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?;
        if !parent.starts_with(root) {
            return Err(DatabaseError::Cognitive("workspace_path_invalid"));
        }
        Ok(candidate)
    }
}

fn capture_file_identity(path: &Path) -> Result<String, DatabaseError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| DatabaseError::Cognitive("workspace_compensation_unavailable"))?;
    if metadata.file_type().is_symlink() {
        return Err(DatabaseError::Cognitive(
            "workspace_compensation_unavailable",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{
                CreateFileW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
                FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
                FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            },
        };
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(DatabaseError::Cognitive(
                "workspace_compensation_unavailable",
            ));
        }
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let result = unsafe { GetFileInformationByHandle(handle, &mut info) };
        unsafe { CloseHandle(handle) };
        if result == 0 {
            return Err(DatabaseError::Cognitive(
                "workspace_compensation_unavailable",
            ));
        }
        Ok(format!(
            "win:{:x}:{:x}{:08x}",
            info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow
        ))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return Ok(format!("unix:{:x}:{:x}", metadata.dev(), metadata.ino()));
    }
    #[cfg(not(any(unix, windows)))]
    Err(DatabaseError::Cognitive(
        "workspace_compensation_unavailable",
    ))
}

fn local_compensation_moves(
    transaction: &Transaction<'_>,
    scope_ref: &str,
    input: &ToolActionInput,
) -> Result<Vec<ToolCompensationMove>, DatabaseError> {
    let ToolActionInput::WorkspaceOrganize { moves } = input else {
        return Err(DatabaseError::Cognitive(
            "workspace_compensation_unavailable",
        ));
    };
    let root = workspace_root_path(transaction, scope_ref)?;
    moves
        .iter()
        .map(|movement| {
            let destination = safe_child(&root, &movement.to, true)?;
            Ok(ToolCompensationMove {
                from: movement.from.clone(),
                to: movement.to.clone(),
                identity: capture_file_identity(&destination)?,
            })
        })
        .collect()
}

fn record_compensation_failure(
    transaction: &Transaction<'_>,
    action: &ToolAction,
    request: &ToolActionCancellationRequest,
    code: &'static str,
) -> Result<(), DatabaseError> {
    let result_json = serde_json::to_string(&ToolExecutionResult {
        status: ToolResultStatus::Failed,
        output: "A compensação local não foi aplicada integralmente; nenhum destino inesperado foi movido.".into(),
        changed: false,
        untrusted: true,
    })
    .map_err(|_| DatabaseError::Unavailable)?;
    transaction.execute(
        "UPDATE tool_actions SET status = 'failed', result_json = ?1, error_code = ?2,
         updated_at = ?3 WHERE id = ?4 AND agent_id = ?5",
        params![
            result_json,
            code,
            now_millis(),
            request.action_id,
            request.agent_id
        ],
    )?;
    audit_tx(
        transaction,
        AuditContext {
            action_id: Some(&request.action_id),
            session_id: Some(&action.session_id),
            agent_id: &request.agent_id,
            owner_id: OWNER_ID,
            tool_id: Some(&action.tool_id),
            event: "action_compensation_failed",
            result: "failed",
            code: Some(code),
            summary: "A compensação local falhou e não moveu destino não verificado.",
        },
    )?;
    Ok(())
}

fn map_tool_manifest(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolManifest> {
    let manifest_version: i64 = row.get(1)?;
    let classification = classification_from_str(&row.get::<_, String>(3)?)?;
    let adapter_kind = adapter_from_str(&row.get::<_, String>(4)?)?;
    let capabilities_json: String = row.get(7)?;
    let capabilities = parse_tool_capabilities(&capabilities_json)?;
    Ok(ToolManifest {
        tool_id: row.get(0)?,
        manifest_version,
        name: row.get(2)?,
        classification,
        adapter_kind,
        scope_kind: row.get(5)?,
        requires_second_confirmation: row.get(6)?,
        capabilities,
        updated_at: row.get(8)?,
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StoredToolCapabilities {
    List(Vec<String>),
    LegacyOperations { operations: Vec<String> },
}

fn parse_tool_capabilities(value: &str) -> rusqlite::Result<Vec<String>> {
    match serde_json::from_str(value).map_err(|_| invalid_query())? {
        StoredToolCapabilities::List(capabilities) => Ok(capabilities),
        StoredToolCapabilities::LegacyOperations { operations } => Ok(operations),
    }
}

fn map_tool_audit_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolAuditRecord> {
    let details_json: String = row.get(8)?;
    let details: serde_json::Value =
        serde_json::from_str(&details_json).map_err(|_| invalid_query())?;
    let summary = details
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(invalid_query)?
        .to_string();
    Ok(ToolAuditRecord {
        id: row.get(0)?,
        action_id: row.get(1)?,
        session_id: row.get(2)?,
        agent_id: row.get(3)?,
        tool_id: row.get(4)?,
        event: row.get(5)?,
        result: row.get(6)?,
        code: row.get(7)?,
        summary,
        created_at: row.get(9)?,
    })
}

fn invalid_query() -> rusqlite::Error {
    rusqlite::Error::InvalidQuery
}

fn classification_from_str(value: &str) -> rusqlite::Result<ToolClassification> {
    match value {
        "read_only" => Ok(ToolClassification::ReadOnly),
        "state_changing" => Ok(ToolClassification::StateChanging),
        _ => Err(invalid_query()),
    }
}

fn adapter_from_str(value: &str) -> rusqlite::Result<ToolAdapterKind> {
    match value {
        "workspace_mock" => Ok(ToolAdapterKind::Workspace),
        "calendar_mock" => Ok(ToolAdapterKind::Calendar),
        "messaging_mock" => Ok(ToolAdapterKind::Messaging),
        "workspace_local" => Ok(ToolAdapterKind::WorkspaceLocal),
        _ => Err(invalid_query()),
    }
}

fn permission_from_str(value: &str) -> rusqlite::Result<ToolPermission> {
    match value {
        "preview" => Ok(ToolPermission::Preview),
        "execute_read_only" => Ok(ToolPermission::ExecuteReadOnly),
        "execute_state_changing" => Ok(ToolPermission::ExecuteStateChanging),
        _ => Err(invalid_query()),
    }
}

fn permission_kind(permission: &ToolPermission) -> &'static str {
    match permission {
        ToolPermission::Preview => "preview",
        ToolPermission::ExecuteReadOnly => "execute_read_only",
        ToolPermission::ExecuteStateChanging => "execute_state_changing",
    }
}

fn classification_kind(classification: &ToolClassification) -> &'static str {
    match classification {
        ToolClassification::ReadOnly => "read_only",
        ToolClassification::StateChanging => "state_changing",
    }
}

fn session_status_from_str(value: &str) -> rusqlite::Result<ToolSessionStatus> {
    match value {
        "active" => Ok(ToolSessionStatus::Active),
        "cancelled" => Ok(ToolSessionStatus::Cancelled),
        "closed" => Ok(ToolSessionStatus::Closed),
        _ => Err(invalid_query()),
    }
}

fn action_status_from_str(value: &str) -> rusqlite::Result<ToolActionStatus> {
    match value {
        "previewed" => Ok(ToolActionStatus::Previewed),
        "approved" => Ok(ToolActionStatus::Approved),
        "confirmed" => Ok(ToolActionStatus::Confirmed),
        "dry_run" => Ok(ToolActionStatus::DryRun),
        "executed" => Ok(ToolActionStatus::Executed),
        "cancelled" => Ok(ToolActionStatus::Cancelled),
        "failed" => Ok(ToolActionStatus::Failed),
        "compensated" => Ok(ToolActionStatus::Compensated),
        "rejected" => Ok(ToolActionStatus::Rejected),
        _ => Err(invalid_query()),
    }
}

fn load_tool_manifest_tx(
    transaction: &Transaction<'_>,
    tool_id: &str,
) -> Result<ToolManifest, DatabaseError> {
    transaction
        .query_row(
            "SELECT tool_id, manifest_version, name, classification, adapter_kind,
                    scope_kind, requires_second_confirmation, capabilities_json, updated_at
             FROM tool_catalog WHERE tool_id = ?1",
            params![tool_id],
            map_tool_manifest,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("tool_not_found"))
}

fn load_tool_session_tx(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<ToolSession, DatabaseError> {
    let session = transaction
        .query_row(
            "SELECT id, agent_id, scope_ref, status, created_at, updated_at
             FROM tool_sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    session_status_from_str(&row.get::<_, String>(3)?)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("tool_session_not_found"))?;
    let mut statement = transaction.prepare(
        "SELECT tool_id, permission FROM tool_session_permissions
         WHERE session_id = ?1 ORDER BY tool_id, permission",
    )?;
    let permissions = statement
        .query_map(params![session_id], |row| {
            Ok(ToolSessionPermission {
                tool_id: row.get(0)?,
                permission: permission_from_str(&row.get::<_, String>(1)?)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ToolSession {
        id: session.0,
        agent_id: session.1,
        scope_ref: session.2,
        status: session.3,
        permissions,
        created_at: session.4,
        updated_at: session.5,
    })
}

fn load_tool_session_connection(
    connection: &Connection,
    session_id: &str,
) -> Result<ToolSession, DatabaseError> {
    let session = connection
        .query_row(
            "SELECT id, agent_id, scope_ref, status, created_at, updated_at
             FROM tool_sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    session_status_from_str(&row.get::<_, String>(3)?)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("tool_session_not_found"))?;
    let mut statement = connection.prepare(
        "SELECT tool_id, permission FROM tool_session_permissions
         WHERE session_id = ?1 ORDER BY tool_id, permission",
    )?;
    let permissions = statement
        .query_map(params![session_id], |row| {
            Ok(ToolSessionPermission {
                tool_id: row.get(0)?,
                permission: permission_from_str(&row.get::<_, String>(1)?)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ToolSession {
        id: session.0,
        agent_id: session.1,
        scope_ref: session.2,
        status: session.3,
        permissions,
        created_at: session.4,
        updated_at: session.5,
    })
}

fn load_tool_action_tx(
    transaction: &Transaction<'_>,
    action_id: &str,
) -> Result<ToolAction, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, session_id, agent_id, tool_id, classification, input_json,
                    summary, affected_resources_json, exact_effect, status, dry_run,
                    requires_second_confirmation, owner_approved, second_confirmed,
                    result_json, compensation_json, error_code, created_at, updated_at
             FROM tool_actions WHERE id = ?1",
            params![action_id],
            map_tool_action,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("tool_action_not_found"))
}

fn load_tool_action_for_agent_tx(
    transaction: &Transaction<'_>,
    action_id: &str,
    agent_id: &str,
) -> Result<ToolAction, DatabaseError> {
    let action = load_tool_action_tx(transaction, action_id)?;
    if action.agent_id != agent_id {
        return Err(DatabaseError::Cognitive("tool_action_not_found"));
    }
    Ok(action)
}

fn map_tool_action(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolAction> {
    let classification = classification_from_str(&row.get::<_, String>(4)?)?;
    let input: ToolActionInput =
        serde_json::from_str(&row.get::<_, String>(5)?).map_err(|_| invalid_query())?;
    let affected_resources: Vec<String> =
        serde_json::from_str(&row.get::<_, String>(7)?).map_err(|_| invalid_query())?;
    let result_json: Option<String> = row.get(14)?;
    let result = result_json
        .map(|json| serde_json::from_str::<ToolExecutionResult>(&json).map_err(|_| invalid_query()))
        .transpose()?;
    let compensation_json: Option<String> = row.get(15)?;
    let compensation = compensation_json
        .map(|json| serde_json::from_str::<ToolCompensation>(&json).map_err(|_| invalid_query()))
        .transpose()?;
    Ok(ToolAction {
        id: row.get(0)?,
        session_id: row.get(1)?,
        agent_id: row.get(2)?,
        tool_id: row.get(3)?,
        requires_owner_approval: classification == ToolClassification::StateChanging,
        classification,
        input,
        summary: row.get(6)?,
        affected_resources,
        exact_effect: row.get(8)?,
        status: action_status_from_str(&row.get::<_, String>(9)?)?,
        dry_run: row.get(10)?,
        requires_second_confirmation: row.get(11)?,
        owner_approved: row.get(12)?,
        second_confirmed: row.get(13)?,
        result,
        compensation,
        code: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

fn ensure_not_temporary(temporary_chat: bool) -> Result<(), DatabaseError> {
    if temporary_chat {
        Err(DatabaseError::Cognitive("tools_blocked_temporary"))
    } else {
        Ok(())
    }
}

fn valid_idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        return Err(DatabaseError::Cognitive("invalid_idempotency_key"));
    }
    Ok(value.to_string())
}

fn validate_scope(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.starts_with("workspace_root:") {
        let root_id = value.strip_prefix("workspace_root:").unwrap_or_default();
        if root_id.is_empty()
            || root_id.len() > 64
            || root_id
                .chars()
                .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
        {
            return Err(DatabaseError::Cognitive("tool_scope_invalid"));
        }
        return Ok(value.to_string());
    }
    if value.len() > MAX_SCOPE_BYTES
        || !value.starts_with("fixture:")
        || value.contains("..")
        || value.contains('\\')
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._/-".contains(character)))
    {
        return Err(DatabaseError::Cognitive("tool_scope_invalid"));
    }
    if ![
        "fixture:workspace/",
        "fixture:calendar/",
        "fixture:messaging/",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix) && value.len() > prefix.len())
    {
        return Err(DatabaseError::Cognitive("tool_scope_invalid"));
    }
    Ok(value.to_string())
}

fn validate_fixture_reference(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_REFERENCE_BYTES
        || value.contains("..")
        || value.contains('\\')
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._/-".contains(character)))
    {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    Ok(value.to_string())
}

fn validate_text(value: &str, maximum: usize) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > maximum
        || value
            .chars()
            .any(|character| character == '\0' || character.is_control())
    {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    Ok(value.to_string())
}

fn validate_relative_path(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_REFERENCE_BYTES
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains("..")
        || value.contains('\\')
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || ".-_ /".replace(' ', "").contains(character))
        })
    {
        return Err(DatabaseError::Cognitive("tool_scope_invalid"));
    }
    Ok(value.to_string())
}

fn normalize_permissions(
    transaction: &Transaction<'_>,
    scope_ref: &str,
    requested: &[ToolSessionPermission],
) -> Result<Vec<ToolSessionPermission>, DatabaseError> {
    if requested.is_empty() || requested.len() > MAX_SESSION_PERMISSIONS {
        return Err(DatabaseError::Cognitive("tool_permission_invalid"));
    }
    let mut permissions = requested.to_vec();
    permissions.sort_by(|left, right| {
        left.tool_id
            .cmp(&right.tool_id)
            .then(permission_kind(&left.permission).cmp(permission_kind(&right.permission)))
    });
    for pair in permissions.windows(2) {
        if pair[0].tool_id == pair[1].tool_id
            && permission_kind(&pair[0].permission) == permission_kind(&pair[1].permission)
        {
            return Err(DatabaseError::Cognitive("tool_permission_invalid"));
        }
    }
    let mut scope_kind: Option<String> = None;
    for permission in &permissions {
        let manifest = load_tool_manifest_tx(transaction, &permission.tool_id)?;
        if manifest.manifest_version != TOOL_MANIFEST_VERSION {
            return Err(DatabaseError::Cognitive("tool_manifest_invalid"));
        }
        match (&manifest.classification, &permission.permission) {
            (_, ToolPermission::Preview) => {}
            (ToolClassification::ReadOnly, ToolPermission::ExecuteReadOnly) => {}
            (ToolClassification::StateChanging, ToolPermission::ExecuteStateChanging) => {}
            _ => return Err(DatabaseError::Cognitive("tool_permission_invalid")),
        }
        if let Some(existing) = &scope_kind {
            if existing != &manifest.scope_kind {
                return Err(DatabaseError::Cognitive("tool_scope_invalid"));
            }
        } else {
            scope_kind = Some(manifest.scope_kind.clone());
        }
        if !scope_prefix(&manifest.scope_kind).is_some_and(|prefix| scope_ref.starts_with(prefix)) {
            return Err(DatabaseError::Cognitive("tool_scope_invalid"));
        }
    }
    Ok(permissions)
}

fn scope_prefix(scope_kind: &str) -> Option<&'static str> {
    match scope_kind {
        "workspace" => Some("fixture:workspace/"),
        "calendar" => Some("fixture:calendar/"),
        "messaging" => Some("fixture:messaging/"),
        "workspace_root" => Some("workspace_root:"),
        _ => None,
    }
}

fn ensure_permission(
    transaction: &Transaction<'_>,
    session_id: &str,
    tool_id: &str,
    permission: &ToolPermission,
) -> Result<(), DatabaseError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM tool_session_permissions
           WHERE session_id = ?1 AND tool_id = ?2 AND permission = ?3
         )",
        params![session_id, tool_id, permission_kind(permission)],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("tool_permission_denied"))
    }
}

fn ensure_tools_enabled_tx(transaction: &Transaction<'_>) -> Result<(), DatabaseError> {
    let safe_mode = transaction
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = 'safe_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some_and(|value| value == "true");
    if safe_mode {
        Err(DatabaseError::Cognitive("tools_blocked_safe_mode"))
    } else {
        Ok(())
    }
}

fn ensure_session_agent(session: &ToolSession, agent_id: &str) -> Result<(), DatabaseError> {
    if session.agent_id == agent_id {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("tool_session_not_found"))
    }
}

fn ensure_session_active(session: &ToolSession) -> Result<(), DatabaseError> {
    if session.status == ToolSessionStatus::Active {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("tool_session_cancelled"))
    }
}

fn ensure_owner(connection: &Connection, agent_id: &str) -> Result<String, DatabaseError> {
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

fn ensure_owner_tx(transaction: &Transaction<'_>, agent_id: &str) -> Result<String, DatabaseError> {
    transaction
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

fn validate_action_input(
    transaction: &Transaction<'_>,
    manifest: &ToolManifest,
    scope_ref: &str,
    input: &ToolActionInput,
    capture_local_identity: bool,
) -> Result<ToolPreviewPlan, DatabaseError> {
    let Some(prefix) = scope_prefix(&manifest.scope_kind) else {
        return Err(DatabaseError::Cognitive("tool_manifest_invalid"));
    };
    if !scope_ref.starts_with(prefix) {
        return Err(DatabaseError::Cognitive("tool_scope_invalid"));
    }
    if manifest.adapter_kind == ToolAdapterKind::WorkspaceLocal {
        return validate_local_action_input(
            transaction,
            manifest,
            scope_ref,
            input,
            capture_local_identity,
        );
    }
    match (manifest.tool_id.as_str(), input) {
        ("workspace.inspect_scope", ToolActionInput::WorkspaceInspect { relative_paths }) => {
            if relative_paths.is_empty() || relative_paths.len() > 32 {
                return Err(DatabaseError::Cognitive("tool_input_invalid"));
            }
            let paths = relative_paths
                .iter()
                .map(|path| validate_relative_path(path))
                .collect::<Result<Vec<_>, _>>()?;
            let affected_resources = paths
                .iter()
                .map(|path| format!("{scope_ref}/{path}"))
                .collect::<Vec<_>>();
            Ok(ToolPreviewPlan {
                input: ToolActionInput::WorkspaceInspect {
                    relative_paths: paths,
                },
                summary: format!(
                    "Inspecionar {} entrada(s) da área fixture; nenhum arquivo real será lido.",
                    affected_resources.len()
                ),
                affected_resources,
                exact_effect: "O mock retorna metadados determinísticos; não acessa o sistema de arquivos do host.".into(),
            })
        }
        ("workspace.organize_files", ToolActionInput::WorkspaceOrganize { moves }) => {
            if moves.is_empty() || moves.len() > 32 {
                return Err(DatabaseError::Cognitive("tool_input_invalid"));
            }
            let mut normalized = Vec::with_capacity(moves.len());
            let mut affected_resources = Vec::with_capacity(moves.len() * 2);
            for movement in moves {
                let from = validate_relative_path(&movement.from)?;
                let to = validate_relative_path(&movement.to)?;
                if from == to {
                    return Err(DatabaseError::Cognitive("tool_input_invalid"));
                }
                affected_resources.push(format!("{scope_ref}/{from}"));
                affected_resources.push(format!("{scope_ref}/{to}"));
                normalized.push(ToolFileMove {
                    from,
                    to,
                    source_identity: None,
                });
            }
            Ok(ToolPreviewPlan {
                input: ToolActionInput::WorkspaceOrganize { moves: normalized },
                summary: format!(
                    "Organizar {} movimento(s) na área fixture; nenhum arquivo real será alterado.",
                    moves.len()
                ),
                affected_resources,
                exact_effect: "O mock apenas simula a organização dentro da área fixture; não cria, move, renomeia ou apaga arquivos do host.".into(),
            })
        }
        ("calendar.list_events", ToolActionInput::CalendarList { date }) => {
            let date = validate_date(date)?;
            Ok(ToolPreviewPlan {
                input: ToolActionInput::CalendarList { date: date.clone() },
                summary: format!("Listar eventos do calendário fixture em {date}."),
                affected_resources: vec![format!("{scope_ref}/{date}")],
                exact_effect: "O mock retorna eventos determinísticos; não acessa nenhuma conta ou calendário real.".into(),
            })
        }
        (
            "calendar.create_event",
            ToolActionInput::CalendarCreate {
                title,
                date,
                start,
                end,
            },
        ) => {
            let title = validate_text(title, 120)?;
            let date = validate_date(date)?;
            let start = validate_time(start)?;
            let end = validate_time(end)?;
            if start >= end {
                return Err(DatabaseError::Cognitive("tool_input_invalid"));
            }
            Ok(ToolPreviewPlan {
                input: ToolActionInput::CalendarCreate {
                    title: title.clone(),
                    date: date.clone(),
                    start,
                    end,
                },
                summary: format!("Criar evento fixture \"{title}\" em {date}; nenhum calendário real será alterado."),
                affected_resources: vec![format!("{scope_ref}/{date}")],
                exact_effect: "O mock não cria eventos, não contata provedores e retorna apenas uma confirmação simulada.".into(),
            })
        }
        ("messaging.preview_message", ToolActionInput::MessagingPreview { recipient, body }) => {
            let (recipient, body) = validate_message(recipient, body)?;
            Ok(ToolPreviewPlan {
                input: ToolActionInput::MessagingPreview { recipient: recipient.clone(), body },
                summary: format!("Pré-visualizar mensagem fixture para {recipient}; nenhuma mensagem será enviada."),
                affected_resources: vec![recipient],
                exact_effect: "O mock apenas mostra uma prévia limitada; não acessa contas, contatos ou rede.".into(),
            })
        }
        ("messaging.send_message", ToolActionInput::MessagingSend { recipient, body }) => {
            let (recipient, body) = validate_message(recipient, body)?;
            Ok(ToolPreviewPlan {
                input: ToolActionInput::MessagingSend { recipient: recipient.clone(), body },
                summary: format!("Enviar mensagem fixture para {recipient}; nenhum serviço real será contatado."),
                affected_resources: vec![recipient],
                exact_effect: "O mock retorna metadados de envio sem enviar, persistir ou transmitir a mensagem.".into(),
            })
        }
        _ => Err(DatabaseError::Cognitive("tool_input_invalid")),
    }
}

fn validate_local_action_input(
    transaction: &Transaction<'_>,
    manifest: &ToolManifest,
    scope_ref: &str,
    input: &ToolActionInput,
    capture_local_identity: bool,
) -> Result<ToolPreviewPlan, DatabaseError> {
    let root = workspace_root_path(transaction, scope_ref)?;
    match (manifest.tool_id.as_str(), input) {
        ("workspace.inspect_local", ToolActionInput::WorkspaceInspect { relative_paths }) => {
            if relative_paths.is_empty() || relative_paths.len() > 32 {
                return Err(DatabaseError::Cognitive("tool_input_invalid"));
            }
            for path in relative_paths {
                let item = safe_child(&root, path, true)?;
                if !fs::metadata(&item)
                    .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?
                    .is_file()
                {
                    return Err(DatabaseError::Cognitive("workspace_path_invalid"));
                }
            }
            let paths = relative_paths
                .iter()
                .map(|path| validate_relative_path(path))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ToolPreviewPlan {
                input: ToolActionInput::WorkspaceInspect {
                    relative_paths: paths.clone(),
                },
                summary: format!(
                    "Inspecionar {} arquivo(s) locais; conteúdo não será retornado.",
                    paths.len()
                ),
                affected_resources: paths
                    .iter()
                    .map(|path| format!("{scope_ref}/{path}"))
                    .collect(),
                exact_effect:
                    "Ler somente nomes e metadados limitados dentro da raiz Owner configurada."
                        .into(),
            })
        }
        ("workspace.organize_local", ToolActionInput::WorkspaceOrganize { moves }) => {
            if moves.is_empty() || moves.len() > 32 {
                return Err(DatabaseError::Cognitive("tool_input_invalid"));
            }
            let mut normalized = Vec::with_capacity(moves.len());
            let mut affected_resources = Vec::with_capacity(moves.len() * 2);
            for movement in moves {
                let from = validate_relative_path(&movement.from)?;
                let to = validate_relative_path(&movement.to)?;
                if from == to {
                    return Err(DatabaseError::Cognitive("tool_input_invalid"));
                }
                let source = safe_child(&root, &from, true)?;
                if !fs::metadata(&source)
                    .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?
                    .is_file()
                {
                    return Err(DatabaseError::Cognitive("workspace_path_invalid"));
                }
                let _destination = safe_child(&root, &to, false)?;
                affected_resources.push(format!("{scope_ref}/{from}"));
                affected_resources.push(format!("{scope_ref}/{to}"));
                let source_identity = if capture_local_identity {
                    capture_file_identity(&source)?
                } else {
                    movement
                        .source_identity
                        .clone()
                        .ok_or(DatabaseError::Cognitive(
                            "workspace_source_identity_unavailable",
                        ))?
                };
                normalized.push(ToolFileMove {
                    source_identity: Some(source_identity),
                    from,
                    to,
                });
            }
            Ok(ToolPreviewPlan {
                input: ToolActionInput::WorkspaceOrganize {
                    moves: normalized.clone(),
                },
                summary: format!(
                    "Organizar {} arquivo(s) locais após confirmação do Proprietário.",
                    normalized.len()
                ),
                affected_resources,
                exact_effect:
                    "Mover somente arquivos dentro da raiz Owner configurada; sem exclusão.".into(),
            })
        }
        _ => Err(DatabaseError::Cognitive("tool_input_invalid")),
    }
}

fn execute_adapter(
    transaction: &Transaction<'_>,
    manifest: &ToolManifest,
    plan: &ToolPreviewPlan,
    dry_run: bool,
) -> Result<ToolExecutionResult, DatabaseError> {
    if manifest.adapter_kind != ToolAdapterKind::WorkspaceLocal {
        return mock_execute(manifest, plan, dry_run);
    }
    let root = workspace_root_path(
        transaction,
        plan.affected_resources[0]
            .split('/')
            .next()
            .unwrap_or_default(),
    )?;
    match &plan.input {
        ToolActionInput::WorkspaceInspect { relative_paths } => {
            let mut output = Vec::new();
            for path in relative_paths {
                let item = safe_child(&root, path, true)?;
                let metadata = fs::metadata(item)
                    .map_err(|_| DatabaseError::Cognitive("workspace_path_unavailable"))?;
                output.push(json!({"path": path, "bytes": metadata.len()}));
            }
            let output = serde_json::to_string(&output).map_err(|_| DatabaseError::Unavailable)?;
            if output.len() > MAX_OUTPUT_BYTES {
                return Err(DatabaseError::Cognitive("tool_output_oversized"));
            }
            Ok(ToolExecutionResult {
                status: if dry_run {
                    ToolResultStatus::DryRun
                } else {
                    ToolResultStatus::Executed
                },
                output,
                changed: false,
                untrusted: true,
            })
        }
        ToolActionInput::WorkspaceOrganize { moves } => {
            if dry_run {
                return Ok(ToolExecutionResult {
                    status: ToolResultStatus::DryRun,
                    output: format!("{} movimento(s) local(is) simulados.", moves.len()),
                    changed: false,
                    untrusted: true,
                });
            }
            let mut preflight = Vec::with_capacity(moves.len());
            for movement in moves {
                let expected =
                    movement
                        .source_identity
                        .as_deref()
                        .ok_or(DatabaseError::Cognitive(
                            "workspace_source_identity_unavailable",
                        ))?;
                let source = safe_child(&root, &movement.from, true)?;
                let _destination = safe_child(&root, &movement.to, false)?;
                if capture_file_identity(&source)? != expected {
                    return Err(DatabaseError::Cognitive(
                        "workspace_source_identity_mismatch",
                    ));
                }
                preflight.push(movement);
            }
            let mut completed: Vec<(String, String)> = Vec::new();
            for movement in preflight {
                let source = safe_child(&root, &movement.from, true)?;
                let destination = safe_child(&root, &movement.to, false)?;
                if capture_file_identity(&source)?
                    != movement.source_identity.as_deref().unwrap_or_default()
                {
                    return Err(DatabaseError::Cognitive(
                        "workspace_source_identity_mismatch",
                    ));
                }
                if fs::rename(&source, &destination).is_err() {
                    let mut rollback_failed = false;
                    for (original, moved) in completed.iter().rev() {
                        let current = safe_child(&root, moved, true);
                        let target = safe_child(&root, original, false);
                        if current.is_err()
                            || target.is_err()
                            || fs::rename(current.unwrap(), target.unwrap()).is_err()
                        {
                            rollback_failed = true;
                        }
                    }
                    return Err(DatabaseError::Cognitive(if rollback_failed {
                        "workspace_move_partial"
                    } else {
                        "workspace_move_failed"
                    }));
                }
                completed.push((movement.from.clone(), movement.to.clone()));
            }
            Ok(ToolExecutionResult {
                status: ToolResultStatus::Executed,
                output: format!("{} movimento(s) local(is) executado(s).", moves.len()),
                changed: true,
                untrusted: true,
            })
        }
        _ => Err(DatabaseError::Cognitive("tool_input_invalid")),
    }
}

fn validate_date(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    let month = value[5..7]
        .parse::<u8>()
        .map_err(|_| DatabaseError::Cognitive("tool_input_invalid"))?;
    let day = value[8..10]
        .parse::<u8>()
        .map_err(|_| DatabaseError::Cognitive("tool_input_invalid"))?;
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    Ok(value.to_string())
}

fn validate_time(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
    {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    let hour = value[0..2]
        .parse::<u8>()
        .map_err(|_| DatabaseError::Cognitive("tool_input_invalid"))?;
    let minute = value[3..5]
        .parse::<u8>()
        .map_err(|_| DatabaseError::Cognitive("tool_input_invalid"))?;
    if hour > 23 || minute > 59 {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    Ok(value.to_string())
}

fn validate_message(recipient: &str, body: &str) -> Result<(String, String), DatabaseError> {
    let recipient = validate_fixture_reference(recipient)?;
    if !recipient.starts_with("fixture:recipient-") {
        return Err(DatabaseError::Cognitive("tool_input_invalid"));
    }
    let body = validate_text(body, MAX_TEXT_BYTES)?;
    Ok((recipient, body))
}

fn mock_execute(
    manifest: &ToolManifest,
    plan: &ToolPreviewPlan,
    dry_run: bool,
) -> Result<ToolExecutionResult, DatabaseError> {
    let output = match (&manifest.tool_id[..], &plan.input) {
        ("workspace.inspect_scope", ToolActionInput::WorkspaceInspect { relative_paths }) => {
            format!(
                "Fixture workspace inspecionado: {} entrada(s); nenhum caminho do host foi acessado.",
                relative_paths.len()
            )
        }
        ("workspace.organize_files", ToolActionInput::WorkspaceOrganize { moves }) => format!(
            "Mock de organização concluído para {} movimento(s); nenhum arquivo real foi alterado.",
            moves.len()
        ),
        ("calendar.list_events", ToolActionInput::CalendarList { date }) => {
            format!("Calendário fixture: nenhum evento para {date}.")
        }
        ("calendar.create_event", ToolActionInput::CalendarCreate { title, date, .. }) => {
            format!("Mock de evento \"{title}\" em {date}; nenhum calendário real foi alterado.")
        }
        ("messaging.preview_message", ToolActionInput::MessagingPreview { recipient, .. }) => {
            format!("Prévia fixture para {recipient}; nenhuma mensagem foi enviada.")
        }
        ("messaging.send_message", ToolActionInput::MessagingSend { recipient, .. }) => {
            format!("Mock de mensagem para {recipient}; nenhum serviço real foi contatado.")
        }
        _ => return Err(DatabaseError::Cognitive("tool_input_invalid")),
    };
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(DatabaseError::Cognitive("tool_output_oversized"));
    }
    Ok(ToolExecutionResult {
        status: if dry_run {
            ToolResultStatus::DryRun
        } else {
            ToolResultStatus::Simulated
        },
        output,
        changed: false,
        untrusted: true,
    })
}

struct AuditContext<'a> {
    action_id: Option<&'a str>,
    session_id: Option<&'a str>,
    agent_id: &'a str,
    owner_id: &'a str,
    tool_id: Option<&'a str>,
    event: &'a str,
    result: &'a str,
    code: Option<&'a str>,
    summary: &'a str,
}

fn audit_tx(transaction: &Transaction<'_>, context: AuditContext<'_>) -> Result<(), DatabaseError> {
    let details_json = serde_json::to_string(&AuditDetails {
        summary: context.summary,
    })
    .map_err(|_| DatabaseError::Unavailable)?;
    if details_json.len() > MAX_AUDIT_BYTES {
        return Err(DatabaseError::Cognitive("tool_audit_oversized"));
    }
    transaction.execute(
        "DELETE FROM tool_audit_log WHERE created_at < ?1",
        params![now_millis() - AUDIT_RETENTION_MS],
    )?;
    transaction.execute(
        "INSERT INTO tool_audit_log
         (id, action_id, session_id, agent_id, owner_user_id, tool_id, event, result, code,
          details_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            Uuid::now_v7().to_string(),
            context.action_id,
            context.session_id,
            context.agent_id,
            context.owner_id,
            context.tool_id,
            context.event,
            context.result,
            context.code,
            details_json,
            now_millis(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use uuid::Uuid;

    use super::*;
    use crate::database::ASTRA_ID;

    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("aip-tools-test-{}", Uuid::now_v7()))
            .join("aip.sqlite3")
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path.parent().expect("test path should have a parent"));
    }

    fn calendar_session(database: &Database) -> ToolSession {
        database
            .create_tool_session(ToolSessionRequest {
                agent_id: ASTRA_ID.into(),
                scope_ref: "fixture:calendar/owner".into(),
                permissions: vec![
                    ToolSessionPermission {
                        tool_id: "calendar.create_event".into(),
                        permission: ToolPermission::Preview,
                    },
                    ToolSessionPermission {
                        tool_id: "calendar.create_event".into(),
                        permission: ToolPermission::ExecuteStateChanging,
                    },
                ],
                idempotency_key: "calendar-session".into(),
                temporary_chat: false,
            })
            .expect("calendar session should be created")
    }

    fn calendar_input() -> ToolActionInput {
        ToolActionInput::CalendarCreate {
            title: "Revisão local".into(),
            date: "2026-08-20".into(),
            start: "10:00".into(),
            end: "11:00".into(),
        }
    }

    #[test]
    fn catalog_and_state_changing_actions_require_owner_confirmation() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        let catalog = database.list_tool_catalog().expect("catalog should load");
        assert!(catalog.len() >= 8);
        assert!(catalog.iter().any(|manifest| {
            manifest.tool_id == "calendar.create_event"
                && manifest.classification == ToolClassification::StateChanging
                && manifest.requires_second_confirmation
        }));

        let session = calendar_session(&database);
        let preview = database
            .preview_tool_action(ToolActionPreviewRequest {
                agent_id: ASTRA_ID.into(),
                session_id: session.id.clone(),
                tool_id: "calendar.create_event".into(),
                input: calendar_input(),
                dry_run: false,
                idempotency_key: "calendar-action".into(),
                temporary_chat: false,
            })
            .expect("action should be previewed");
        assert_eq!(preview.status, ToolActionStatus::Previewed);
        assert!(preview.requires_owner_approval);
        assert!(preview.requires_second_confirmation);
        assert_eq!(
            database.execute_tool_action(ToolActionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                dry_run: false,
                idempotency_key: "calendar-execute-before-approval".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("tool_approval_required"))
        );

        let approved = database
            .decide_tool_action(ToolActionDecisionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                approved: true,
                idempotency_key: "calendar-approve".into(),
                temporary_chat: false,
            })
            .expect("owner approval should be recorded");
        assert_eq!(approved.status, ToolActionStatus::Approved);
        assert!(approved.owner_approved);
        assert_eq!(
            database.execute_tool_action(ToolActionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                dry_run: false,
                idempotency_key: "calendar-execute-before-confirmation".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("tool_confirmation_required"))
        );

        let confirmed = database
            .confirm_tool_action(ToolActionConfirmationRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                idempotency_key: "calendar-confirm".into(),
                temporary_chat: false,
            })
            .expect("second confirmation should be recorded");
        assert_eq!(confirmed.status, ToolActionStatus::Confirmed);
        assert!(confirmed.second_confirmed);

        let executed = database
            .execute_tool_action(ToolActionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                dry_run: false,
                idempotency_key: "calendar-execute".into(),
                temporary_chat: false,
            })
            .expect("mock action should execute");
        assert_eq!(executed.status, ToolActionStatus::Executed);
        let result = executed.result.as_ref().expect("result should be present");
        assert_eq!(result.status, ToolResultStatus::Simulated);
        assert!(!result.changed);
        assert!(result.untrusted);
        assert!(executed.compensation.is_some_and(|value| value.available));

        let compensated = database
            .compensate_tool_action(ToolActionCancellationRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                idempotency_key: "calendar-compensate".into(),
                temporary_chat: false,
            })
            .expect("mock compensation should be recorded");
        assert_eq!(compensated.status, ToolActionStatus::Compensated);
        let audit = database
            .list_tool_audit(ASTRA_ID)
            .expect("audit should load");
        for event in [
            "action_previewed",
            "action_approved",
            "action_confirmed",
            "action_executed",
            "action_compensated",
        ] {
            assert!(audit.iter().any(|record| record.event == event));
        }
        cleanup(&path);
    }

    #[test]
    fn local_workspace_root_move_is_bounded_approved_executed_and_compensated() {
        let path = test_path();
        let workspace = path.parent().unwrap().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("draft.txt"), b"bounded").unwrap();
        fs::write(workspace.join("substitute.txt"), b"substituted-source").unwrap();
        let database = Database::initialize(&path).unwrap();
        let root = database
            .add_workspace_root(WorkspaceRootRequest {
                path: workspace.to_string_lossy().into_owned(),
                idempotency_key: "root-add".into(),
                temporary_chat: false,
            })
            .unwrap_or_else(|error| panic!("local session error: {error:?}"));
        assert!(root.id.starts_with("wrt_"));
        let session = database
            .create_tool_session(ToolSessionRequest {
                agent_id: ASTRA_ID.into(),
                scope_ref: format!("workspace_root:{}", root.id),
                permissions: vec![
                    ToolSessionPermission {
                        tool_id: "workspace.organize_local".into(),
                        permission: ToolPermission::Preview,
                    },
                    ToolSessionPermission {
                        tool_id: "workspace.organize_local".into(),
                        permission: ToolPermission::ExecuteStateChanging,
                    },
                ],
                idempotency_key: "local-session".into(),
                temporary_chat: false,
            })
            .unwrap_or_else(|error| panic!("local session error: {error:?}"));
        assert_eq!(
            database.preview_tool_action(ToolActionPreviewRequest {
                agent_id: ASTRA_ID.into(),
                session_id: session.id.clone(),
                tool_id: "workspace.organize_local".into(),
                input: ToolActionInput::WorkspaceOrganize {
                    moves: vec![ToolFileMove {
                        from: "../draft.txt".into(),
                        to: "organized.txt".into(),
                        source_identity: None,
                    }],
                },
                dry_run: false,
                idempotency_key: "local-traversal".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("tool_scope_invalid"))
        );
        let preview = database
            .preview_tool_action(ToolActionPreviewRequest {
                agent_id: ASTRA_ID.into(),
                session_id: session.id.clone(),
                tool_id: "workspace.organize_local".into(),
                input: ToolActionInput::WorkspaceOrganize {
                    moves: vec![ToolFileMove {
                        from: "draft.txt".into(),
                        to: "organized.txt".into(),
                        source_identity: Some("forged:caller:identity".into()),
                    }],
                },
                dry_run: false,
                idempotency_key: "local-move".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(
            database.execute_tool_action(ToolActionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                dry_run: false,
                idempotency_key: "local-before-approval".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("tool_approval_required"))
        );
        database
            .decide_tool_action(ToolActionDecisionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                approved: true,
                idempotency_key: "local-approve".into(),
                temporary_chat: false,
            })
            .unwrap();
        database
            .confirm_tool_action(ToolActionConfirmationRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                idempotency_key: "local-confirm".into(),
                temporary_chat: false,
            })
            .unwrap();
        fs::rename(workspace.join("draft.txt"), workspace.join("original.txt")).unwrap();
        fs::rename(
            workspace.join("substitute.txt"),
            workspace.join("draft.txt"),
        )
        .unwrap();
        let execution_error = database
            .execute_tool_action(ToolActionExecutionRequest {
                agent_id: ASTRA_ID.into(),
                action_id: preview.id.clone(),
                dry_run: false,
                idempotency_key: "local-execute".into(),
                temporary_chat: false,
            })
            .unwrap_err();
        assert_eq!(
            execution_error,
            DatabaseError::Cognitive("workspace_source_identity_mismatch")
        );
        let connection = database.open().unwrap();
        let (status, error_code): (String, String) = connection
            .query_row(
                "SELECT status, error_code FROM tool_actions WHERE id = ?1",
                [&preview.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(error_code, "workspace_source_identity_mismatch");
        assert!(workspace.join("draft.txt").is_file());
        assert!(!workspace.join("organized.txt").exists());
        cleanup(&path);
    }

    #[test]
    fn workspace_root_policy_rejects_broad_directories_but_allows_nested_workspaces() {
        for path in [
            "/", "/home", "/tmp", "/var", "/etc", "/usr", "/opt", "/Users",
        ] {
            assert!(is_broad_workspace_root(Path::new(path)));
        }
        for path in [
            "/home/owner/workspace",
            "/tmp/aip-workspace",
            "C:/Users/Owner/workspace",
        ] {
            assert!(!is_broad_workspace_root(Path::new(path)));
        }
        assert!(is_broad_workspace_root(Path::new("C:/")));
        assert!(is_broad_workspace_root(Path::new("C:/Windows/System32")));
        assert!(is_broad_workspace_root(Path::new("C:/ProgramData")));
    }

    #[cfg(unix)]
    #[test]
    fn nested_symlink_component_is_rejected_before_local_access() {
        use std::os::unix::fs::symlink;

        let path = test_path();
        let workspace = path.parent().unwrap().join("nested-link-workspace");
        let target = workspace.join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("source.txt"), b"bounded").unwrap();
        symlink(&target, workspace.join("nested")).unwrap();

        let result = safe_child(
            &validate_workspace_root(&workspace).unwrap(),
            "nested/source.txt",
            true,
        );
        assert_eq!(
            result,
            Err(DatabaseError::Cognitive("workspace_path_invalid"))
        );
        assert!(!workspace.join("nested").join("moved.txt").exists());
        cleanup(&path);
    }

    #[cfg(windows)]
    #[test]
    fn nested_reparse_component_is_rejected_when_link_creation_is_available() {
        use std::os::windows::fs::symlink_dir;

        let path = test_path();
        let workspace = path.parent().unwrap().join("nested-reparse-workspace");
        let target = workspace.join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("source.txt"), b"bounded").unwrap();
        if symlink_dir(&target, workspace.join("nested")).is_err() {
            cleanup(&path);
            return;
        }

        let result = safe_child(
            &validate_workspace_root(&workspace).unwrap(),
            "nested/source.txt",
            true,
        );
        assert_eq!(
            result,
            Err(DatabaseError::Cognitive("workspace_path_invalid"))
        );
        assert!(!workspace.join("nested").join("moved.txt").exists());
        cleanup(&path);
    }

    #[test]
    fn local_multi_move_failure_rolls_back_without_overwriting_destination() {
        let path = test_path();
        let workspace = path.parent().unwrap().join("partial-workspace");
        fs::create_dir_all(workspace.join("blocked")).unwrap();
        fs::write(workspace.join("first.txt"), b"first").unwrap();
        fs::write(workspace.join("second.txt"), b"second").unwrap();
        let database = Database::initialize(&path).unwrap();
        let root = database
            .add_workspace_root(WorkspaceRootRequest {
                path: workspace.to_string_lossy().into_owned(),
                idempotency_key: "partial-root".into(),
                temporary_chat: false,
            })
            .unwrap();
        let mut connection = database.open().unwrap();
        let transaction = connection.transaction().unwrap();
        let manifest = load_tool_manifest_tx(&transaction, "workspace.organize_local").unwrap();
        let input = ToolActionInput::WorkspaceOrganize {
            moves: vec![
                ToolFileMove {
                    from: "first.txt".into(),
                    to: "done.txt".into(),
                    source_identity: None,
                },
                ToolFileMove {
                    from: "second.txt".into(),
                    to: "blocked/out.txt".into(),
                    source_identity: None,
                },
            ],
        };
        let plan = validate_local_action_input(
            &transaction,
            &manifest,
            &format!("workspace_root:{}", root.id),
            &input,
            true,
        )
        .unwrap();
        fs::remove_dir(workspace.join("blocked")).unwrap();
        fs::write(workspace.join("blocked"), b"unexpected").unwrap();
        let error = execute_adapter(&transaction, &manifest, &plan, false).unwrap_err();
        assert_eq!(error, DatabaseError::Cognitive("workspace_move_failed"));
        assert!(workspace.join("first.txt").is_file());
        assert!(!workspace.join("done.txt").exists());
        assert_eq!(fs::read(workspace.join("blocked")).unwrap(), b"unexpected");
        transaction.rollback().unwrap();
        cleanup(&path);
    }

    #[test]
    fn temporary_chat_safe_mode_and_untrusted_output_fail_closed() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        assert_eq!(
            database.create_tool_session(ToolSessionRequest {
                agent_id: ASTRA_ID.into(),
                scope_ref: "fixture:workspace/owner".into(),
                permissions: vec![ToolSessionPermission {
                    tool_id: "workspace.inspect_scope".into(),
                    permission: ToolPermission::Preview,
                }],
                idempotency_key: "temporary-session".into(),
                temporary_chat: true,
            }),
            Err(DatabaseError::Cognitive("tools_blocked_temporary"))
        );

        let session = database
            .create_tool_session(ToolSessionRequest {
                agent_id: ASTRA_ID.into(),
                scope_ref: "fixture:workspace/owner".into(),
                permissions: vec![
                    ToolSessionPermission {
                        tool_id: "workspace.inspect_scope".into(),
                        permission: ToolPermission::Preview,
                    },
                    ToolSessionPermission {
                        tool_id: "workspace.inspect_scope".into(),
                        permission: ToolPermission::ExecuteReadOnly,
                    },
                ],
                idempotency_key: "workspace-session".into(),
                temporary_chat: false,
            })
            .expect("workspace session should be created");
        assert_eq!(
            database.preview_tool_action(ToolActionPreviewRequest {
                agent_id: ASTRA_ID.into(),
                session_id: session.id,
                tool_id: "workspace.inspect_scope".into(),
                input: ToolActionInput::WorkspaceInspect {
                    relative_paths: vec!["inbox".into()],
                },
                dry_run: false,
                idempotency_key: "temporary-preview".into(),
                temporary_chat: true,
            }),
            Err(DatabaseError::Cognitive("tools_blocked_temporary"))
        );

        database
            .set_safe_mode(true)
            .expect("safe mode should enable");
        assert_eq!(
            database.create_tool_session(ToolSessionRequest {
                agent_id: ASTRA_ID.into(),
                scope_ref: "fixture:workspace/owner".into(),
                permissions: vec![ToolSessionPermission {
                    tool_id: "workspace.inspect_scope".into(),
                    permission: ToolPermission::Preview,
                }],
                idempotency_key: "safe-session".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("tools_blocked_safe_mode"))
        );
        cleanup(&path);
    }

    #[test]
    fn legacy_tool_capabilities_remain_readable_by_compatible_parser() {
        let path = test_path();
        let database = Database::initialize(&path).expect("database should initialize");
        let connection = database.open().expect("database should open");
        connection
            .execute(
                "UPDATE tool_catalog
                 SET capabilities_json = '{\"operations\":[\"inspect_scope\"]}'
                 WHERE tool_id = 'workspace.inspect_scope'",
                [],
            )
            .expect("legacy capability fixture should be stored");
        drop(connection);

        let manifest = database
            .list_tool_catalog()
            .expect("catalog should load")
            .into_iter()
            .find(|manifest| manifest.tool_id == "workspace.inspect_scope")
            .expect("workspace manifest should exist");
        assert_eq!(manifest.capabilities, vec!["inspect_scope"]);
        cleanup(&path);
    }
}
