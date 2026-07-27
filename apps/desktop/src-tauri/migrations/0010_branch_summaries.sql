ALTER TABLE conversation_summaries ADD COLUMN branch_id TEXT;

UPDATE conversation_summaries
SET branch_id = conversation_id || ':main'
WHERE branch_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_conversation_summaries_branch
  ON conversation_summaries (agent_id, conversation_id, branch_id, superseded_at, created_at);

INSERT INTO schema_migrations (version, applied_at) VALUES (10, unixepoch('subsec') * 1000);
