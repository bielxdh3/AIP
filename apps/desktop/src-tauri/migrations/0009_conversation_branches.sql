PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE conversation_branches (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  parent_branch_id TEXT REFERENCES conversation_branches(id) ON DELETE SET NULL,
  parent_message_id TEXT REFERENCES conversation_messages(id) ON DELETE SET NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(id, conversation_id, agent_id)
);

ALTER TABLE conversation_messages ADD COLUMN branch_id TEXT;

INSERT INTO conversation_branches (id, conversation_id, agent_id, created_at, updated_at)
SELECT id || ':main', id, agent_id, created_at, updated_at FROM conversations;

UPDATE conversation_messages
SET branch_id = conversation_id || ':main'
WHERE branch_id IS NULL;

CREATE TABLE conversation_active_branches (
  conversation_id TEXT PRIMARY KEY REFERENCES conversations(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  branch_id TEXT NOT NULL REFERENCES conversation_branches(id) ON DELETE RESTRICT,
  updated_at INTEGER NOT NULL
);

INSERT INTO conversation_active_branches (conversation_id, agent_id, branch_id, updated_at)
SELECT id, agent_id, id || ':main', updated_at FROM conversations;

CREATE INDEX conversation_messages_branch_order
ON conversation_messages(conversation_id, agent_id, branch_id, created_at, id);

INSERT INTO schema_migrations (version, applied_at)
VALUES (9, unixepoch('subsec') * 1000);

COMMIT;
