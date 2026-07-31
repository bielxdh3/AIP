PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE cognitive_events (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id),
  owner_user_id TEXT NOT NULL REFERENCES users(id),
  idempotency_key TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('trait_delta', 'owner_correction', 'rollback')),
  trait_key TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  reason TEXT NOT NULL,
  confidence REAL NOT NULL CHECK(confidence >= 0.0 AND confidence <= 1.0),
  requested_value REAL NOT NULL,
  prior_value REAL NOT NULL CHECK(prior_value >= 0.0 AND prior_value <= 1.0),
  resulting_value REAL NOT NULL CHECK(resulting_value >= 0.0 AND resulting_value <= 1.0),
  status TEXT NOT NULL CHECK(status IN ('applied', 'rejected', 'rolled_back')),
  policy_version INTEGER NOT NULL,
  schema_version INTEGER NOT NULL,
  rollback_of_event_id TEXT REFERENCES cognitive_events(id),
  created_at INTEGER NOT NULL,
  terminal_at INTEGER NOT NULL,
  UNIQUE(agent_id, idempotency_key)
);
CREATE INDEX cognitive_events_agent_created ON cognitive_events(agent_id, created_at, id);
CREATE INDEX cognitive_events_agent_trait_created ON cognitive_events(agent_id, trait_key, created_at, id);

CREATE TABLE cognitive_processing_checkpoints (
  agent_id TEXT NOT NULL REFERENCES agents(id),
  processor_key TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  event_id TEXT NOT NULL REFERENCES cognitive_events(id),
  terminal_status TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(agent_id, processor_key, idempotency_key)
);

INSERT INTO schema_migrations (version, applied_at) VALUES (12, unixepoch('subsec') * 1000);
COMMIT;
