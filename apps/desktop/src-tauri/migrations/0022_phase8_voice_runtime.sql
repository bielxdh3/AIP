PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE voice_operation_records (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  operation TEXT NOT NULL CHECK (
    operation IN ('transcription', 'synthesis', 'wake_word')
  ),
  status TEXT NOT NULL CHECK (
    status IN ('started', 'running', 'completed', 'cancelled', 'degraded', 'failed')
  ),
  code TEXT,
  provider_ref TEXT CHECK (
    provider_ref IS NULL OR length(provider_ref) BETWEEN 1 AND 160
  ),
  duration_ms INTEGER CHECK (
    duration_ms IS NULL OR duration_ms BETWEEN 0 AND 30000
  ),
  idempotency_key TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  completed_at INTEGER,
  updated_at INTEGER NOT NULL,
  UNIQUE (agent_id, idempotency_key)
);

CREATE INDEX voice_operation_records_agent_updated
  ON voice_operation_records(agent_id, updated_at DESC, id);

INSERT INTO schema_migrations (version, applied_at)
VALUES (22, unixepoch('subsec') * 1000);
COMMIT;
