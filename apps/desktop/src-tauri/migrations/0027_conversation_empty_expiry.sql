PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

-- The migration runner adds this column when upgrading a legacy database.

CREATE INDEX IF NOT EXISTS conversations_empty_expiry
ON conversations(empty_expires_at)
WHERE empty_expires_at IS NOT NULL;

INSERT OR IGNORE INTO schema_migrations (version, applied_at)
VALUES (27, unixepoch('subsec') * 1000);

COMMIT;
PRAGMA foreign_keys = ON;
