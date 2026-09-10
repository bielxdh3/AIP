PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

INSERT INTO schema_migrations (version, applied_at)
VALUES (29, unixepoch('subsec') * 1000);

COMMIT;
