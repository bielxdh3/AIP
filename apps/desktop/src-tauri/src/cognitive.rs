use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::database::{ensure_agent, now_millis, Database, DatabaseError, OWNER_ID};

const POLICY_VERSION: i64 = 1;
const SCHEMA_VERSION: i64 = 1;
const MAX_SUBJECT_TYPE: usize = 48;
const MAX_SUBJECT_REF: usize = 128;
const MAX_REASON: usize = 500;
const MAX_CLAIM_KEY: usize = 80;
const MAX_CLAIM_VALUE: usize = 500;
const MAX_ACTIVITY_TYPE: usize = 80;
const MAX_ACTIVITY_DURATION_MS: i64 = 86_400_000;
const MAX_RELATIONSHIP_DELTA: f64 = 0.10;
const MAX_RELATIONSHIP_WINDOW: f64 = 0.20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpinionEvidence {
    pub id: String,
    pub opinion_id: String,
    pub source_kind: String,
    pub classification: String,
    pub stance: f64,
    pub claim_key: String,
    pub claim_value: String,
    pub source_reference: Option<String>,
    pub attribution: Option<String>,
    pub confidence: f64,
    pub status: String,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use uuid::Uuid;

    use super::*;
    use crate::database::{Database, DatabaseError, ASTRA_ID, LUMA_ID};

    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("aip-cognitive-test-{}", Uuid::now_v7()))
            .join("aip.sqlite3")
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path.parent().expect("test path should have a parent"));
    }

    fn opinion_request(agent_id: &str, idempotency_key: &str) -> OpinionCandidateRequest {
        OpinionCandidateRequest {
            agent_id: agent_id.into(),
            subject_type: "topic".into(),
            subject_ref: "bounded-topic".into(),
            stance: 0.4,
            confidence: 0.8,
            source_kind: "owner_testimony".into(),
            classification: "verified_fact".into(),
            claim_key: "name".into(),
            claim_value: "A stable fictional topic".into(),
            source_reference: None,
            attribution: None,
            reason: "Owner provided bounded evidence".into(),
            idempotency_key: idempotency_key.into(),
        }
    }

    fn relationship_request(agent_id: &str, idempotency_key: &str) -> RelationshipCandidateRequest {
        RelationshipCandidateRequest {
            agent_id: agent_id.into(),
            subject_type: "agent".into(),
            subject_ref: "agt_related".into(),
            deltas: RelationshipDeltas {
                familiarity: 0.05,
                trust: 0.03,
                affinity: 0.02,
                admiration: 0.01,
                irritation: 0.0,
                reliability_expectation: 0.02,
            },
            source_kind: "owner_testimony".into(),
            source_reference: None,
            confidence: 0.8,
            reason: "A calm interaction was observed".into(),
            idempotency_key: idempotency_key.into(),
        }
    }

    fn goal_request(agent_id: &str, idempotency_key: &str) -> GoalRequest {
        GoalRequest {
            agent_id: agent_id.into(),
            title: "Study a fictional topic".into(),
            description: "Plan a bounded fictional study".into(),
            priority: 50,
            budget_units: 10,
            due_at: None,
            expires_at: None,
            parent_goal_id: None,
            idempotency_key: idempotency_key.into(),
        }
    }

    #[test]
    fn phase_7b_to_7f_migrations_load() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        assert_eq!(database.snapshot().unwrap().migration_version, 14);
        let connection = database.open().unwrap();
        for table in [
            "cognitive_core_events",
            "opinions",
            "relationships",
            "goals",
            "agent_conversations",
            "cognitive_resource_jobs",
        ] {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
                     )",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "missing migrated table: {table}");
        }
        drop(connection);
        drop(database);
        cleanup(&path);
    }

    #[test]
    fn cognitive_records_remain_agent_owned() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let opinion = database
            .propose_cognitive_opinion(opinion_request(ASTRA_ID, "ownership-opinion"))
            .unwrap();
        let evidence_id = opinion.evidence[0].id.clone();
        let correction = OpinionEvidenceCorrectionRequest {
            agent_id: LUMA_ID.into(),
            evidence_id,
            claim_value: "Cross-agent correction".into(),
            reason: "Owner correction".into(),
            idempotency_key: "ownership-correction".into(),
        };
        assert_eq!(
            database.correct_opinion_evidence(correction).unwrap_err(),
            DatabaseError::OwnershipMismatch
        );

        let parent = database
            .create_owner_goal(goal_request(ASTRA_ID, "ownership-parent"))
            .unwrap();
        let mut child_request = goal_request(LUMA_ID, "ownership-child");
        child_request.parent_goal_id = Some(parent.id);
        assert_eq!(
            database.create_owner_goal(child_request).unwrap_err(),
            DatabaseError::OwnershipMismatch
        );
        cleanup(&path);
    }

    #[test]
    fn cognitive_writes_are_idempotent_and_conflicts_are_rejected() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let request = opinion_request(ASTRA_ID, "idempotent-opinion");
        let first = database.propose_cognitive_opinion(request.clone()).unwrap();
        let replay = database.propose_cognitive_opinion(request).unwrap();
        assert_eq!(replay.id, first.id);
        assert_eq!(replay.evidence.len(), 1);

        let mut conflict = opinion_request(ASTRA_ID, "idempotent-opinion");
        conflict.claim_value = "A different claim".into();
        assert_eq!(
            database.propose_cognitive_opinion(conflict).unwrap_err(),
            DatabaseError::Cognitive("idempotency_conflict")
        );

        let connection = database.open().unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM cognitive_core_events WHERE agent_id = ?1",
                [ASTRA_ID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn cognitive_values_are_bounded() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();

        let mut opinion = opinion_request(ASTRA_ID, "bounded-opinion");
        opinion.stance = 1.1;
        assert_eq!(
            database.propose_cognitive_opinion(opinion).unwrap_err(),
            DatabaseError::Cognitive("invalid_value")
        );

        let mut relationship = relationship_request(ASTRA_ID, "bounded-relationship");
        relationship.deltas.trust = 0.11;
        assert_eq!(
            database
                .propose_relationship_event(relationship)
                .unwrap_err(),
            DatabaseError::Cognitive("relationship_delta_limit")
        );

        let mut goal = goal_request(ASTRA_ID, "bounded-goal");
        goal.budget_units = 1001;
        assert_eq!(
            database.create_owner_goal(goal).unwrap_err(),
            DatabaseError::Cognitive("invalid_goal_budget")
        );
        cleanup(&path);
    }

    #[test]
    fn temporary_chat_references_are_rejected_before_persistence() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut request = opinion_request(ASTRA_ID, "temporary-rejected");
        request.source_kind = "model_inference".into();
        request.source_reference = Some("temporary-chat-1".into());
        assert_eq!(
            database.propose_cognitive_opinion(request).unwrap_err(),
            DatabaseError::Cognitive("source_ineligible")
        );

        let connection = database.open().unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM cognitive_core_events WHERE agent_id = ?1",
                [ASTRA_ID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 0);
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn cognitive_records_survive_restart() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let opinion = database
            .propose_cognitive_opinion(opinion_request(ASTRA_ID, "restart-opinion"))
            .unwrap();
        let relationship = database
            .propose_relationship_event(relationship_request(ASTRA_ID, "restart-relationship"))
            .unwrap();
        let goal = database
            .create_owner_goal(goal_request(ASTRA_ID, "restart-goal"))
            .unwrap();
        drop(database);

        let reopened = Database::initialize(&path).unwrap();
        assert_eq!(
            reopened.list_cognitive_opinions(ASTRA_ID).unwrap()[0].id,
            opinion.id
        );
        assert_eq!(
            reopened.list_relationships(ASTRA_ID).unwrap()[0].id,
            relationship.id
        );
        assert_eq!(
            reopened.list_cognitive_goals(ASTRA_ID).unwrap()[0].id,
            goal.id
        );
        cleanup(&path);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveOpinion {
    pub id: String,
    pub agent_id: String,
    pub subject_type: String,
    pub subject_ref: String,
    pub stance: f64,
    pub confidence: f64,
    pub status: String,
    pub reason: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub evidence: Vec<OpinionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpinionCandidateRequest {
    pub agent_id: String,
    pub subject_type: String,
    pub subject_ref: String,
    pub stance: f64,
    pub confidence: f64,
    pub source_kind: String,
    pub classification: String,
    pub claim_key: String,
    pub claim_value: String,
    pub source_reference: Option<String>,
    pub attribution: Option<String>,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpinionEvidenceCorrectionRequest {
    pub agent_id: String,
    pub evidence_id: String,
    pub claim_value: String,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipValues {
    pub familiarity: f64,
    pub trust: f64,
    pub affinity: f64,
    pub admiration: f64,
    pub irritation: f64,
    pub reliability_expectation: f64,
}

impl Default for RelationshipValues {
    fn default() -> Self {
        Self {
            familiarity: 0.5,
            trust: 0.5,
            affinity: 0.5,
            admiration: 0.5,
            irritation: 0.0,
            reliability_expectation: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDeltas {
    pub familiarity: f64,
    pub trust: f64,
    pub affinity: f64,
    pub admiration: f64,
    pub irritation: f64,
    pub reliability_expectation: f64,
}

impl RelationshipDeltas {
    fn is_zero(&self) -> bool {
        [
            self.familiarity,
            self.trust,
            self.affinity,
            self.admiration,
            self.irritation,
            self.reliability_expectation,
        ]
        .iter()
        .all(|value| value.abs() <= f64::EPSILON)
    }

    fn validate(&self) -> Result<(), DatabaseError> {
        let values = [
            self.familiarity,
            self.trust,
            self.affinity,
            self.admiration,
            self.irritation,
            self.reliability_expectation,
        ];
        if self.is_zero()
            || values
                .iter()
                .any(|value| !value.is_finite() || value.abs() > MAX_RELATIONSHIP_DELTA)
        {
            return Err(DatabaseError::Cognitive("relationship_delta_limit"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipEvent {
    pub id: String,
    pub relationship_id: String,
    pub deltas: RelationshipDeltas,
    pub prior: RelationshipValues,
    pub resulting: RelationshipValues,
    pub source_kind: String,
    pub source_reference: Option<String>,
    pub confidence: f64,
    pub reason: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipState {
    pub id: String,
    pub agent_id: String,
    pub subject_type: String,
    pub subject_ref: String,
    pub values: RelationshipValues,
    pub updated_at: i64,
    pub events: Vec<RelationshipEvent>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipCandidateRequest {
    pub agent_id: String,
    pub subject_type: String,
    pub subject_ref: String,
    pub deltas: RelationshipDeltas,
    pub source_kind: String,
    pub source_reference: Option<String>,
    pub confidence: f64,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveGoal {
    pub id: String,
    pub agent_id: String,
    pub title: String,
    pub description: String,
    pub origin: String,
    pub fictional_only: bool,
    pub priority: i64,
    pub status: String,
    pub budget_units: i64,
    pub due_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub completion_evidence: Option<String>,
    pub parent_goal_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalRequest {
    pub agent_id: String,
    pub title: String,
    pub description: String,
    pub priority: i64,
    pub budget_units: i64,
    pub due_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub parent_goal_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FictionalActivity {
    pub id: String,
    pub goal_id: String,
    pub agent_id: String,
    pub activity_type: String,
    pub status: String,
    pub fictional_only: bool,
    pub budget_units: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FictionalActivityRequest {
    pub agent_id: String,
    pub goal_id: String,
    pub activity_type: String,
    pub budget_units: i64,
    pub duration_ms: i64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone)]
struct CoreEventRecord {
    id: String,
    kind: String,
    payload_json: String,
    result_ref: Option<String>,
}

fn text(value: &str, max: usize, code: &'static str) -> Result<String, DatabaseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max || trimmed.chars().any(char::is_control) {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(trimmed.to_owned())
}

fn reference(value: &str, max: usize, code: &'static str) -> Result<String, DatabaseError> {
    let trimmed = text(value, max, code)?;
    if trimmed.contains("temporary-")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\n')
    {
        return Err(DatabaseError::Cognitive("source_ineligible"));
    }
    Ok(trimmed)
}

fn idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = text(value, 128, "invalid_idempotency_key")?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':'))
    {
        return Err(DatabaseError::Cognitive("invalid_idempotency_key"));
    }
    Ok(value)
}

fn finite_range(value: f64, min: f64, max: f64, code: &'static str) -> Result<f64, DatabaseError> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(value)
}

fn canonical_source(value: &str) -> Result<&'static str, DatabaseError> {
    match value.trim() {
        "owner" | "owner_testimony" | "direct_owner" => Ok("owner_testimony"),
        "model" | "model_inference" => Ok("model_inference"),
        "internet" | "future_internet" | "internet_information" => Ok("internet_information"),
        _ => Err(DatabaseError::Cognitive("source_ineligible")),
    }
}

fn valid_classification(value: &str) -> bool {
    matches!(
        value,
        "verified_fact" | "reported_experience" | "impression"
    )
}

fn validate_subject(
    subject_type: &str,
    subject_ref: &str,
) -> Result<(String, String), DatabaseError> {
    let subject_type = reference(subject_type, MAX_SUBJECT_TYPE, "invalid_subject")?;
    let subject_ref = reference(subject_ref, MAX_SUBJECT_REF, "invalid_subject")?;
    if !matches!(
        subject_type.as_str(),
        "topic" | "object" | "fictional_character" | "real_person" | "agent"
    ) {
        return Err(DatabaseError::Cognitive("invalid_subject"));
    }
    Ok((subject_type, subject_ref))
}

fn unsafe_relationship_language(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "guilt",
        "threat",
        "retaliat",
        "replace humans",
        "only me",
        "exclusiv",
        "depend",
        "culpa",
        "ameaça",
        "retalia",
        "substituir pessoas",
        "só comigo",
        "exclusiv",
        "dependência",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn unsafe_external_goal_language(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "file",
        "arquivo",
        "app",
        "application",
        "aplicativo",
        "device",
        "dispositivo",
        "network",
        "rede",
        "calendar",
        "calendário",
        "email",
        "e-mail",
        "message",
        "mensagem",
        "account",
        "conta",
        "website",
        "site",
        "download",
        "install",
        "instalar",
        "send",
        "enviar",
        "delete",
        "apagar",
        "external",
        "externo",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn unsafe_defamatory_language(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "is a criminal",
        "é criminoso",
        "dangerous person",
        "pessoa perigosa",
        "stole",
        "roubou",
        "fraud",
        "fraude",
        "liar",
        "mentiroso",
        "evil",
        "maligno",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn validate_source_reference(
    source_kind: &str,
    source_reference: Option<&str>,
) -> Result<Option<String>, DatabaseError> {
    let canonical = canonical_source(source_kind)?;
    let reference = source_reference
        .map(|value| reference(value, MAX_SUBJECT_REF, "source_ineligible"))
        .transpose()?;
    if canonical != "owner_testimony" && reference.is_none() {
        return Err(DatabaseError::Cognitive("source_not_found"));
    }
    Ok(reference)
}

fn ensure_owned(connection: &Connection, agent_id: &str) -> Result<(), DatabaseError> {
    ensure_agent(connection, agent_id)
}

fn ensure_owned_tx(tx: &Transaction<'_>, agent_id: &str) -> Result<String, DatabaseError> {
    tx.query_row(
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

fn versioned_payload(payload: &Value) -> Value {
    json!({
        "schemaVersion": SCHEMA_VERSION,
        "policyVersion": POLICY_VERSION,
        "data": payload,
    })
}

fn core_event(
    tx: &Transaction<'_>,
    agent_id: &str,
    owner_id: &str,
    idempotency_key: &str,
    kind: &str,
    subject_type: &str,
    subject_ref: &str,
    source_kind: &str,
    source_reference: Option<&str>,
    reason: &str,
    confidence: f64,
    payload: &Value,
    result_ref: Option<&str>,
    related_event_id: Option<&str>,
    now: i64,
) -> Result<CoreEventRecord, DatabaseError> {
    let payload_json = serde_json::to_string(&versioned_payload(payload))
        .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?;
    let id = Uuid::now_v7().to_string();
    tx.execute(
        "INSERT INTO cognitive_core_events
         (id, agent_id, owner_user_id, idempotency_key, kind, subject_type, subject_ref,
          source_kind, source_reference, reason, confidence, payload_json, status, result_ref,
          related_event_id, created_at, terminal_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'applied', ?13, ?14, ?15, ?15)",
        params![
            id,
            agent_id,
            owner_id,
            idempotency_key,
            kind,
            subject_type,
            subject_ref,
            source_kind,
            source_reference,
            reason,
            confidence,
            payload_json,
            result_ref,
            related_event_id,
            now
        ],
    )?;
    Ok(CoreEventRecord {
        id,
        kind: kind.to_owned(),
        payload_json,
        result_ref: result_ref.map(str::to_owned),
    })
}

fn existing_core_event(
    tx: &Transaction<'_>,
    agent_id: &str,
    key: &str,
) -> Result<Option<CoreEventRecord>, DatabaseError> {
    tx.query_row(
        "SELECT id, kind, payload_json, result_ref
         FROM cognitive_core_events WHERE agent_id = ?1 AND idempotency_key = ?2",
        params![agent_id, key],
        |row| {
            Ok(CoreEventRecord {
                id: row.get(0)?,
                kind: row.get(1)?,
                payload_json: row.get(2)?,
                result_ref: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn idempotent_or_conflict(
    existing: Option<CoreEventRecord>,
    kind: &str,
    payload: &Value,
) -> Result<Option<CoreEventRecord>, DatabaseError> {
    let Some(existing) = existing else {
        return Ok(None);
    };
    let payload_json = serde_json::to_string(&versioned_payload(payload))
        .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?;
    if existing.kind != kind || existing.payload_json != payload_json {
        return Err(DatabaseError::Cognitive("idempotency_conflict"));
    }
    Ok(Some(existing))
}

fn map_opinion(row: &rusqlite::Row<'_>) -> rusqlite::Result<CognitiveOpinion> {
    Ok(CognitiveOpinion {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        subject_type: row.get(2)?,
        subject_ref: row.get(3)?,
        stance: row.get(4)?,
        confidence: row.get(5)?,
        status: row.get(6)?,
        reason: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        evidence: Vec::new(),
    })
}

fn map_evidence(row: &rusqlite::Row<'_>) -> rusqlite::Result<OpinionEvidence> {
    Ok(OpinionEvidence {
        id: row.get(0)?,
        opinion_id: row.get(1)?,
        source_kind: row.get(2)?,
        classification: row.get(3)?,
        stance: row.get(4)?,
        claim_key: row.get(5)?,
        claim_value: row.get(6)?,
        source_reference: row.get(7)?,
        attribution: row.get(8)?,
        confidence: row.get(9)?,
        status: row.get(10)?,
        created_at: row.get(11)?,
    })
}

fn map_relationship(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelationshipState> {
    Ok(RelationshipState {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        subject_type: row.get(2)?,
        subject_ref: row.get(3)?,
        values: RelationshipValues {
            familiarity: row.get(4)?,
            trust: row.get(5)?,
            affinity: row.get(6)?,
            admiration: row.get(7)?,
            irritation: row.get(8)?,
            reliability_expectation: row.get(9)?,
        },
        updated_at: row.get(10)?,
        events: Vec::new(),
    })
}

fn map_goal(row: &rusqlite::Row<'_>) -> rusqlite::Result<CognitiveGoal> {
    Ok(CognitiveGoal {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        origin: row.get(4)?,
        fictional_only: row.get::<_, bool>(5)?,
        priority: row.get(6)?,
        status: row.get(7)?,
        budget_units: row.get(8)?,
        due_at: row.get(9)?,
        expires_at: row.get(10)?,
        completion_evidence: row.get(11)?,
        parent_goal_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn map_activity(row: &rusqlite::Row<'_>) -> rusqlite::Result<FictionalActivity> {
    Ok(FictionalActivity {
        id: row.get(0)?,
        goal_id: row.get(1)?,
        agent_id: row.get(2)?,
        activity_type: row.get(3)?,
        status: row.get(4)?,
        fictional_only: row.get::<_, bool>(5)?,
        budget_units: row.get(6)?,
        started_at: row.get(7)?,
        ended_at: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn map_relationship_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelationshipEvent> {
    let deltas: RelationshipDeltas = serde_json::from_str(&row.get::<_, String>(2)?)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let prior: RelationshipValues = serde_json::from_str(&row.get::<_, String>(3)?)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let resulting: RelationshipValues = serde_json::from_str(&row.get::<_, String>(4)?)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(RelationshipEvent {
        id: row.get(0)?,
        relationship_id: row.get(1)?,
        deltas,
        prior,
        resulting,
        source_kind: row.get(5)?,
        source_reference: row.get(6)?,
        confidence: row.get(7)?,
        reason: row.get(8)?,
        status: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn load_relationship_tx(
    tx: &Transaction<'_>,
    agent_id: &str,
    relationship_id: &str,
) -> Result<RelationshipState, DatabaseError> {
    let mut relationship = tx
        .query_row(
            "SELECT id, agent_id, subject_type, subject_ref, familiarity, trust, affinity,
                    admiration, irritation, reliability_expectation, updated_at
             FROM relationships WHERE agent_id = ?1 AND id = ?2",
            params![agent_id, relationship_id],
            map_relationship,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("relationship_not_found"))?;
    let mut statement = tx.prepare(
        "SELECT id, relationship_id, delta_json, prior_json, resulting_json,
                source_kind, source_reference, confidence, reason, status, created_at
         FROM relationship_events WHERE relationship_id = ?1
         ORDER BY created_at DESC, id DESC",
    )?;
    relationship.events = statement
        .query_map(params![relationship_id], map_relationship_event)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(relationship)
}

fn relationship_exists(tx: &Transaction<'_>, relationship_id: &str) -> Result<bool, DatabaseError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM relationships WHERE id = ?1)",
        params![relationship_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn relationship_window_total(
    tx: &Transaction<'_>,
    relationship_id: &str,
    now: i64,
) -> Result<f64, DatabaseError> {
    let mut statement = tx.prepare(
        "SELECT delta_json FROM relationship_events
         WHERE relationship_id = ?1 AND status = 'applied' AND created_at >= ?2",
    )?;
    let mut total = 0.0;
    for row in statement.query_map(params![relationship_id, now - 30 * 86_400_000_i64], |row| {
        row.get::<_, String>(0)
    })? {
        let deltas: RelationshipDeltas =
            serde_json::from_str(&row?).map_err(|_| rusqlite::Error::InvalidQuery)?;
        total += [
            deltas.familiarity.abs(),
            deltas.trust.abs(),
            deltas.affinity.abs(),
            deltas.admiration.abs(),
            deltas.irritation.abs(),
            deltas.reliability_expectation.abs(),
        ]
        .iter()
        .sum::<f64>();
    }
    Ok(total)
}

fn load_goal_tx(
    tx: &Transaction<'_>,
    agent_id: &str,
    goal_id: &str,
) -> Result<CognitiveGoal, DatabaseError> {
    tx.query_row(
        "SELECT id, agent_id, title, description, origin, fictional_only, priority, status,
                budget_units, due_at, expires_at, completion_evidence, parent_goal_id,
                created_at, updated_at
         FROM goals WHERE agent_id = ?1 AND id = ?2",
        params![agent_id, goal_id],
        map_goal,
    )
    .optional()?
    .ok_or(DatabaseError::Cognitive("goal_not_found"))
}

fn load_activity_tx(
    tx: &Transaction<'_>,
    agent_id: &str,
    activity_id: &str,
) -> Result<FictionalActivity, DatabaseError> {
    tx.query_row(
        "SELECT id, goal_id, agent_id, activity_type, status, fictional_only,
                budget_units, started_at, ended_at, created_at
         FROM fictional_activities WHERE agent_id = ?1 AND id = ?2",
        params![agent_id, activity_id],
        map_activity,
    )
    .optional()?
    .ok_or(DatabaseError::Cognitive("activity_not_found"))
}

fn load_opinion_tx(
    tx: &Transaction<'_>,
    agent_id: &str,
    opinion_id: &str,
) -> Result<CognitiveOpinion, DatabaseError> {
    let mut opinion = tx
        .query_row(
            "SELECT id, agent_id, subject_type, subject_ref, stance, confidence, status,
                    reason, created_at, updated_at
             FROM opinions WHERE agent_id = ?1 AND id = ?2",
            params![agent_id, opinion_id],
            map_opinion,
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("opinion_not_found"))?;
    let mut statement = tx.prepare(
        "SELECT id, opinion_id, source_kind, classification, stance, claim_key, claim_value,
                source_reference, attribution, confidence, status, created_at
         FROM opinion_evidence WHERE opinion_id = ?1 ORDER BY created_at DESC, id DESC",
    )?;
    opinion.evidence = statement
        .query_map(params![opinion_id], map_evidence)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(opinion)
}

fn recalculate_opinion_values(
    tx: &Transaction<'_>,
    opinion_id: &str,
) -> Result<(f64, f64, bool), DatabaseError> {
    let mut statement = tx.prepare(
        "SELECT source_kind, classification, stance, confidence
         FROM opinion_evidence WHERE opinion_id = ?1 AND status = 'active'",
    )?;
    let rows = statement
        .query_map(params![opinion_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Err(DatabaseError::Cognitive("opinion_evidence_required"));
    }
    let mut weighted_stance = 0.0;
    let mut weight_total = 0.0;
    let mut stances = Vec::new();
    for (source, classification, stance, confidence) in rows {
        let source_weight = match source.as_str() {
            "owner_testimony" => 1.0,
            "model_inference" => 0.6,
            "internet_information" => 0.5,
            _ => 0.25,
        };
        let classification_weight = match classification.as_str() {
            "verified_fact" => 0.9,
            "reported_experience" => 0.7,
            _ => 0.5,
        };
        let weight = source_weight * classification_weight * confidence.max(0.05);
        weighted_stance += stance * weight;
        weight_total += weight;
        stances.push(stance);
    }
    let stance = (weighted_stance / weight_total.max(f64::EPSILON)).clamp(-1.0, 1.0);
    let confidence = (weight_total / stances.len() as f64).clamp(0.0, 1.0);
    let disputed = stances.iter().enumerate().any(|(index, stance)| {
        stances
            .iter()
            .skip(index + 1)
            .any(|other| (*stance - *other).abs() > 0.6)
    });
    Ok((stance, confidence, disputed))
}

impl Database {
    pub fn list_cognitive_opinions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CognitiveOpinion>, DatabaseError> {
        let connection = self.open()?;
        ensure_owned(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, agent_id, subject_type, subject_ref, stance, confidence, status,
                    reason, created_at, updated_at
             FROM opinions WHERE agent_id = ?1 ORDER BY updated_at DESC, id DESC",
        )?;
        let ids = statement
            .query_map(params![agent_id], map_opinion)?
            .collect::<Result<Vec<_>, _>>()?;
        let tx = connection.unchecked_transaction()?;
        let opinions = ids
            .into_iter()
            .map(|opinion| load_opinion_tx(&tx, agent_id, &opinion.id))
            .collect::<Result<Vec<_>, _>>()?;
        tx.rollback()?;
        Ok(opinions)
    }

    pub fn propose_cognitive_opinion(
        &self,
        request: OpinionCandidateRequest,
    ) -> Result<CognitiveOpinion, DatabaseError> {
        let (subject_type, subject_ref) =
            validate_subject(&request.subject_type, &request.subject_ref)?;
        let stance = finite_range(request.stance, -1.0, 1.0, "invalid_value")?;
        let confidence = finite_range(request.confidence, 0.0, 1.0, "invalid_value")?;
        let source_kind = canonical_source(&request.source_kind)?;
        let source_reference =
            validate_source_reference(source_kind, request.source_reference.as_deref())?;
        let classification = text(&request.classification, 32, "invalid_classification")?;
        if !valid_classification(&classification) {
            return Err(DatabaseError::Cognitive("invalid_classification"));
        }
        let claim_key = reference(&request.claim_key, MAX_CLAIM_KEY, "invalid_evidence")?;
        let claim_value = text(&request.claim_value, MAX_CLAIM_VALUE, "invalid_evidence")?;
        let reason = text(&request.reason, MAX_REASON, "invalid_reason")?;
        let idempotency_key = idempotency(&request.idempotency_key)?;
        let attribution = request
            .attribution
            .as_deref()
            .map(|value| text(value, 160, "invalid_evidence"))
            .transpose()?;
        if classification == "reported_experience" && attribution.is_none() {
            return Err(DatabaseError::Cognitive("attribution_required"));
        }
        if source_kind == "internet_information" && classification == "verified_fact" {
            return Err(DatabaseError::Cognitive("internet_fact_unverified"));
        }
        if source_kind == "model_inference" && classification == "verified_fact" {
            return Err(DatabaseError::Cognitive("inference_not_fact"));
        }
        if subject_type == "real_person" && (classification == "verified_fact" || confidence > 0.75)
        {
            return Err(DatabaseError::Cognitive("real_person_uncertain"));
        }
        if unsafe_defamatory_language(&claim_value) || unsafe_defamatory_language(&reason) {
            return Err(DatabaseError::Cognitive("defamation_blocked"));
        }
        let payload = json!({
            "subjectType": subject_type,
            "subjectRef": subject_ref,
            "stance": stance,
            "confidence": confidence,
            "sourceKind": source_kind,
            "classification": classification,
            "claimKey": claim_key,
            "claimValue": claim_value,
            "sourceReference": source_reference,
            "attribution": attribution,
            "reason": reason,
        });
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, &request.agent_id)?;
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, &request.agent_id, &idempotency_key)?,
            "opinion_evidence",
            &payload,
        )? {
            let opinion_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let opinion = load_opinion_tx(&tx, &request.agent_id, &opinion_id)?;
            tx.commit()?;
            return Ok(opinion);
        }
        let now = now_millis();
        let opinion_id = tx
            .query_row(
                "SELECT id FROM opinions WHERE agent_id = ?1 AND subject_type = ?2 AND subject_ref = ?3",
                params![request.agent_id, subject_type, subject_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let opinion_id = opinion_id.unwrap_or_else(|| Uuid::now_v7().to_string());
        let duplicate_evidence: bool = tx.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM opinion_evidence
               WHERE opinion_id = ?1 AND status = 'active'
                 AND source_kind = ?2 AND source_reference IS ?3
                 AND classification = ?4 AND stance = ?5
                 AND claim_key = ?6 AND claim_value = ?7
             )",
            params![
                opinion_id,
                source_kind,
                source_reference.as_deref(),
                classification,
                stance,
                claim_key,
                claim_value
            ],
            |row| row.get(0),
        )?;
        if duplicate_evidence {
            return Err(DatabaseError::Cognitive("duplicate_evidence"));
        }
        let prior = tx
            .query_row(
                "SELECT stance, confidence, status, reason FROM opinions WHERE id = ?1 AND agent_id = ?2",
                params![opinion_id, request.agent_id],
                |row| {
                    Ok(json!({
                        "stance": row.get::<_, f64>(0)?,
                        "confidence": row.get::<_, f64>(1)?,
                        "status": row.get::<_, String>(2)?,
                        "reason": row.get::<_, String>(3)?,
                    }))
                },
            )
            .optional()?
            .unwrap_or_else(|| json!({"status": "rejected"}));
        if prior.get("status").and_then(Value::as_str) == Some("rejected") {
            tx.execute(
                "INSERT INTO opinions
                 (id, agent_id, owner_user_id, subject_type, subject_ref, stance, confidence,
                  status, reason, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?9, ?9)",
                params![
                    opinion_id,
                    request.agent_id,
                    owner_id,
                    subject_type,
                    subject_ref,
                    stance,
                    confidence,
                    reason,
                    now
                ],
            )?;
        }
        let event = core_event(
            &tx,
            &request.agent_id,
            &owner_id,
            &idempotency_key,
            "opinion_evidence",
            &subject_type,
            &subject_ref,
            source_kind,
            source_reference.as_deref(),
            &reason,
            confidence,
            &payload,
            Some(&opinion_id),
            None,
            now,
        )?;
        tx.execute(
            "INSERT INTO opinion_evidence
             (id, opinion_id, agent_id, owner_user_id, source_kind, classification, stance,
              claim_key, claim_value, source_reference, attribution, confidence, status, event_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'active', ?13, ?14)",
            params![
                Uuid::now_v7().to_string(),
                opinion_id,
                request.agent_id,
                owner_id,
                source_kind,
                classification,
                stance,
                claim_key,
                claim_value,
                source_reference,
                attribution,
                confidence,
                event.id,
                now
            ],
        )?;
        let (resulting_stance, resulting_confidence, conflicted) =
            recalculate_opinion_values(&tx, &opinion_id)?;
        let status = if conflicted {
            "disputed"
        } else if prior.get("status").and_then(Value::as_str) == Some("disputed") {
            "disputed"
        } else {
            "active"
        };
        tx.execute(
            "UPDATE opinions SET stance = ?1, confidence = ?2, status = ?3, reason = ?4,
             current_event_id = ?5, updated_at = ?6 WHERE id = ?7 AND agent_id = ?8",
            params![
                resulting_stance,
                resulting_confidence,
                status,
                reason,
                event.id,
                now,
                opinion_id,
                request.agent_id
            ],
        )?;
        tx.execute(
            "INSERT INTO cognitive_core_checkpoints
             (agent_id, processor_key, source_key, event_id, terminal_status, updated_at)
             VALUES (?1, 'phase7b', ?2, ?3, 'applied', ?4)",
            params![request.agent_id, idempotency_key, event.id, now],
        )?;
        let opinion = load_opinion_tx(&tx, &request.agent_id, &opinion_id)?;
        tx.commit()?;
        Ok(opinion)
    }

    pub fn correct_opinion_evidence(
        &self,
        request: OpinionEvidenceCorrectionRequest,
    ) -> Result<CognitiveOpinion, DatabaseError> {
        let evidence_id = reference(&request.evidence_id, 128, "evidence_not_found")?;
        let claim_value = text(&request.claim_value, MAX_CLAIM_VALUE, "invalid_evidence")?;
        let reason = text(&request.reason, MAX_REASON, "invalid_reason")?;
        let idempotency_key = idempotency(&request.idempotency_key)?;
        if unsafe_defamatory_language(&claim_value) || unsafe_defamatory_language(&reason) {
            return Err(DatabaseError::Cognitive("defamation_blocked"));
        }
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, &request.agent_id)?;
        let evidence_agent: Option<String> = tx
            .query_row(
                "SELECT agent_id FROM opinion_evidence WHERE id = ?1",
                params![evidence_id],
                |row| row.get(0),
            )
            .optional()?;
        if evidence_agent
            .as_deref()
            .is_some_and(|agent| agent != request.agent_id)
        {
            return Err(DatabaseError::OwnershipMismatch);
        }
        let (
            opinion_id,
            subject_type,
            subject_ref,
            _old_source_kind,
            old_classification,
            old_stance,
            old_confidence,
            old_status,
            old_event_id,
        ): (
            String,
            String,
            String,
            String,
            String,
            f64,
            f64,
            String,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT oe.opinion_id, o.subject_type, o.subject_ref, oe.source_kind,
                        oe.classification, oe.stance, oe.confidence, oe.status, oe.event_id
                 FROM opinion_evidence oe
                 JOIN opinions o ON o.id = oe.opinion_id AND o.agent_id = oe.agent_id
                 WHERE oe.id = ?1 AND oe.agent_id = ?2",
                params![evidence_id, request.agent_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()?
            .ok_or(DatabaseError::Cognitive("evidence_not_found"))?;
        if old_status != "active" {
            return Err(DatabaseError::Cognitive("evidence_not_active"));
        }
        let payload = json!({
            "evidenceId": evidence_id,
            "claimValue": claim_value,
            "reason": reason,
        });
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, &request.agent_id, &idempotency_key)?,
            "opinion_evidence",
            &payload,
        )? {
            let opinion_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let opinion = load_opinion_tx(&tx, &request.agent_id, &opinion_id)?;
            tx.commit()?;
            return Ok(opinion);
        }
        let classification = if subject_type == "real_person" {
            "reported_experience"
        } else {
            old_classification.as_str()
        };
        let confidence = if subject_type == "real_person" {
            old_confidence.min(0.75)
        } else {
            old_confidence
        };
        let now = now_millis();
        let event = core_event(
            &tx,
            &request.agent_id,
            &owner_id,
            &idempotency_key,
            "opinion_evidence",
            &subject_type,
            &subject_ref,
            "owner_testimony",
            Some("owner"),
            &reason,
            confidence,
            &payload,
            Some(&opinion_id),
            old_event_id.as_deref(),
            now,
        )?;
        tx.execute(
            "UPDATE opinion_evidence SET status = 'superseded' WHERE id = ?1 AND agent_id = ?2",
            params![evidence_id, request.agent_id],
        )?;
        tx.execute(
            "INSERT INTO opinion_evidence
             (id, opinion_id, agent_id, owner_user_id, source_kind, classification, stance,
              claim_key, claim_value, source_reference, attribution, confidence, status, event_id, created_at)
             SELECT ?1, opinion_id, agent_id, owner_user_id, 'owner_testimony', ?2, ?3,
                    claim_key, ?4, 'owner', 'Owner correction', ?5, 'active', ?6, ?7
             FROM opinion_evidence WHERE id = ?8 AND agent_id = ?9",
            params![
                Uuid::now_v7().to_string(),
                classification,
                old_stance,
                claim_value,
                confidence,
                event.id,
                now,
                evidence_id,
                request.agent_id
            ],
        )?;
        let (stance, recalculated_confidence, conflicted) =
            recalculate_opinion_values(&tx, &opinion_id)?;
        let status = if conflicted { "disputed" } else { "active" };
        tx.execute(
            "UPDATE opinions SET stance = ?1, confidence = ?2, status = ?3, reason = ?4,
             current_event_id = ?5, updated_at = ?6 WHERE id = ?7 AND agent_id = ?8",
            params![
                stance,
                recalculated_confidence,
                status,
                reason,
                event.id,
                now,
                opinion_id,
                request.agent_id
            ],
        )?;
        tx.execute(
            "INSERT INTO cognitive_core_checkpoints
             (agent_id, processor_key, source_key, event_id, terminal_status, updated_at)
             VALUES (?1, 'phase7b', ?2, ?3, 'applied', ?4)",
            params![request.agent_id, idempotency_key, event.id, now],
        )?;
        let opinion = load_opinion_tx(&tx, &request.agent_id, &opinion_id)?;
        tx.commit()?;
        Ok(opinion)
    }

    pub fn set_opinion_status(
        &self,
        agent_id: &str,
        opinion_id: &str,
        status: &str,
        reason: &str,
        idempotency_key: &str,
    ) -> Result<CognitiveOpinion, DatabaseError> {
        let status = text(status, 32, "invalid_status")?;
        if !matches!(
            status.as_str(),
            "disputed" | "superseded" | "archived" | "rejected"
        ) {
            return Err(DatabaseError::Cognitive("invalid_status"));
        }
        let reason = text(reason, MAX_REASON, "invalid_reason")?;
        let idempotency_key = idempotency(idempotency_key)?;
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, agent_id)?;
        let _prior = load_opinion_tx(&tx, agent_id, opinion_id)?;
        let payload = json!({ "opinionId": opinion_id, "status": status, "reason": reason });
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, agent_id, &idempotency_key)?,
            "opinion_status",
            &payload,
        )? {
            let opinion_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let opinion = load_opinion_tx(&tx, agent_id, &opinion_id)?;
            tx.commit()?;
            return Ok(opinion);
        }
        let now = now_millis();
        let event = core_event(
            &tx,
            agent_id,
            &owner_id,
            &idempotency_key,
            "opinion_status",
            "opinion",
            opinion_id,
            "owner_testimony",
            Some("owner"),
            &reason,
            1.0,
            &payload,
            Some(opinion_id),
            None,
            now,
        )?;
        tx.execute(
            "UPDATE opinions SET status = ?1, reason = ?2, current_event_id = ?3, updated_at = ?4
             WHERE id = ?5 AND agent_id = ?6",
            params![status, reason, event.id, now, opinion_id, agent_id],
        )?;
        let opinion = load_opinion_tx(&tx, agent_id, opinion_id)?;
        tx.commit()?;
        Ok(opinion)
    }

    pub fn recalculate_opinion(
        &self,
        agent_id: &str,
        opinion_id: &str,
        reason: &str,
        idempotency_key: &str,
    ) -> Result<CognitiveOpinion, DatabaseError> {
        let reason = text(reason, MAX_REASON, "invalid_reason")?;
        let idempotency_key = idempotency(idempotency_key)?;
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, agent_id)?;
        let _prior = load_opinion_tx(&tx, agent_id, opinion_id)?;
        let payload = json!({ "opinionId": opinion_id, "reason": reason });
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, agent_id, &idempotency_key)?,
            "opinion_recalculate",
            &payload,
        )? {
            let opinion_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let opinion = load_opinion_tx(&tx, agent_id, &opinion_id)?;
            tx.commit()?;
            return Ok(opinion);
        }
        let (stance, confidence, conflicted) = recalculate_opinion_values(&tx, opinion_id)?;
        let status = if conflicted { "disputed" } else { "active" };
        let now = now_millis();
        let event = core_event(
            &tx,
            agent_id,
            &owner_id,
            &idempotency_key,
            "opinion_recalculate",
            "opinion",
            opinion_id,
            "owner_testimony",
            Some("owner"),
            &reason,
            confidence,
            &payload,
            Some(opinion_id),
            None,
            now,
        )?;
        tx.execute(
            "UPDATE opinions SET stance = ?1, confidence = ?2, status = ?3, reason = ?4,
             current_event_id = ?5, updated_at = ?6 WHERE id = ?7 AND agent_id = ?8",
            params![stance, confidence, status, reason, event.id, now, opinion_id, agent_id],
        )?;
        let opinion = load_opinion_tx(&tx, agent_id, opinion_id)?;
        tx.commit()?;
        Ok(opinion)
    }

    pub fn list_relationships(
        &self,
        agent_id: &str,
    ) -> Result<Vec<RelationshipState>, DatabaseError> {
        let connection = self.open()?;
        ensure_owned(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, agent_id, subject_type, subject_ref, familiarity, trust, affinity,
                    admiration, irritation, reliability_expectation, updated_at
             FROM relationships WHERE agent_id = ?1 ORDER BY updated_at DESC, id DESC",
        )?;
        let relationships = statement
            .query_map(params![agent_id], map_relationship)?
            .collect::<Result<Vec<_>, _>>()?;
        let mut relationships_with_events = Vec::with_capacity(relationships.len());
        for mut relationship in relationships {
            let mut events = connection.prepare(
                "SELECT re.id, re.relationship_id, re.delta_json, re.prior_json,
                        re.resulting_json, re.source_kind, re.source_reference, re.confidence,
                        re.reason, re.status, re.created_at
                 FROM relationship_events re
                 WHERE re.relationship_id = ?1 ORDER BY re.created_at DESC, re.id DESC",
            )?;
            relationship.events = events
                .query_map(params![relationship.id], |row| {
                    let deltas: RelationshipDeltas =
                        serde_json::from_str(&row.get::<_, String>(2)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?;
                    let prior: RelationshipValues = serde_json::from_str(&row.get::<_, String>(3)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?;
                    let resulting: RelationshipValues =
                        serde_json::from_str(&row.get::<_, String>(4)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?;
                    Ok(RelationshipEvent {
                        id: row.get(0)?,
                        relationship_id: row.get(1)?,
                        deltas,
                        prior,
                        resulting,
                        source_kind: row.get(5)?,
                        source_reference: row.get(6)?,
                        confidence: row.get(7)?,
                        reason: row.get(8)?,
                        status: row.get(9)?,
                        created_at: row.get(10)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            relationships_with_events.push(relationship);
        }
        Ok(relationships_with_events)
    }

    pub fn propose_relationship_event(
        &self,
        request: RelationshipCandidateRequest,
    ) -> Result<RelationshipState, DatabaseError> {
        let (subject_type, subject_ref) =
            validate_subject(&request.subject_type, &request.subject_ref)?;
        request.deltas.validate()?;
        let confidence = finite_range(request.confidence, 0.0, 1.0, "invalid_value")?;
        let source_kind = canonical_source(&request.source_kind)?;
        let source_reference =
            validate_source_reference(source_kind, request.source_reference.as_deref())?;
        let reason = text(&request.reason, MAX_REASON, "invalid_reason")?;
        if unsafe_relationship_language(&reason) || unsafe_relationship_language(&subject_ref) {
            return Err(DatabaseError::Cognitive("manipulation_blocked"));
        }
        let idempotency_key = idempotency(&request.idempotency_key)?;
        let deltas_json = serde_json::to_string(&request.deltas)
            .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?;
        let payload = json!({
            "subjectType": subject_type,
            "subjectRef": subject_ref,
            "deltas": request.deltas,
            "sourceKind": source_kind,
            "sourceReference": source_reference,
            "confidence": confidence,
            "reason": reason,
        });
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, &request.agent_id)?;
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, &request.agent_id, &idempotency_key)?,
            "relationship_event",
            &payload,
        )? {
            let relationship_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let relationship = load_relationship_tx(&tx, &request.agent_id, &relationship_id)?;
            tx.commit()?;
            return Ok(relationship);
        }
        let now = now_millis();
        let relationship_id = tx
            .query_row(
                "SELECT id FROM relationships WHERE agent_id = ?1 AND subject_type = ?2 AND subject_ref = ?3",
                params![request.agent_id, subject_type, subject_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let prior = tx
            .query_row(
                "SELECT familiarity, trust, affinity, admiration, irritation, reliability_expectation
                 FROM relationships WHERE id = ?1 AND agent_id = ?2",
                params![relationship_id, request.agent_id],
                |row| {
                    Ok(RelationshipValues {
                        familiarity: row.get(0)?,
                        trust: row.get(1)?,
                        affinity: row.get(2)?,
                        admiration: row.get(3)?,
                        irritation: row.get(4)?,
                        reliability_expectation: row.get(5)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();
        if !relationship_exists(&tx, &relationship_id)? {
            tx.execute(
                "INSERT INTO relationships
                 (id, agent_id, owner_user_id, subject_type, subject_ref, familiarity, trust,
                  affinity, admiration, irritation, reliability_expectation, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                params![
                    relationship_id,
                    request.agent_id,
                    owner_id,
                    subject_type,
                    subject_ref,
                    prior.familiarity,
                    prior.trust,
                    prior.affinity,
                    prior.admiration,
                    prior.irritation,
                    prior.reliability_expectation,
                    now
                ],
            )?;
        }
        let recent_total = relationship_window_total(&tx, &relationship_id, now)?;
        let delta_total = [
            request.deltas.familiarity.abs(),
            request.deltas.trust.abs(),
            request.deltas.affinity.abs(),
            request.deltas.admiration.abs(),
            request.deltas.irritation.abs(),
            request.deltas.reliability_expectation.abs(),
        ]
        .iter()
        .sum::<f64>();
        if recent_total + delta_total > MAX_RELATIONSHIP_WINDOW + f64::EPSILON {
            return Err(DatabaseError::Cognitive("relationship_rate_limit"));
        }
        let resulting = RelationshipValues {
            familiarity: (prior.familiarity + request.deltas.familiarity).clamp(0.0, 1.0),
            trust: (prior.trust + request.deltas.trust).clamp(0.0, 1.0),
            affinity: (prior.affinity + request.deltas.affinity).clamp(0.0, 1.0),
            admiration: (prior.admiration + request.deltas.admiration).clamp(0.0, 1.0),
            irritation: (prior.irritation + request.deltas.irritation).clamp(0.0, 1.0),
            reliability_expectation: (prior.reliability_expectation
                + request.deltas.reliability_expectation)
                .clamp(0.0, 1.0),
        };
        let prior_json = serde_json::to_string(&prior)
            .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?;
        let resulting_json = serde_json::to_string(&resulting)
            .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?;
        let event = core_event(
            &tx,
            &request.agent_id,
            &owner_id,
            &idempotency_key,
            "relationship_event",
            &subject_type,
            &subject_ref,
            source_kind,
            source_reference.as_deref(),
            &reason,
            confidence,
            &payload,
            Some(&relationship_id),
            None,
            now,
        )?;
        tx.execute(
            "INSERT INTO relationship_events
             (id, relationship_id, agent_id, owner_user_id, event_id, delta_json, prior_json,
              resulting_json, source_kind, source_reference, confidence, reason, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'applied', ?13)",
            params![
                Uuid::now_v7().to_string(),
                relationship_id,
                request.agent_id,
                owner_id,
                event.id,
                deltas_json,
                prior_json,
                resulting_json,
                source_kind,
                source_reference,
                confidence,
                reason,
                now
            ],
        )?;
        tx.execute(
            "UPDATE relationships SET familiarity = ?1, trust = ?2, affinity = ?3,
             admiration = ?4, irritation = ?5, reliability_expectation = ?6,
             current_event_id = ?7, updated_at = ?8 WHERE id = ?9 AND agent_id = ?10",
            params![
                resulting.familiarity,
                resulting.trust,
                resulting.affinity,
                resulting.admiration,
                resulting.irritation,
                resulting.reliability_expectation,
                event.id,
                now,
                relationship_id,
                request.agent_id
            ],
        )?;
        let relationship = load_relationship_tx(&tx, &request.agent_id, &relationship_id)?;
        tx.commit()?;
        Ok(relationship)
    }

    pub fn reset_relationship(
        &self,
        agent_id: &str,
        relationship_id: &str,
        reason: &str,
        idempotency_key: &str,
    ) -> Result<RelationshipState, DatabaseError> {
        let reason = text(reason, MAX_REASON, "invalid_reason")?;
        let idempotency_key = idempotency(idempotency_key)?;
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, agent_id)?;
        let prior = load_relationship_tx(&tx, agent_id, relationship_id)?;
        let payload = json!({ "relationshipId": relationship_id, "reason": reason });
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, agent_id, &idempotency_key)?,
            "relationship_reset",
            &payload,
        )? {
            let relationship_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let relationship = load_relationship_tx(&tx, agent_id, &relationship_id)?;
            tx.commit()?;
            return Ok(relationship);
        }
        let now = now_millis();
        let resulting = RelationshipValues::default();
        let event = core_event(
            &tx,
            agent_id,
            &owner_id,
            &idempotency_key,
            "relationship_reset",
            "relationship",
            relationship_id,
            "owner_testimony",
            Some("owner"),
            &reason,
            1.0,
            &payload,
            Some(relationship_id),
            None,
            now,
        )?;
        let deltas = RelationshipDeltas {
            familiarity: resulting.familiarity - prior.values.familiarity,
            trust: resulting.trust - prior.values.trust,
            affinity: resulting.affinity - prior.values.affinity,
            admiration: resulting.admiration - prior.values.admiration,
            irritation: resulting.irritation - prior.values.irritation,
            reliability_expectation: resulting.reliability_expectation
                - prior.values.reliability_expectation,
        };
        tx.execute(
            "INSERT INTO relationship_events
             (id, relationship_id, agent_id, owner_user_id, event_id, delta_json, prior_json,
              resulting_json, source_kind, source_reference, confidence, reason, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'owner_testimony', 'owner', 1.0, ?9, 'applied', ?10)",
            params![
                Uuid::now_v7().to_string(),
                relationship_id,
                agent_id,
                owner_id,
                event.id,
                serde_json::to_string(&deltas)
                    .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?,
                serde_json::to_string(&prior.values)
                    .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?,
                serde_json::to_string(&resulting)
                    .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?,
                reason,
                now
            ],
        )?;
        tx.execute(
            "UPDATE relationships SET familiarity = ?1, trust = ?2, affinity = ?3,
             admiration = ?4, irritation = ?5, reliability_expectation = ?6,
             current_event_id = ?7, updated_at = ?8 WHERE id = ?9 AND agent_id = ?10",
            params![
                resulting.familiarity,
                resulting.trust,
                resulting.affinity,
                resulting.admiration,
                resulting.irritation,
                resulting.reliability_expectation,
                event.id,
                now,
                relationship_id,
                agent_id
            ],
        )?;
        let relationship = load_relationship_tx(&tx, agent_id, relationship_id)?;
        tx.commit()?;
        Ok(relationship)
    }

    pub fn rollback_relationship_event(
        &self,
        agent_id: &str,
        event_id: &str,
        idempotency_key: &str,
    ) -> Result<RelationshipState, DatabaseError> {
        let event_id = reference(event_id, 128, "event_not_found")?;
        let idempotency_key = idempotency(idempotency_key)?;
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, agent_id)?;
        let event_agent: Option<String> = tx
            .query_row(
                "SELECT agent_id FROM relationship_events WHERE event_id = ?1",
                params![event_id],
                |row| row.get(0),
            )
            .optional()?;
        if event_agent
            .as_deref()
            .is_some_and(|owner| owner != agent_id)
        {
            return Err(DatabaseError::OwnershipMismatch);
        }
        let (relationship_id, subject_type, subject_ref, prior_json, target_status): (
            String,
            String,
            String,
            String,
            String,
        ) = tx
            .query_row(
                "SELECT re.relationship_id, r.subject_type, r.subject_ref, re.prior_json, re.status
                 FROM relationship_events re
                 JOIN relationships r ON r.id = re.relationship_id AND r.agent_id = re.agent_id
                 WHERE re.event_id = ?1 AND re.agent_id = ?2",
                params![event_id, agent_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or(DatabaseError::Cognitive("event_not_found"))?;
        let resulting: RelationshipValues = serde_json::from_str(&prior_json)
            .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?;
        let current = load_relationship_tx(&tx, agent_id, &relationship_id)?;
        let payload = json!({
            "relationshipId": relationship_id,
            "rollbackOf": event_id,
            "resulting": resulting,
        });
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, agent_id, &idempotency_key)?,
            "relationship_reset",
            &payload,
        )? {
            let relationship_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let relationship = load_relationship_tx(&tx, agent_id, &relationship_id)?;
            tx.commit()?;
            return Ok(relationship);
        }
        if target_status != "applied" {
            return Err(DatabaseError::Cognitive("rollback_not_allowed"));
        }
        let latest_event: String = tx.query_row(
            "SELECT event_id FROM relationship_events
             WHERE relationship_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
            params![relationship_id],
            |row| row.get(0),
        )?;
        if latest_event != event_id {
            return Err(DatabaseError::Cognitive("rollback_conflict"));
        }
        let deltas = RelationshipDeltas {
            familiarity: resulting.familiarity - current.values.familiarity,
            trust: resulting.trust - current.values.trust,
            affinity: resulting.affinity - current.values.affinity,
            admiration: resulting.admiration - current.values.admiration,
            irritation: resulting.irritation - current.values.irritation,
            reliability_expectation: resulting.reliability_expectation
                - current.values.reliability_expectation,
        };
        let now = now_millis();
        let event = core_event(
            &tx,
            agent_id,
            &owner_id,
            &idempotency_key,
            "relationship_reset",
            &subject_type,
            &subject_ref,
            "owner_testimony",
            Some("owner"),
            "Reversão solicitada pelo Owner",
            1.0,
            &payload,
            Some(&relationship_id),
            Some(&event_id),
            now,
        )?;
        tx.execute(
            "UPDATE relationship_events SET status = 'rolled_back' WHERE event_id = ?1 AND agent_id = ?2",
            params![event_id, agent_id],
        )?;
        tx.execute(
            "UPDATE cognitive_core_events SET status = 'rolled_back' WHERE id = ?1 AND agent_id = ?2",
            params![event_id, agent_id],
        )?;
        tx.execute(
            "INSERT INTO relationship_events
             (id, relationship_id, agent_id, owner_user_id, event_id, delta_json, prior_json,
              resulting_json, source_kind, source_reference, confidence, reason, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'owner_testimony', 'owner', 1.0,
                     'Reversão solicitada pelo Owner', 'applied', ?9)",
            params![
                Uuid::now_v7().to_string(),
                relationship_id,
                agent_id,
                owner_id,
                event.id,
                serde_json::to_string(&deltas)
                    .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?,
                serde_json::to_string(&current.values)
                    .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?,
                serde_json::to_string(&resulting)
                    .map_err(|_| DatabaseError::Cognitive("persistence_failed"))?,
                now
            ],
        )?;
        tx.execute(
            "UPDATE relationships SET familiarity = ?1, trust = ?2, affinity = ?3,
             admiration = ?4, irritation = ?5, reliability_expectation = ?6,
             current_event_id = ?7, updated_at = ?8 WHERE id = ?9 AND agent_id = ?10",
            params![
                resulting.familiarity,
                resulting.trust,
                resulting.affinity,
                resulting.admiration,
                resulting.irritation,
                resulting.reliability_expectation,
                event.id,
                now,
                relationship_id,
                agent_id
            ],
        )?;
        let relationship = load_relationship_tx(&tx, agent_id, &relationship_id)?;
        tx.commit()?;
        Ok(relationship)
    }

    pub fn list_cognitive_goals(
        &self,
        agent_id: &str,
    ) -> Result<Vec<CognitiveGoal>, DatabaseError> {
        let connection = self.open()?;
        ensure_owned(&connection, agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, agent_id, title, description, origin, fictional_only, priority, status,
                    budget_units, due_at, expires_at, completion_evidence, parent_goal_id,
                    created_at, updated_at
             FROM goals WHERE agent_id = ?1 ORDER BY priority DESC, updated_at DESC, id DESC",
        )?;
        let goals = statement
            .query_map(params![agent_id], map_goal)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into);
        goals
    }

    pub fn create_owner_goal(&self, request: GoalRequest) -> Result<CognitiveGoal, DatabaseError> {
        self.create_goal(request, "owner", "active")
    }

    pub fn propose_agent_goal(&self, request: GoalRequest) -> Result<CognitiveGoal, DatabaseError> {
        self.create_goal(request, "agent_proposal", "proposed")
    }

    fn create_goal(
        &self,
        request: GoalRequest,
        origin: &str,
        initial_status: &str,
    ) -> Result<CognitiveGoal, DatabaseError> {
        let title = text(&request.title, 160, "invalid_goal")?;
        let description = text(&request.description, 1000, "invalid_goal")?;
        if unsafe_external_goal_language(&title) || unsafe_external_goal_language(&description) {
            return Err(DatabaseError::Cognitive("external_action_blocked"));
        }
        if !(0..=100).contains(&request.priority) || !(1..=1000).contains(&request.budget_units) {
            return Err(DatabaseError::Cognitive("invalid_goal_budget"));
        }
        if let (Some(due_at), Some(expires_at)) = (request.due_at, request.expires_at) {
            if due_at > expires_at {
                return Err(DatabaseError::Cognitive("invalid_goal_schedule"));
            }
        }
        let idempotency_key = idempotency(&request.idempotency_key)?;
        let parent_goal_id = request
            .parent_goal_id
            .as_deref()
            .map(|value| reference(value, 128, "invalid_goal"))
            .transpose()?;
        let payload = json!({
            "title": title,
            "description": description,
            "origin": origin,
            "fictionalOnly": true,
            "priority": request.priority,
            "budgetUnits": request.budget_units,
            "dueAt": request.due_at,
            "expiresAt": request.expires_at,
            "parentGoalId": parent_goal_id,
        });
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, &request.agent_id)?;
        if let Some(parent_goal_id) = parent_goal_id.as_deref() {
            let parent_agent: Option<String> = tx
                .query_row(
                    "SELECT agent_id FROM goals WHERE id = ?1",
                    params![parent_goal_id],
                    |row| row.get(0),
                )
                .optional()?;
            if parent_agent.as_deref() != Some(request.agent_id.as_str()) {
                return Err(DatabaseError::OwnershipMismatch);
            }
            if origin == "agent_proposal"
                && tx.query_row(
                    "SELECT status FROM goals WHERE id = ?1",
                    params![parent_goal_id],
                    |row| row.get::<_, String>(0),
                )? == "proposed"
            {
                return Err(DatabaseError::Cognitive("goal_loop_blocked"));
            }
        }
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, &request.agent_id, &idempotency_key)?,
            "goal_create",
            &payload,
        )? {
            let goal_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let goal = load_goal_tx(&tx, &request.agent_id, &goal_id)?;
            tx.commit()?;
            return Ok(goal);
        }
        let goal_id = Uuid::now_v7().to_string();
        let now = now_millis();
        tx.execute(
            "INSERT INTO goals
             (id, agent_id, owner_user_id, title, description, origin, fictional_only, priority,
              status, budget_units, due_at, expires_at, parent_goal_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
            params![
                goal_id,
                request.agent_id,
                owner_id,
                title,
                description,
                origin,
                request.priority,
                initial_status,
                request.budget_units,
                request.due_at,
                request.expires_at,
                parent_goal_id,
                now
            ],
        )?;
        let event = core_event(
            &tx,
            &request.agent_id,
            &owner_id,
            &idempotency_key,
            "goal_create",
            "goal",
            &goal_id,
            "owner_testimony",
            Some("owner"),
            &title,
            1.0,
            &payload,
            Some(&goal_id),
            None,
            now,
        )?;
        tx.execute(
            "UPDATE goals SET current_event_id = ?1 WHERE id = ?2 AND agent_id = ?3",
            params![event.id, goal_id, request.agent_id],
        )?;
        let goal = load_goal_tx(&tx, &request.agent_id, &goal_id)?;
        tx.commit()?;
        Ok(goal)
    }

    pub fn approve_cognitive_goal(
        &self,
        agent_id: &str,
        goal_id: &str,
        idempotency_key: &str,
    ) -> Result<CognitiveGoal, DatabaseError> {
        self.update_goal_status(agent_id, goal_id, "active", None, idempotency_key)
    }

    pub fn update_goal_status(
        &self,
        agent_id: &str,
        goal_id: &str,
        status: &str,
        completion_evidence: Option<&str>,
        idempotency_key: &str,
    ) -> Result<CognitiveGoal, DatabaseError> {
        let status = text(status, 32, "invalid_status")?;
        if !matches!(
            status.as_str(),
            "active" | "suspended" | "completed" | "cancelled" | "archived" | "rejected"
        ) {
            return Err(DatabaseError::Cognitive("invalid_status"));
        }
        let idempotency_key = idempotency(idempotency_key)?;
        let completion_evidence = completion_evidence
            .map(|value| text(value, 500, "invalid_goal"))
            .transpose()?;
        if completion_evidence
            .as_deref()
            .is_some_and(unsafe_external_goal_language)
        {
            return Err(DatabaseError::Cognitive("external_action_blocked"));
        }
        let mut connection = self.open()?;
        let tx = connection.transaction()?;
        let owner_id = ensure_owned_tx(&tx, agent_id)?;
        let prior = load_goal_tx(&tx, agent_id, goal_id)?;
        if prior.origin == "agent_proposal"
            && prior.status == "proposed"
            && !matches!(status.as_str(), "active" | "rejected")
        {
            return Err(DatabaseError::Cognitive("invalid_transition"));
        }
        if matches!(
            prior.status.as_str(),
            "completed" | "cancelled" | "rejected"
        ) && status != "archived"
        {
            return Err(DatabaseError::Cognitive("invalid_transition"));
        }
        if prior.status == "archived" || (prior.status == "suspended" && status == "suspended") {
            return Err(DatabaseError::Cognitive("invalid_transition"));
        }
        let payload = json!({
            "goalId": goal_id,
            "priorStatus": prior.status,
            "status": status,
            "completionEvidence": completion_evidence,
        });
        if let Some(existing) = idempotent_or_conflict(
            existing_core_event(&tx, agent_id, &idempotency_key)?,
            "goal_status",
            &payload,
        )? {
            let goal_id = existing
                .result_ref
                .ok_or(DatabaseError::Cognitive("persistence_failed"))?;
            let goal = load_goal_tx(&tx, agent_id, &goal_id)?;
            tx.commit()?;
            return Ok(goal);
        }
        let now = now_millis();
        let event = core_event(
            &tx,
            agent_id,
            &owner_id,
            &idempotency_key,
            "goal_status",
            "goal",
            goal_id,
            "owner_testimony",
            Some("owner"),
            "Atualização de objetivo fictício",
            1.0,
            &payload,
            Some(goal_id),
            None,
            now,
        )?;
        tx.execute(
            "UPDATE goals SET status = ?1, completion_evidence = ?2, current_event_id = ?3,
             updated_at = ?4 WHERE id = ?5 AND agent_id = ?6",
            params![
                status,
                completion_evidence,
                event.id,
                now,
                goal_id,
                agent_id
            ],
        )?;
        let goal = load_goal_tx(&tx, agent_id, goal_id)?;
        tx.commit()?;
        Ok(goal)
    }
}
