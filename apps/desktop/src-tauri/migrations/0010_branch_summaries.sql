PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

ALTER TABLE conversation_summaries ADD COLUMN branch_id TEXT;

-- Pre-branch summaries can mix abandoned alternatives. Retire them rather than
-- assigning an unverifiable branch; future completions regenerate them safely.
UPDATE conversation_summaries
SET superseded_at = COALESCE(superseded_at, unixepoch('subsec') * 1000)
WHERE branch_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_conversation_summaries_branch
  ON conversation_summaries (agent_id, conversation_id, branch_id, superseded_at, created_at);

INSERT INTO schema_migrations (version, applied_at) VALUES (10, unixepoch('subsec') * 1000);

COMMIT;
