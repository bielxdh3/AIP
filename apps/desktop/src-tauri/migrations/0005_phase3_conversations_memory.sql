PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE agent_phase3_settings (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  active_conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO agent_phase3_settings (agent_id, active_conversation_id, updated_at)
SELECT a.id,
       (SELECT c.id FROM conversations c WHERE c.agent_id = a.id AND c.is_main = 1 AND c.archived_at IS NULL LIMIT 1),
       unixepoch('subsec') * 1000
FROM agents a;

CREATE TABLE agent_memories (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  category TEXT NOT NULL,
  content TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('active', 'archived', 'trashed', 'candidate_rejected')),
  confirmation_status TEXT NOT NULL CHECK (confirmation_status IN ('confirmed', 'pending', 'rejected')),
  confidence REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
  importance INTEGER NOT NULL CHECK (importance BETWEEN 0 AND 100),
  source_type TEXT NOT NULL,
  source_message_id TEXT REFERENCES conversation_messages(id) ON DELETE SET NULL,
  source_conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
  conflict_key TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  archived_at INTEGER,
  trashed_at INTEGER
);
CREATE INDEX agent_memories_agent_search ON agent_memories(agent_id, status, confirmation_status, category, updated_at DESC);

CREATE TABLE conversation_summaries (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  through_message_id TEXT NOT NULL REFERENCES conversation_messages(id) ON DELETE RESTRICT,
  content TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  superseded_at INTEGER
);
CREATE INDEX conversation_summaries_current ON conversation_summaries(conversation_id, agent_id, superseded_at);

INSERT INTO schema_migrations (version, applied_at) VALUES (5, unixepoch('subsec') * 1000);
COMMIT;
