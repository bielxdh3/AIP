PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE agent_conversation_policies (
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  purpose TEXT NOT NULL CHECK (length(purpose) BETWEEN 1 AND 160),
  opted_in INTEGER NOT NULL CHECK (opted_in IN (0, 1)),
  max_turns INTEGER NOT NULL CHECK (max_turns BETWEEN 1 AND 24),
  max_tokens INTEGER NOT NULL CHECK (max_tokens BETWEEN 64 AND 8192),
  max_duration_ms INTEGER NOT NULL CHECK (max_duration_ms BETWEEN 1000 AND 900000),
  max_repetitions INTEGER NOT NULL CHECK (max_repetitions BETWEEN 1 AND 3),
  resource_budget INTEGER NOT NULL CHECK (resource_budget BETWEEN 1 AND 100),
  revoked_at INTEGER,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (agent_id, purpose)
);

CREATE TABLE agent_conversations (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  initiator_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  participant_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  purpose TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'cancelled', 'suspended', 'rejected')),
  max_turns INTEGER NOT NULL CHECK (max_turns BETWEEN 1 AND 24),
  max_tokens INTEGER NOT NULL CHECK (max_tokens BETWEEN 64 AND 8192),
  max_duration_ms INTEGER NOT NULL CHECK (max_duration_ms BETWEEN 1000 AND 900000),
  max_repetitions INTEGER NOT NULL CHECK (max_repetitions BETWEEN 1 AND 3),
  resource_budget INTEGER NOT NULL CHECK (resource_budget BETWEEN 1 AND 100),
  turn_count INTEGER NOT NULL CHECK (turn_count >= 0),
  token_count INTEGER NOT NULL CHECK (token_count >= 0),
  loop_count INTEGER NOT NULL CHECK (loop_count >= 0),
  termination_reason TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  completed_at INTEGER,
  CHECK (initiator_agent_id <> participant_agent_id)
);
CREATE INDEX agent_conversations_agent_status
  ON agent_conversations(initiator_agent_id, participant_agent_id, status, created_at DESC);

CREATE TABLE agent_conversation_turns (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES agent_conversations(id) ON DELETE CASCADE,
  speaker_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  turn_index INTEGER NOT NULL CHECK (turn_index >= 0),
  content TEXT NOT NULL CHECK (length(content) BETWEEN 1 AND 4096),
  source_kind TEXT NOT NULL CHECK (source_kind IN ('owner', 'model_candidate')),
  created_at INTEGER NOT NULL,
  UNIQUE (conversation_id, turn_index)
);
CREATE INDEX agent_conversation_turns_order
  ON agent_conversation_turns(conversation_id, turn_index);

CREATE TABLE cognitive_candidates (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES agent_conversations(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  candidate_kind TEXT NOT NULL CHECK (candidate_kind IN ('opinion', 'relationship', 'goal')),
  candidate_json TEXT NOT NULL CHECK (length(candidate_json) <= 8192),
  source_reference TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'applied', 'rejected')),
  created_at INTEGER NOT NULL
);
CREATE INDEX cognitive_candidates_agent_status ON cognitive_candidates(agent_id, status, created_at DESC);

CREATE TABLE cognitive_resource_jobs (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  conversation_id TEXT REFERENCES agent_conversations(id) ON DELETE SET NULL,
  job_kind TEXT NOT NULL CHECK (job_kind IN ('heavy_generation', 'bounded_projection')),
  heavy INTEGER NOT NULL CHECK (heavy IN (0, 1)),
  priority INTEGER NOT NULL CHECK (priority BETWEEN 0 AND 100),
  budget_units INTEGER NOT NULL CHECK (budget_units BETWEEN 1 AND 100),
  status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'cancelled', 'failed')),
  error_code TEXT,
  created_at INTEGER NOT NULL,
  started_at INTEGER,
  ended_at INTEGER
);
CREATE INDEX cognitive_resource_jobs_status_priority
  ON cognitive_resource_jobs(status, priority DESC, created_at);
CREATE UNIQUE INDEX cognitive_resource_one_heavy_running
  ON cognitive_resource_jobs(heavy)
  WHERE heavy = 1 AND status = 'running';

INSERT INTO schema_migrations (version, applied_at)
VALUES (14, unixepoch('subsec') * 1000);
COMMIT;
