PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

-- The migration runner adds missing columns before executing this rerunnable body.

INSERT OR IGNORE INTO schema_migrations (version, applied_at)
VALUES (28, unixepoch('subsec') * 1000);
COMMIT;
