PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE cognitive_core_events (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  idempotency_key TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN (
    'opinion_evidence', 'opinion_status', 'opinion_recalculate',
    'relationship_event', 'relationship_reset',
    'goal_create', 'goal_status', 'fictional_activity'
  )),
  subject_type TEXT NOT NULL,
  subject_ref TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_reference TEXT,
  reason TEXT NOT NULL,
  confidence REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
  payload_json TEXT NOT NULL CHECK (length(payload_json) <= 8192),
  status TEXT NOT NULL CHECK (status IN ('applied', 'rejected', 'superseded', 'rolled_back')),
  result_ref TEXT,
  related_event_id TEXT REFERENCES cognitive_core_events(id),
  created_at INTEGER NOT NULL,
  terminal_at INTEGER NOT NULL,
  UNIQUE (agent_id, idempotency_key)
);
CREATE INDEX cognitive_core_events_agent_created
  ON cognitive_core_events(agent_id, created_at DESC, id);
CREATE INDEX cognitive_core_events_subject
  ON cognitive_core_events(agent_id, subject_type, subject_ref, created_at DESC);

CREATE TABLE cognitive_core_checkpoints (
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  processor_key TEXT NOT NULL,
  source_key TEXT NOT NULL,
  event_id TEXT NOT NULL REFERENCES cognitive_core_events(id),
  terminal_status TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (agent_id, processor_key, source_key)
);

CREATE TABLE opinions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  subject_type TEXT NOT NULL,
  subject_ref TEXT NOT NULL,
  stance REAL NOT NULL CHECK (stance BETWEEN -1.0 AND 1.0),
  confidence REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
  status TEXT NOT NULL CHECK (status IN ('active', 'disputed', 'superseded', 'archived', 'rejected')),
  reason TEXT NOT NULL,
  current_event_id TEXT REFERENCES cognitive_core_events(id),
  supersedes_id TEXT REFERENCES opinions(id),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (agent_id, subject_type, subject_ref)
);
CREATE INDEX opinions_agent_status ON opinions(agent_id, status, updated_at DESC);

CREATE TABLE opinion_evidence (
  id TEXT PRIMARY KEY,
  opinion_id TEXT NOT NULL REFERENCES opinions(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('owner_testimony', 'model_inference', 'internet_information')),
  classification TEXT NOT NULL CHECK (classification IN ('verified_fact', 'reported_experience', 'impression')),
  stance REAL NOT NULL CHECK (stance BETWEEN -1.0 AND 1.0),
  claim_key TEXT NOT NULL,
  claim_value TEXT NOT NULL CHECK (length(claim_value) <= 500),
  source_reference TEXT,
  attribution TEXT,
  confidence REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
  status TEXT NOT NULL CHECK (status IN ('active', 'disputed', 'superseded', 'rejected')),
  event_id TEXT REFERENCES cognitive_core_events(id),
  created_at INTEGER NOT NULL
);
CREATE INDEX opinion_evidence_opinion_created ON opinion_evidence(opinion_id, created_at DESC, id);
CREATE INDEX opinion_evidence_source ON opinion_evidence(agent_id, source_kind, source_reference);

CREATE TABLE relationships (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  subject_type TEXT NOT NULL,
  subject_ref TEXT NOT NULL,
  familiarity REAL NOT NULL CHECK (familiarity BETWEEN 0.0 AND 1.0),
  trust REAL NOT NULL CHECK (trust BETWEEN 0.0 AND 1.0),
  affinity REAL NOT NULL CHECK (affinity BETWEEN 0.0 AND 1.0),
  admiration REAL NOT NULL CHECK (admiration BETWEEN 0.0 AND 1.0),
  irritation REAL NOT NULL CHECK (irritation BETWEEN 0.0 AND 1.0),
  reliability_expectation REAL NOT NULL CHECK (reliability_expectation BETWEEN 0.0 AND 1.0),
  current_event_id TEXT REFERENCES cognitive_core_events(id),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (agent_id, subject_type, subject_ref)
);
CREATE INDEX relationships_agent_updated ON relationships(agent_id, updated_at DESC);

CREATE TABLE relationship_events (
  id TEXT PRIMARY KEY,
  relationship_id TEXT NOT NULL REFERENCES relationships(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  event_id TEXT NOT NULL UNIQUE REFERENCES cognitive_core_events(id),
  delta_json TEXT NOT NULL CHECK (length(delta_json) <= 2048),
  prior_json TEXT NOT NULL CHECK (length(prior_json) <= 2048),
  resulting_json TEXT NOT NULL CHECK (length(resulting_json) <= 2048),
  source_kind TEXT NOT NULL,
  source_reference TEXT,
  confidence REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
  reason TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('applied', 'superseded', 'rolled_back')),
  created_at INTEGER NOT NULL
);
CREATE INDEX relationship_events_agent_created ON relationship_events(agent_id, created_at DESC, id);

CREATE TABLE goals (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  title TEXT NOT NULL CHECK (length(title) <= 160),
  description TEXT NOT NULL CHECK (length(description) <= 1000),
  origin TEXT NOT NULL CHECK (origin IN ('owner', 'agent_proposal')),
  fictional_only INTEGER NOT NULL CHECK (fictional_only IN (0, 1)),
  priority INTEGER NOT NULL CHECK (priority BETWEEN 0 AND 100),
  status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'suspended', 'completed', 'cancelled', 'archived', 'rejected')),
  budget_units INTEGER NOT NULL CHECK (budget_units BETWEEN 1 AND 1000),
  due_at INTEGER,
  expires_at INTEGER,
  completion_evidence TEXT CHECK (completion_evidence IS NULL OR length(completion_evidence) <= 500),
  parent_goal_id TEXT REFERENCES goals(id),
  current_event_id TEXT REFERENCES cognitive_core_events(id),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX goals_agent_status_priority ON goals(agent_id, status, priority DESC, updated_at DESC);

CREATE TABLE fictional_activities (
  id TEXT PRIMARY KEY,
  goal_id TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  activity_type TEXT NOT NULL CHECK (length(activity_type) <= 80),
  status TEXT NOT NULL CHECK (status IN ('active', 'paused', 'completed', 'expired', 'archived')),
  fictional_only INTEGER NOT NULL CHECK (fictional_only = 1),
  budget_units INTEGER NOT NULL CHECK (budget_units BETWEEN 1 AND 500),
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  created_at INTEGER NOT NULL
);
CREATE INDEX fictional_activities_agent_status ON fictional_activities(agent_id, status, created_at DESC);

INSERT INTO schema_migrations (version, applied_at)
VALUES (13, unixepoch('subsec') * 1000);
COMMIT;
