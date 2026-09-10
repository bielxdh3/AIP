PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

INSERT INTO schema_migrations (version, applied_at)
VALUES (30, unixepoch('subsec') * 1000);

COMMIT;
