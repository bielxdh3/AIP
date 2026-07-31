PRAGMA foreign_keys = ON;

BEGIN IMMEDIATE;

CREATE TABLE conversations (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  title TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind = 'normal'),
  is_main INTEGER NOT NULL CHECK (is_main IN (0, 1)),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  archived_at INTEGER,
  UNIQUE (id, agent_id)
);

CREATE UNIQUE INDEX conversations_one_active_main_per_agent
ON conversations(agent_id)
WHERE is_main = 1 AND archived_at IS NULL;

CREATE TABLE conversation_messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  author_type TEXT NOT NULL CHECK (author_type IN ('user', 'agent', 'system')),
  content TEXT NOT NULL,
  actual_model_ref TEXT,
  status TEXT NOT NULL CHECK (status IN ('pending', 'streaming', 'complete', 'failed', 'cancelled')),
  generation_request_id TEXT UNIQUE,
  created_at INTEGER NOT NULL,
  completed_at INTEGER,
  terminal_error_code TEXT,
  FOREIGN KEY (conversation_id, agent_id)
    REFERENCES conversations(id, agent_id) ON DELETE CASCADE
);

CREATE INDEX conversation_messages_order
ON conversation_messages(conversation_id, created_at, id);

INSERT INTO schema_migrations (version, applied_at)
VALUES (2, unixepoch('subsec') * 1000);

COMMIT;
