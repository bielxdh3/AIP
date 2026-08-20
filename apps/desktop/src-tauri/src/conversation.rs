use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::{now_millis, Database, DatabaseError, OWNER_ID};

const MAX_PURPOSE_BYTES: usize = 160;
const MAX_REASON_BYTES: usize = 500;
const MAX_TURN_CONTENT_BYTES: usize = 4096;
const MAX_CANDIDATE_BYTES: usize = 8192;
const MIN_TURNS: i64 = 1;
const MAX_TURNS: i64 = 24;
const MIN_TOKENS: i64 = 64;
const MAX_TOKENS: i64 = 8192;
const MIN_DURATION_MS: i64 = 1_000;
const MAX_DURATION_MS: i64 = 900_000;
const MIN_REPETITIONS: i64 = 1;
const MAX_REPETITIONS: i64 = 3;
const MIN_RESOURCE_BUDGET: i64 = 1;
const MAX_RESOURCE_BUDGET: i64 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationPolicyRequest {
    pub agent_id: String,
    pub purpose: String,
    pub opted_in: bool,
    pub max_turns: i64,
    pub max_tokens: i64,
    pub max_duration_ms: i64,
    pub max_repetitions: i64,
    pub resource_budget: i64,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationPolicy {
    pub agent_id: String,
    pub purpose: String,
    pub opted_in: bool,
    pub max_turns: i64,
    pub max_tokens: i64,
    pub max_duration_ms: i64,
    pub max_repetitions: i64,
    pub resource_budget: i64,
    pub revoked_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationStartRequest {
    pub initiator_agent_id: String,
    pub participant_agent_id: String,
    pub purpose: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicConversationTurnRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub speaker_agent_id: String,
    pub content: String,
    pub source_kind: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationInterruptRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub reason: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveCandidateRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub candidate_kind: String,
    pub candidate_json: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveCandidateRejectionRequest {
    pub agent_id: String,
    pub candidate_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeavyGenerationRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub priority: i64,
    pub budget_units: i64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceJobCompletionRequest {
    pub agent_id: String,
    pub job_id: String,
    pub status: String,
    pub error_code: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationSummary {
    pub id: String,
    pub initiator_agent_id: String,
    pub participant_agent_id: String,
    pub purpose: String,
    pub status: String,
    pub max_turns: i64,
    pub max_tokens: i64,
    pub max_duration_ms: i64,
    pub max_repetitions: i64,
    pub resource_budget: i64,
    pub turn_count: i64,
    pub token_count: i64,
    pub loop_count: i64,
    pub termination_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicConversationTurn {
    pub id: String,
    pub conversation_id: String,
    pub speaker_agent_id: String,
    pub turn_index: i64,
    pub content: String,
    pub source_kind: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationInspection {
    pub conversation: AgentConversationSummary,
    pub turns: Vec<PublicConversationTurn>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveCandidate {
    pub id: String,
    pub conversation_id: String,
    pub agent_id: String,
    pub candidate_kind: String,
    pub candidate_json: String,
    pub source_reference: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveResourceJob {
    pub id: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub job_kind: String,
    pub heavy: bool,
    pub priority: i64,
    pub budget_units: i64,
    pub status: String,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConversationRecord {
    owner_user_id: String,
    summary: AgentConversationSummary,
}

impl Database {
    pub fn list_conversation_policies(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ConversationPolicy>, DatabaseError> {
        let connection = self.open()?;
        let owner_id = ensure_agent_owner(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT agent_id, purpose, opted_in, max_turns, max_tokens,
                    max_duration_ms, max_repetitions, resource_budget, revoked_at, updated_at
             FROM agent_conversation_policies
             WHERE agent_id = ?1 AND owner_user_id = ?2
             ORDER BY purpose ASC",
        )?;
        let policies = statement
            .query_map(params![agent_id, owner_id], map_policy)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(policies)
    }

    pub fn set_conversation_policy(
        &self,
        request: ConversationPolicyRequest,
    ) -> Result<ConversationPolicy, DatabaseError> {
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let purpose = purpose(&request.purpose)?;
        validate_policy_limits(
            request.max_turns,
            request.max_tokens,
            request.max_duration_ms,
            request.max_repetitions,
            request.resource_budget,
        )?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &request.agent_id)?;
        let now = now_millis();
        transaction.execute(
            "INSERT INTO agent_conversation_policies
             (agent_id, owner_user_id, purpose, opted_in, max_turns, max_tokens,
              max_duration_ms, max_repetitions, resource_budget, revoked_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                     CASE WHEN ?4 = 1 THEN NULL ELSE ?10 END, ?10)
             ON CONFLICT(agent_id, purpose) DO UPDATE SET
               owner_user_id = excluded.owner_user_id,
               opted_in = excluded.opted_in,
               max_turns = excluded.max_turns,
               max_tokens = excluded.max_tokens,
               max_duration_ms = excluded.max_duration_ms,
               max_repetitions = excluded.max_repetitions,
               resource_budget = excluded.resource_budget,
               revoked_at = excluded.revoked_at,
               updated_at = excluded.updated_at",
            params![
                request.agent_id,
                owner_id,
                purpose,
                request.opted_in,
                request.max_turns,
                request.max_tokens,
                request.max_duration_ms,
                request.max_repetitions,
                request.resource_budget,
                now,
            ],
        )?;
        let policy = transaction.query_row(
            "SELECT agent_id, purpose, opted_in, max_turns, max_tokens,
                        max_duration_ms, max_repetitions, resource_budget, revoked_at, updated_at
                 FROM agent_conversation_policies
                 WHERE agent_id = ?1 AND purpose = ?2",
            params![request.agent_id, purpose],
            map_policy,
        )?;
        transaction.commit()?;
        Ok(policy)
    }

    pub fn start_agent_conversation(
        &self,
        request: ConversationStartRequest,
    ) -> Result<AgentConversationSummary, DatabaseError> {
        self.start_agent_conversation_at(request, now_millis())
    }

    fn start_agent_conversation_at(
        &self,
        request: ConversationStartRequest,
        now: i64,
    ) -> Result<AgentConversationSummary, DatabaseError> {
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let initiator = agent_id(&request.initiator_agent_id)?;
        let participant = agent_id(&request.participant_agent_id)?;
        if initiator == participant {
            return Err(DatabaseError::Cognitive("conversation_participant_invalid"));
        }
        let purpose = purpose(&request.purpose)?;
        let key = idempotency(&request.idempotency_key)?;
        let conversation_id = derived_id("conversation", &key);
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let initiator_owner = ensure_agent_owner_tx(&transaction, &initiator)?;
        let participant_owner = ensure_agent_owner_tx(&transaction, &participant)?;
        if initiator_owner != participant_owner {
            return Err(DatabaseError::OwnershipMismatch);
        }

        if let Some(existing) = load_record_tx(&transaction, &conversation_id)? {
            if existing.owner_user_id != initiator_owner
                || existing.summary.initiator_agent_id != initiator
                || existing.summary.participant_agent_id != participant
                || existing.summary.purpose != purpose
            {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            transaction.commit()?;
            return Ok(existing.summary);
        }

        let initiator_policy = active_policy_tx(&transaction, &initiator, &purpose)?;
        let participant_policy = active_policy_tx(&transaction, &participant, &purpose)?;
        let (initiator_policy, participant_policy) = match (initiator_policy, participant_policy) {
            (Some(initiator), Some(participant)) => (initiator, participant),
            _ => {
                return Err(DatabaseError::Cognitive("conversation_opt_in_required"));
            }
        };
        check_modes_tx(&transaction, [&initiator, &participant])?;

        let summary = AgentConversationSummary {
            id: conversation_id,
            initiator_agent_id: initiator,
            participant_agent_id: participant,
            purpose,
            status: "active".into(),
            max_turns: initiator_policy.max_turns.min(participant_policy.max_turns),
            max_tokens: initiator_policy
                .max_tokens
                .min(participant_policy.max_tokens),
            max_duration_ms: initiator_policy
                .max_duration_ms
                .min(participant_policy.max_duration_ms),
            max_repetitions: initiator_policy
                .max_repetitions
                .min(participant_policy.max_repetitions),
            resource_budget: initiator_policy
                .resource_budget
                .min(participant_policy.resource_budget),
            turn_count: 0,
            token_count: 0,
            loop_count: 0,
            termination_reason: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };
        transaction.execute(
            "INSERT INTO agent_conversations
             (id, owner_user_id, initiator_agent_id, participant_agent_id, purpose, status,
              max_turns, max_tokens, max_duration_ms, max_repetitions, resource_budget,
              turn_count, token_count, loop_count, termination_reason, created_at, updated_at,
              completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?8, ?9, ?10, 0, 0, 0,
                     NULL, ?11, ?11, NULL)",
            params![
                summary.id,
                initiator_owner,
                summary.initiator_agent_id,
                summary.participant_agent_id,
                summary.purpose,
                summary.max_turns,
                summary.max_tokens,
                summary.max_duration_ms,
                summary.max_repetitions,
                summary.resource_budget,
                now,
            ],
        )?;
        transaction.commit()?;
        Ok(summary)
    }

    pub fn list_agent_conversations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentConversationSummary>, DatabaseError> {
        let connection = self.open()?;
        let owner_id = ensure_agent_owner(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, owner_user_id, initiator_agent_id, participant_agent_id, purpose, status,
                    max_turns, max_tokens, max_duration_ms, max_repetitions, resource_budget,
                    turn_count, token_count, loop_count, termination_reason, created_at,
                    updated_at, completed_at
             FROM agent_conversations
             WHERE owner_user_id = ?1 AND (initiator_agent_id = ?2 OR participant_agent_id = ?2)
             ORDER BY created_at DESC, id DESC",
        )?;
        let conversations = statement
            .query_map(params![owner_id, agent_id], map_summary)
            .map_err(DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(conversations)
    }

    pub fn inspect_agent_conversation(
        &self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Result<AgentConversationInspection, DatabaseError> {
        let connection = self.open()?;
        let owner_id = ensure_agent_owner(&connection, agent_id)?;
        let record = load_record(&connection, conversation_id)?
            .ok_or(DatabaseError::Cognitive("conversation_not_found"))?;
        if record.owner_user_id != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        let turns = load_turns(&connection, conversation_id)?;
        Ok(AgentConversationInspection {
            conversation: record.summary,
            turns,
        })
    }

    pub fn append_public_conversation_turn(
        &self,
        request: PublicConversationTurnRequest,
    ) -> Result<AgentConversationInspection, DatabaseError> {
        self.append_public_conversation_turn_at(request, now_millis())
    }

    fn append_public_conversation_turn_at(
        &self,
        request: PublicConversationTurnRequest,
        now: i64,
    ) -> Result<AgentConversationInspection, DatabaseError> {
        if request.temporary_chat || request.conversation_id.starts_with("temporary-") {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let request_agent_id = agent_id(&request.agent_id)?;
        let speaker_agent_id = agent_id(&request.speaker_agent_id)?;
        let content = public_content(&request.content)?;
        let source_kind = source_kind(&request.source_kind)?;
        let key = idempotency(&request.idempotency_key)?;
        let turn_id = derived_id("turn", &format!("{}:{}", request.conversation_id, key));
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &request_agent_id)?;
        let speaker_owner_id = ensure_agent_owner_tx(&transaction, &speaker_agent_id)?;
        if owner_id != speaker_owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        let record = load_record_tx(&transaction, &request.conversation_id)?
            .ok_or(DatabaseError::Cognitive("conversation_not_found"))?;
        if record.owner_user_id != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if let Some(existing) = load_turn_by_id_tx(&transaction, &turn_id)? {
            if existing.conversation_id != request.conversation_id
                || existing.speaker_agent_id != speaker_agent_id
                || existing.content != content
                || existing.source_kind != source_kind
            {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            transaction.commit()?;
            return self.inspect_agent_conversation(&request_agent_id, &request.conversation_id);
        }
        if record.summary.status != "active" {
            return Err(DatabaseError::Cognitive("conversation_not_active"));
        }
        if speaker_agent_id != record.summary.initiator_agent_id
            && speaker_agent_id != record.summary.participant_agent_id
        {
            return Err(DatabaseError::Cognitive("conversation_participant_invalid"));
        }
        check_modes_tx(
            &transaction,
            [
                &record.summary.initiator_agent_id,
                &record.summary.participant_agent_id,
            ],
        )?;
        if now.saturating_sub(record.summary.created_at) > record.summary.max_duration_ms {
            terminate_conversation_tx(
                &transaction,
                &record.summary.id,
                "completed",
                "duration_exhausted",
                now,
            )?;
            transaction.commit()?;
            return Err(DatabaseError::Cognitive("conversation_duration_limit"));
        }
        if record.summary.turn_count >= record.summary.max_turns {
            terminate_conversation_tx(
                &transaction,
                &record.summary.id,
                "completed",
                "turn_budget_exhausted",
                now,
            )?;
            transaction.commit()?;
            return Err(DatabaseError::Cognitive("conversation_turn_limit"));
        }
        let token_count = estimate_tokens(&content);
        if record.summary.token_count + token_count > record.summary.max_tokens {
            terminate_conversation_tx(
                &transaction,
                &record.summary.id,
                "completed",
                "token_budget_exhausted",
                now,
            )?;
            transaction.commit()?;
            return Err(DatabaseError::Cognitive("conversation_token_limit"));
        }
        let previous_content = transaction
            .query_row(
                "SELECT content FROM agent_conversation_turns
                 WHERE conversation_id = ?1 ORDER BY turn_index DESC LIMIT 1",
                params![record.summary.id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let loop_count = previous_content
            .as_deref()
            .is_some_and(|previous| normalize_turn(previous) == normalize_turn(&content))
            .then_some(record.summary.loop_count + 1)
            .unwrap_or(0);
        let is_repetition = loop_count > 0;
        transaction.execute(
            "INSERT INTO agent_conversation_turns
             (id, conversation_id, speaker_agent_id, turn_index, content, source_kind, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                turn_id,
                record.summary.id,
                speaker_agent_id,
                record.summary.turn_count,
                content,
                source_kind,
                now,
            ],
        )?;
        let turn_count = record.summary.turn_count + 1;
        let terminal = if is_repetition && loop_count + 1 >= record.summary.max_repetitions {
            Some("loop_detected")
        } else if turn_count >= record.summary.max_turns {
            Some("turn_budget_exhausted")
        } else if record.summary.token_count + token_count >= record.summary.max_tokens {
            Some("token_budget_exhausted")
        } else {
            None
        };
        if let Some(reason) = terminal {
            terminate_conversation_tx(&transaction, &record.summary.id, "completed", reason, now)?;
        }
        transaction.execute(
            "UPDATE agent_conversations
             SET turn_count = ?1, token_count = ?2, loop_count = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                turn_count,
                record.summary.token_count + token_count,
                loop_count,
                now,
                record.summary.id,
            ],
        )?;
        transaction.commit()?;
        self.inspect_agent_conversation(&request_agent_id, &request.conversation_id)
    }

    pub fn interrupt_agent_conversation(
        &self,
        request: ConversationInterruptRequest,
    ) -> Result<AgentConversationSummary, DatabaseError> {
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let agent_id = agent_id(&request.agent_id)?;
        let _reason = reason(&request.reason)?;
        let _key = idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &agent_id)?;
        let record = load_record_tx(&transaction, &request.conversation_id)?
            .ok_or(DatabaseError::Cognitive("conversation_not_found"))?;
        if record.owner_user_id != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if record.summary.status == "active" || record.summary.status == "suspended" {
            terminate_conversation_tx(
                &transaction,
                &request.conversation_id,
                "cancelled",
                "owner_interrupted",
                now_millis(),
            )?;
        }
        let summary = load_record_tx(&transaction, &request.conversation_id)
            .map_err(|_| DatabaseError::Cognitive("conversation_not_found"))?
            .ok_or(DatabaseError::Cognitive("conversation_not_found"))?
            .summary;
        transaction.commit()?;
        Ok(summary)
    }

    pub fn emit_cognitive_candidate(
        &self,
        request: CognitiveCandidateRequest,
    ) -> Result<CognitiveCandidate, DatabaseError> {
        let agent_id = agent_id(&request.agent_id)?;
        let candidate_kind = candidate_kind(&request.candidate_kind)?;
        let candidate_json = public_candidate_json(&request.candidate_json)?;
        let key = idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &agent_id)?;
        let record = load_record_tx(&transaction, &request.conversation_id)?
            .ok_or(DatabaseError::Cognitive("conversation_not_found"))?;
        if record.owner_user_id != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if record.summary.status != "completed" {
            return Err(DatabaseError::Cognitive("conversation_not_completed"));
        }
        if agent_id != record.summary.initiator_agent_id
            && agent_id != record.summary.participant_agent_id
        {
            return Err(DatabaseError::Cognitive("conversation_participant_invalid"));
        }
        let candidate_id = derived_id(
            "candidate",
            &format!(
                "{}:{}:{}:{}",
                request.conversation_id, agent_id, candidate_kind, key
            ),
        );
        if let Some(existing) = load_candidate_by_id_tx(&transaction, &candidate_id)? {
            if existing.agent_id != agent_id
                || existing.candidate_kind != candidate_kind
                || existing.candidate_json != candidate_json
            {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            transaction.commit()?;
            return Ok(existing);
        }
        let candidate = CognitiveCandidate {
            id: candidate_id,
            conversation_id: request.conversation_id,
            agent_id,
            candidate_kind,
            candidate_json,
            source_reference: format!("conversation:{}", record.summary.id),
            status: "pending".into(),
            created_at: now_millis(),
        };
        transaction.execute(
            "INSERT INTO cognitive_candidates
             (id, conversation_id, agent_id, owner_user_id, candidate_kind, candidate_json,
              source_reference, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
            params![
                candidate.id,
                candidate.conversation_id,
                candidate.agent_id,
                owner_id,
                candidate.candidate_kind,
                candidate.candidate_json,
                candidate.source_reference,
                candidate.created_at,
            ],
        )?;
        transaction.commit()?;
        Ok(candidate)
    }

    pub fn list_cognitive_candidates(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CognitiveCandidate>, DatabaseError> {
        let connection = self.open()?;
        let owner_id = ensure_agent_owner(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, conversation_id, agent_id, candidate_kind, candidate_json,
                    source_reference, status, created_at
             FROM cognitive_candidates
             WHERE owner_user_id = ?1 AND agent_id = ?2
             ORDER BY created_at DESC, id DESC",
        )?;
        let candidates = statement
            .query_map(params![owner_id, agent_id], map_candidate)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(candidates)
    }

    pub fn reject_cognitive_candidate(
        &self,
        request: CognitiveCandidateRejectionRequest,
    ) -> Result<CognitiveCandidate, DatabaseError> {
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let agent_id = agent_id(&request.agent_id)?;
        let _key = idempotency(&request.idempotency_key)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &agent_id)?;
        let candidate = load_candidate_by_id_tx(&transaction, &request.candidate_id)?
            .ok_or(DatabaseError::Cognitive("candidate_not_found"))?;
        let candidate_owner: String = transaction.query_row(
            "SELECT owner_user_id FROM cognitive_candidates WHERE id = ?1",
            params![request.candidate_id],
            |row| row.get(0),
        )?;
        if candidate_owner != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if candidate.status == "applied" {
            return Err(DatabaseError::Cognitive("candidate_already_decided"));
        }
        if candidate.status == "pending" {
            transaction.execute(
                "UPDATE cognitive_candidates SET status = 'rejected' WHERE id = ?1",
                params![request.candidate_id],
            )?;
        }
        let result = load_candidate_by_id_tx(&transaction, &request.candidate_id)?
            .ok_or(DatabaseError::Cognitive("candidate_not_found"))?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn reserve_heavy_generation(
        &self,
        request: HeavyGenerationRequest,
    ) -> Result<CognitiveResourceJob, DatabaseError> {
        let agent_id = agent_id(&request.agent_id)?;
        let key = idempotency(&request.idempotency_key)?;
        if !(0..=100).contains(&request.priority)
            || !(MIN_RESOURCE_BUDGET..=MAX_RESOURCE_BUDGET).contains(&request.budget_units)
        {
            return Err(DatabaseError::Cognitive("conversation_budget_invalid"));
        }
        let job_id = derived_id("resource-job", &key);
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &agent_id)?;
        let record = load_record_tx(&transaction, &request.conversation_id)?
            .ok_or(DatabaseError::Cognitive("conversation_not_found"))?;
        if record.owner_user_id != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if record.summary.status != "active" {
            return Err(DatabaseError::Cognitive("conversation_not_active"));
        }
        if request.budget_units > record.summary.resource_budget {
            return Err(DatabaseError::Cognitive("conversation_budget_invalid"));
        }
        check_modes_tx(
            &transaction,
            [
                &record.summary.initiator_agent_id,
                &record.summary.participant_agent_id,
            ],
        )?;
        if let Some(existing) = load_job_by_id_tx(&transaction, &job_id)? {
            if existing.agent_id != agent_id
                || existing.conversation_id.as_deref() != Some(request.conversation_id.as_str())
                || existing.priority != request.priority
                || existing.budget_units != request.budget_units
            {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            transaction.commit()?;
            return Ok(existing);
        }
        let heavy_running: bool = transaction.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM cognitive_resource_jobs WHERE heavy = 1 AND status = 'running'
             )",
            [],
            |row| row.get(0),
        )?;
        if heavy_running {
            return Err(DatabaseError::Cognitive("heavy_generation_busy"));
        }
        let now = now_millis();
        let result = transaction.execute(
            "INSERT INTO cognitive_resource_jobs
             (id, owner_user_id, agent_id, conversation_id, job_kind, heavy, priority,
              budget_units, status, error_code, created_at, started_at, ended_at)
             VALUES (?1, ?2, ?3, ?4, 'heavy_generation', 1, ?5, ?6, 'running', NULL, ?7, ?7, NULL)",
            params![
                job_id,
                owner_id,
                agent_id,
                request.conversation_id,
                request.priority,
                request.budget_units,
                now,
            ],
        );
        if let Err(error) = result {
            if is_constraint(&error) {
                return Err(DatabaseError::Cognitive("heavy_generation_busy"));
            }
            return Err(DatabaseError::from(error));
        }
        let job = transaction.query_row(
            "SELECT id, agent_id, conversation_id, job_kind, heavy, priority,
                        budget_units, status, error_code, created_at, started_at, ended_at
                 FROM cognitive_resource_jobs WHERE id = ?1",
            params![job_id],
            map_job,
        )?;
        transaction.commit()?;
        Ok(job)
    }

    pub fn complete_resource_job(
        &self,
        request: ResourceJobCompletionRequest,
    ) -> Result<CognitiveResourceJob, DatabaseError> {
        let agent_id = agent_id(&request.agent_id)?;
        let _key = idempotency(&request.idempotency_key)?;
        if !matches!(
            request.status.as_str(),
            "completed" | "cancelled" | "failed"
        ) {
            return Err(DatabaseError::Cognitive("invalid_resource_status"));
        }
        let error_code = request
            .error_code
            .as_deref()
            .map(|value| bounded(value, 80, "invalid_resource_status"))
            .transpose()?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_agent_owner_tx(&transaction, &agent_id)?;
        let existing = load_job_by_id_tx(&transaction, &request.job_id)?
            .ok_or(DatabaseError::Cognitive("resource_job_not_found"))?;
        let job_owner: String = transaction.query_row(
            "SELECT owner_user_id FROM cognitive_resource_jobs WHERE id = ?1",
            params![request.job_id],
            |row| row.get(0),
        )?;
        if job_owner != owner_id {
            return Err(DatabaseError::OwnershipMismatch);
        }
        if matches!(
            existing.status.as_str(),
            "completed" | "cancelled" | "failed"
        ) {
            transaction.commit()?;
            return Ok(existing);
        }
        transaction.execute(
            "UPDATE cognitive_resource_jobs
             SET status = ?1, error_code = ?2, ended_at = ?3
             WHERE id = ?4",
            params![request.status, error_code, now_millis(), request.job_id],
        )?;
        let job = load_job_by_id_tx(&transaction, &request.job_id)?
            .ok_or(DatabaseError::Cognitive("resource_job_not_found"))?;
        transaction.commit()?;
        Ok(job)
    }
}

fn ensure_agent_owner(connection: &Connection, agent_id: &str) -> Result<String, DatabaseError> {
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

fn ensure_agent_owner_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
) -> Result<String, DatabaseError> {
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

fn check_modes_tx<const N: usize>(
    transaction: &Transaction<'_>,
    agent_ids: [&str; N],
) -> Result<(), DatabaseError> {
    let safe_mode: bool = transaction
        .query_row(
            "SELECT COALESCE((SELECT value_json = 'true' FROM app_settings WHERE key = 'safe_mode'), 0)",
            [],
            |row| row.get(0),
        )?;
    if safe_mode {
        return Err(DatabaseError::Cognitive("conversation_blocked_safe_mode"));
    }
    for agent_id in agent_ids {
        let state = transaction
            .query_row(
                "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id = ?1",
                params![agent_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
            )
            .optional()?
            .ok_or(DatabaseError::Cognitive("agent_not_found"))?;
        if state.1 {
            return Err(DatabaseError::Cognitive("conversation_blocked_suspended"));
        }
        if state.0 == "silent" {
            return Err(DatabaseError::Cognitive("conversation_blocked_silent"));
        }
    }
    Ok(())
}

fn active_policy_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    purpose: &str,
) -> Result<Option<ConversationPolicy>, DatabaseError> {
    transaction
        .query_row(
            "SELECT agent_id, purpose, opted_in, max_turns, max_tokens,
                    max_duration_ms, max_repetitions, resource_budget, revoked_at, updated_at
             FROM agent_conversation_policies
             WHERE agent_id = ?1 AND purpose = ?2 AND owner_user_id = ?3",
            params![agent_id, purpose, OWNER_ID],
            map_policy,
        )
        .optional()
        .map(|policy| policy.filter(|policy| policy.opted_in && policy.revoked_at.is_none()))
        .map_err(DatabaseError::from)
}

fn load_record(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Option<ConversationRecord>, DatabaseError> {
    connection
        .query_row(
            "SELECT id, owner_user_id, initiator_agent_id, participant_agent_id, purpose, status,
                    max_turns, max_tokens, max_duration_ms, max_repetitions, resource_budget,
                    turn_count, token_count, loop_count, termination_reason, created_at,
                    updated_at, completed_at
             FROM agent_conversations WHERE id = ?1",
            params![conversation_id],
            map_record,
        )
        .optional()
        .map_err(DatabaseError::from)
}

fn load_record_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
) -> Result<Option<ConversationRecord>, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, owner_user_id, initiator_agent_id, participant_agent_id, purpose, status,
                    max_turns, max_tokens, max_duration_ms, max_repetitions, resource_budget,
                    turn_count, token_count, loop_count, termination_reason, created_at,
                    updated_at, completed_at
             FROM agent_conversations WHERE id = ?1",
            params![conversation_id],
            map_record,
        )
        .optional()
        .map_err(DatabaseError::from)
}

fn load_turns(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<PublicConversationTurn>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT id, conversation_id, speaker_agent_id, turn_index, content, source_kind, created_at
         FROM agent_conversation_turns WHERE conversation_id = ?1 ORDER BY turn_index ASC",
    )?;
    let turns = statement
        .query_map(params![conversation_id], map_turn)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DatabaseError::from)?;
    Ok(turns)
}

fn load_turn_by_id_tx(
    transaction: &Transaction<'_>,
    turn_id: &str,
) -> Result<Option<PublicConversationTurn>, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, conversation_id, speaker_agent_id, turn_index, content, source_kind, created_at
             FROM agent_conversation_turns WHERE id = ?1",
            params![turn_id],
            map_turn,
        )
        .optional()
        .map_err(DatabaseError::from)
}

fn load_candidate_by_id_tx(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> Result<Option<CognitiveCandidate>, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, conversation_id, agent_id, candidate_kind, candidate_json,
                    source_reference, status, created_at
             FROM cognitive_candidates WHERE id = ?1",
            params![candidate_id],
            map_candidate,
        )
        .optional()
        .map_err(DatabaseError::from)
}

fn load_job_by_id_tx(
    transaction: &Transaction<'_>,
    job_id: &str,
) -> Result<Option<CognitiveResourceJob>, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, agent_id, conversation_id, job_kind, heavy, priority,
                    budget_units, status, error_code, created_at, started_at, ended_at
             FROM cognitive_resource_jobs WHERE id = ?1",
            params![job_id],
            map_job,
        )
        .optional()
        .map_err(DatabaseError::from)
}

fn terminate_conversation_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    status: &str,
    reason: &str,
    now: i64,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "UPDATE agent_conversations
         SET status = ?1, termination_reason = ?2, updated_at = ?3, completed_at = ?3
         WHERE id = ?4 AND status IN ('active', 'suspended')",
        params![status, reason, now, conversation_id],
    )?;
    if status != "active" {
        transaction.execute(
            "UPDATE cognitive_resource_jobs
             SET status = 'cancelled', error_code = CASE WHEN ?1 = 'cancelled' THEN 'owner_interrupted' ELSE error_code END, ended_at = ?2
             WHERE conversation_id = ?3 AND status IN ('queued', 'running')",
            params![status, now, conversation_id],
        )?;
    }
    Ok(())
}

fn map_policy(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationPolicy> {
    Ok(ConversationPolicy {
        agent_id: row.get(0)?,
        purpose: row.get(1)?,
        opted_in: row.get(2)?,
        max_turns: row.get(3)?,
        max_tokens: row.get(4)?,
        max_duration_ms: row.get(5)?,
        max_repetitions: row.get(6)?,
        resource_budget: row.get(7)?,
        revoked_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationRecord> {
    Ok(ConversationRecord {
        owner_user_id: row.get(1)?,
        summary: AgentConversationSummary {
            id: row.get(0)?,
            initiator_agent_id: row.get(2)?,
            participant_agent_id: row.get(3)?,
            purpose: row.get(4)?,
            status: row.get(5)?,
            max_turns: row.get(6)?,
            max_tokens: row.get(7)?,
            max_duration_ms: row.get(8)?,
            max_repetitions: row.get(9)?,
            resource_budget: row.get(10)?,
            turn_count: row.get(11)?,
            token_count: row.get(12)?,
            loop_count: row.get(13)?,
            termination_reason: row.get(14)?,
            created_at: row.get(15)?,
            updated_at: row.get(16)?,
            completed_at: row.get(17)?,
        },
    })
}

fn map_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentConversationSummary> {
    Ok(map_record(row)?.summary)
}

fn map_turn(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublicConversationTurn> {
    Ok(PublicConversationTurn {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        speaker_agent_id: row.get(2)?,
        turn_index: row.get(3)?,
        content: row.get(4)?,
        source_kind: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn map_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<CognitiveCandidate> {
    Ok(CognitiveCandidate {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        agent_id: row.get(2)?,
        candidate_kind: row.get(3)?,
        candidate_json: row.get(4)?,
        source_reference: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn map_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<CognitiveResourceJob> {
    Ok(CognitiveResourceJob {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        conversation_id: row.get(2)?,
        job_kind: row.get(3)?,
        heavy: row.get(4)?,
        priority: row.get(5)?,
        budget_units: row.get(6)?,
        status: row.get(7)?,
        error_code: row.get(8)?,
        created_at: row.get(9)?,
        started_at: row.get(10)?,
        ended_at: row.get(11)?,
    })
}

fn validate_policy_limits(
    max_turns: i64,
    max_tokens: i64,
    max_duration_ms: i64,
    max_repetitions: i64,
    resource_budget: i64,
) -> Result<(), DatabaseError> {
    if !(MIN_TURNS..=MAX_TURNS).contains(&max_turns)
        || !(MIN_TOKENS..=MAX_TOKENS).contains(&max_tokens)
        || !(MIN_DURATION_MS..=MAX_DURATION_MS).contains(&max_duration_ms)
        || !(MIN_REPETITIONS..=MAX_REPETITIONS).contains(&max_repetitions)
        || !(MIN_RESOURCE_BUDGET..=MAX_RESOURCE_BUDGET).contains(&resource_budget)
    {
        return Err(DatabaseError::Cognitive("conversation_budget_invalid"));
    }
    Ok(())
}

fn bounded(value: &str, max: usize, code: &'static str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > max
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(value.to_owned())
}

fn agent_id(value: &str) -> Result<String, DatabaseError> {
    let value = bounded(value, 128, "conversation_participant_invalid")?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(DatabaseError::Cognitive("conversation_participant_invalid"));
    }
    Ok(value)
}

fn purpose(value: &str) -> Result<String, DatabaseError> {
    bounded(value, MAX_PURPOSE_BYTES, "conversation_purpose_invalid")
}

fn reason(value: &str) -> Result<String, DatabaseError> {
    bounded(value, MAX_REASON_BYTES, "invalid_reason")
}

fn public_content(value: &str) -> Result<String, DatabaseError> {
    bounded(value, MAX_TURN_CONTENT_BYTES, "conversation_turn_invalid")
}

fn source_kind(value: &str) -> Result<String, DatabaseError> {
    match value.trim() {
        "owner" | "model_candidate" => Ok(value.trim().to_owned()),
        _ => Err(DatabaseError::Cognitive("conversation_turn_invalid")),
    }
}

fn candidate_kind(value: &str) -> Result<String, DatabaseError> {
    match value.trim() {
        "opinion" | "relationship" | "goal" => Ok(value.trim().to_owned()),
        _ => Err(DatabaseError::Cognitive("conversation_candidate_invalid")),
    }
}

fn public_candidate_json(value: &str) -> Result<String, DatabaseError> {
    if value.trim().len() > MAX_CANDIDATE_BYTES {
        return Err(DatabaseError::Cognitive("conversation_candidate_invalid"));
    }
    let parsed: Value = serde_json::from_str(value)
        .map_err(|_| DatabaseError::Cognitive("conversation_candidate_invalid"))?;
    if !parsed.is_object() || contains_private_key(&parsed) {
        return Err(DatabaseError::Cognitive("conversation_candidate_invalid"));
    }
    let canonical = serde_json::to_string(&parsed)
        .map_err(|_| DatabaseError::Cognitive("conversation_candidate_invalid"))?;
    if canonical.len() > MAX_CANDIDATE_BYTES {
        return Err(DatabaseError::Cognitive("conversation_candidate_invalid"));
    }
    Ok(canonical)
}

fn contains_private_key(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let normalized = key
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .flat_map(char::to_lowercase)
                .collect::<String>();
            matches!(
                normalized.as_str(),
                "prompt"
                    | "systemprompt"
                    | "hiddenreasoning"
                    | "chainofthought"
                    | "privateconversation"
                    | "privatemessage"
                    | "secret"
            ) || contains_private_key(value)
        }),
        Value::Array(values) => values.iter().any(contains_private_key),
        _ => false,
    }
}

fn idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = bounded(value, 128, "invalid_idempotency_key")?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':'))
    {
        return Err(DatabaseError::Cognitive("invalid_idempotency_key"));
    }
    Ok(value)
}

fn derived_id(prefix: &str, key: &str) -> String {
    format!("{prefix}:{key}")
}

fn estimate_tokens(content: &str) -> i64 {
    ((content.len() as i64 + 3) / 4).max(1)
}

fn normalize_turn(content: &str) -> String {
    content
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_constraint(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                ErrorCode::ConstraintViolation
            )
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;
    use uuid::Uuid;

    use super::*;
    use crate::database::{ASTRA_ID, LUMA_ID};

    fn test_path() -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("aip-conversation-test-{}", Uuid::now_v7()))
            .join("aip.sqlite3")
    }

    fn cleanup(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    fn policy(agent_id: &str, purpose: &str) -> ConversationPolicyRequest {
        ConversationPolicyRequest {
            agent_id: agent_id.into(),
            purpose: purpose.into(),
            opted_in: true,
            max_turns: 4,
            max_tokens: 64,
            max_duration_ms: 300_000,
            max_repetitions: 2,
            resource_budget: 20,
            temporary_chat: false,
        }
    }

    fn start(purpose: &str, key: &str) -> ConversationStartRequest {
        ConversationStartRequest {
            initiator_agent_id: ASTRA_ID.into(),
            participant_agent_id: LUMA_ID.into(),
            purpose: purpose.into(),
            idempotency_key: key.into(),
            temporary_chat: false,
        }
    }

    fn turn(
        conversation_id: &str,
        speaker: &str,
        content: &str,
        key: &str,
    ) -> PublicConversationTurnRequest {
        PublicConversationTurnRequest {
            agent_id: ASTRA_ID.into(),
            conversation_id: conversation_id.into(),
            speaker_agent_id: speaker.into(),
            content: content.into(),
            source_kind: "model_candidate".into(),
            idempotency_key: key.into(),
            temporary_chat: false,
        }
    }

    fn prepare(database: &Database, purpose: &str) {
        database
            .set_conversation_policy(policy(ASTRA_ID, purpose))
            .unwrap();
        database
            .set_conversation_policy(policy(LUMA_ID, purpose))
            .unwrap();
    }

    #[test]
    fn requires_both_opt_ins_and_rejects_cross_owner_lineage() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        assert_eq!(
            database.start_agent_conversation(start("shared-purpose", "missing-opt-in")),
            Err(DatabaseError::Cognitive("conversation_opt_in_required"))
        );
        database
            .set_conversation_policy(policy(ASTRA_ID, "shared-purpose"))
            .unwrap();
        assert_eq!(
            database.start_agent_conversation(start("shared-purpose", "one-opt-in")),
            Err(DatabaseError::Cognitive("conversation_opt_in_required"))
        );
        database
            .set_conversation_policy(policy(LUMA_ID, "shared-purpose"))
            .unwrap();
        let started = database
            .start_agent_conversation(start("shared-purpose", "same-owner"))
            .unwrap();
        assert_eq!(started.status, "active");

        let connection = database.open().unwrap();
        connection
            .execute(
                "INSERT INTO users (id, role, display_name, created_at, updated_at)
                 VALUES ('usr_other', 'owner', 'Other', 1, 1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE agents SET owner_user_id = 'usr_other' WHERE id = ?1",
                params![LUMA_ID],
            )
            .unwrap();
        assert_eq!(
            database.start_agent_conversation(start("shared-purpose", "cross-owner")),
            Err(DatabaseError::OwnershipMismatch)
        );
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn temporary_safe_silent_and_suspended_modes_block_start() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        prepare(&database, "mode-purpose");
        let mut temporary = start("mode-purpose", "temporary");
        temporary.temporary_chat = true;
        assert_eq!(
            database.start_agent_conversation(temporary),
            Err(DatabaseError::Cognitive("conversation_temporary_blocked"))
        );

        database.set_safe_mode(true).unwrap();
        assert_eq!(
            database.start_agent_conversation(start("mode-purpose", "safe")),
            Err(DatabaseError::Cognitive("conversation_blocked_safe_mode"))
        );
        database.set_safe_mode(false).unwrap();
        database.set_agent_mode(ASTRA_ID, "silent").unwrap();
        assert_eq!(
            database.start_agent_conversation(start("mode-purpose", "silent")),
            Err(DatabaseError::Cognitive("conversation_blocked_silent"))
        );
        database.set_agent_mode(ASTRA_ID, "normal").unwrap();
        database.set_agent_suspended(ASTRA_ID, true).unwrap();
        assert_eq!(
            database.start_agent_conversation(start("mode-purpose", "suspended")),
            Err(DatabaseError::Cognitive("conversation_blocked_suspended"))
        );
        cleanup(&path);
    }

    #[test]
    fn bounded_turns_tokens_duration_and_loop_termination_are_terminal() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut bounded_policy = policy(ASTRA_ID, "bounded-purpose");
        bounded_policy.max_turns = 3;
        bounded_policy.max_tokens = 64;
        bounded_policy.max_repetitions = 2;
        database.set_conversation_policy(bounded_policy).unwrap();
        database
            .set_conversation_policy(policy(LUMA_ID, "bounded-purpose"))
            .unwrap();
        let conversation = database
            .start_agent_conversation(start("bounded-purpose", "bounded"))
            .unwrap();
        database
            .append_public_conversation_turn(turn(
                &conversation.id,
                LUMA_ID,
                "Public bounded turn",
                "turn-1",
            ))
            .unwrap();
        let looped = database
            .append_public_conversation_turn(turn(
                &conversation.id,
                ASTRA_ID,
                "Public bounded turn",
                "turn-2",
            ))
            .unwrap();
        assert_eq!(looped.conversation.status, "completed");
        assert_eq!(
            looped.conversation.termination_reason.as_deref(),
            Some("loop_detected")
        );
        assert_eq!(
            database.append_public_conversation_turn(turn(
                &conversation.id,
                LUMA_ID,
                "after terminal",
                "turn-3",
            )),
            Err(DatabaseError::Cognitive("conversation_not_active"))
        );

        let mut token_policy = policy(ASTRA_ID, "token-purpose");
        token_policy.max_tokens = 64;
        database.set_conversation_policy(token_policy).unwrap();
        database
            .set_conversation_policy(policy(LUMA_ID, "token-purpose"))
            .unwrap();
        let token_conversation = database
            .start_agent_conversation(start("token-purpose", "token"))
            .unwrap();
        let oversized = "x".repeat(300);
        assert_eq!(
            database.append_public_conversation_turn(turn(
                &token_conversation.id,
                ASTRA_ID,
                &oversized,
                "token-turn",
            )),
            Err(DatabaseError::Cognitive("conversation_token_limit"))
        );
        assert_eq!(
            database
                .inspect_agent_conversation(ASTRA_ID, &token_conversation.id)
                .unwrap()
                .conversation
                .status,
            "completed"
        );

        let mut duration_policy = policy(ASTRA_ID, "duration-purpose");
        duration_policy.max_duration_ms = 1_000;
        database.set_conversation_policy(duration_policy).unwrap();
        database
            .set_conversation_policy(policy(LUMA_ID, "duration-purpose"))
            .unwrap();
        let duration_conversation = database
            .start_agent_conversation(start("duration-purpose", "duration"))
            .unwrap();
        let connection = database.open().unwrap();
        connection
            .execute(
                "UPDATE agent_conversations SET created_at = 0 WHERE id = ?1",
                params![duration_conversation.id],
            )
            .unwrap();
        assert_eq!(
            database.append_public_conversation_turn(turn(
                &duration_conversation.id,
                ASTRA_ID,
                "late",
                "duration-turn",
            )),
            Err(DatabaseError::Cognitive("conversation_duration_limit"))
        );
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn owner_interrupt_is_explicit_and_replay_safe() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        prepare(&database, "interrupt-purpose");
        let conversation = database
            .start_agent_conversation(start("interrupt-purpose", "interrupt"))
            .unwrap();
        let request = ConversationInterruptRequest {
            agent_id: ASTRA_ID.into(),
            conversation_id: conversation.id.clone(),
            reason: "Owner interrompeu a conversa pública".into(),
            idempotency_key: "interrupt-key".into(),
            temporary_chat: false,
        };
        let cancelled = database
            .interrupt_agent_conversation(request.clone())
            .unwrap();
        assert_eq!(cancelled.status, "cancelled");
        assert_eq!(
            cancelled.termination_reason.as_deref(),
            Some("owner_interrupted")
        );
        assert_eq!(
            database.interrupt_agent_conversation(request).unwrap(),
            cancelled
        );
        cleanup(&path);
    }

    #[test]
    fn only_one_heavy_generation_can_run() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        prepare(&database, "resource-purpose");
        let conversation = database
            .start_agent_conversation(start("resource-purpose", "resource"))
            .unwrap();
        let first = database
            .reserve_heavy_generation(HeavyGenerationRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: conversation.id.clone(),
                priority: 50,
                budget_units: 10,
                idempotency_key: "heavy-1".into(),
            })
            .unwrap();
        assert_eq!(first.status, "running");
        assert_eq!(
            database.reserve_heavy_generation(HeavyGenerationRequest {
                agent_id: LUMA_ID.into(),
                conversation_id: conversation.id.clone(),
                priority: 50,
                budget_units: 10,
                idempotency_key: "heavy-2".into(),
            }),
            Err(DatabaseError::Cognitive("heavy_generation_busy"))
        );
        let finished = database
            .complete_resource_job(ResourceJobCompletionRequest {
                agent_id: ASTRA_ID.into(),
                job_id: first.id,
                status: "completed".into(),
                error_code: None,
                idempotency_key: "finish-1".into(),
            })
            .unwrap();
        assert_eq!(finished.status, "completed");
        assert_eq!(
            database
                .reserve_heavy_generation(HeavyGenerationRequest {
                    agent_id: LUMA_ID.into(),
                    conversation_id: conversation.id,
                    priority: 40,
                    budget_units: 5,
                    idempotency_key: "heavy-2".into(),
                })
                .unwrap()
                .status,
            "running"
        );
        cleanup(&path);
    }

    #[test]
    fn restart_persists_public_state_and_recovers_running_work() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        prepare(&database, "restart-purpose");
        let conversation = database
            .start_agent_conversation(start("restart-purpose", "restart"))
            .unwrap();
        database
            .append_public_conversation_turn(turn(
                &conversation.id,
                ASTRA_ID,
                "Public persisted turn",
                "restart-turn",
            ))
            .unwrap();
        let job = database
            .reserve_heavy_generation(HeavyGenerationRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: conversation.id.clone(),
                priority: 10,
                budget_units: 2,
                idempotency_key: "restart-job".into(),
            })
            .unwrap();
        drop(database);
        let reopened = Database::initialize(&path).unwrap();
        let inspection = reopened
            .inspect_agent_conversation(ASTRA_ID, &conversation.id)
            .unwrap();
        assert_eq!(inspection.conversation.status, "suspended");
        assert_eq!(inspection.turns.len(), 1);
        let connection = reopened.open().unwrap();
        let job_status: String = connection
            .query_row(
                "SELECT status FROM cognitive_resource_jobs WHERE id = ?1",
                params![job.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(job_status, "failed");
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn candidates_are_pending_attribution_and_never_apply_directly() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut one_turn = policy(ASTRA_ID, "candidate-purpose");
        one_turn.max_turns = 1;
        database.set_conversation_policy(one_turn).unwrap();
        database
            .set_conversation_policy(policy(LUMA_ID, "candidate-purpose"))
            .unwrap();
        let conversation = database
            .start_agent_conversation(start("candidate-purpose", "candidate"))
            .unwrap();
        let completed = database
            .append_public_conversation_turn(turn(
                &conversation.id,
                ASTRA_ID,
                "Public candidate source",
                "candidate-turn",
            ))
            .unwrap();
        assert_eq!(completed.conversation.status, "completed");
        let candidate = database
            .emit_cognitive_candidate(CognitiveCandidateRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: conversation.id.clone(),
                candidate_kind: "opinion".into(),
                candidate_json: r#"{"subject":"fictional-topic","stance":0.2}"#.into(),
                idempotency_key: "candidate-1".into(),
            })
            .unwrap();
        assert_eq!(candidate.status, "pending");
        let connection = database.open().unwrap();
        let opinion_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM opinions", [], |row| row.get(0))
            .unwrap();
        drop(connection);
        assert_eq!(opinion_count, 0);
        assert_eq!(
            database.list_cognitive_candidates(ASTRA_ID).unwrap().len(),
            1
        );
        let rejected = database
            .reject_cognitive_candidate(CognitiveCandidateRejectionRequest {
                agent_id: ASTRA_ID.into(),
                candidate_id: candidate.id,
                idempotency_key: "candidate-reject".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(rejected.status, "rejected");
        cleanup(&path);
    }

    #[test]
    fn private_candidate_fields_are_rejected() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut one_turn = policy(ASTRA_ID, "private-purpose");
        one_turn.max_turns = 1;
        database.set_conversation_policy(one_turn).unwrap();
        database
            .set_conversation_policy(policy(LUMA_ID, "private-purpose"))
            .unwrap();
        let conversation = database
            .start_agent_conversation(start("private-purpose", "private"))
            .unwrap();
        database
            .append_public_conversation_turn(turn(
                &conversation.id,
                ASTRA_ID,
                "Public source",
                "private-turn",
            ))
            .unwrap();
        assert_eq!(
            database.emit_cognitive_candidate(CognitiveCandidateRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: conversation.id,
                candidate_kind: "goal".into(),
                candidate_json: r#"{"hiddenReasoning":"do not persist"}"#.into(),
                idempotency_key: "private-candidate".into(),
            }),
            Err(DatabaseError::Cognitive("conversation_candidate_invalid"))
        );
        cleanup(&path);
    }
}
