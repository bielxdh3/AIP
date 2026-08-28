PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

-- SQLite has no ALTER TABLE ... ADD COLUMN IF NOT EXISTS. The migration runner
-- ensures this column before executing this rerunnable migration body.

CREATE INDEX IF NOT EXISTS conversations_agent_order
ON conversations(agent_id, is_pinned DESC, archived_at, updated_at DESC, id ASC);

-- Legacy main rows remain intact, including their IDs and messages. They are now
-- ordinary conversations; the old flag is retained only for forward compatibility.
UPDATE conversations SET is_main = 0 WHERE is_main = 1;

INSERT OR IGNORE INTO schema_migrations (version, applied_at)
VALUES (26, unixepoch('subsec') * 1000);
COMMIT;
