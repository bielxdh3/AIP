PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

UPDATE agent_simulated_states
SET mode = 'normal', updated_at = unixepoch('subsec') * 1000
WHERE mode = 'safe';

INSERT INTO schema_migrations (version, applied_at)
VALUES (8, unixepoch('subsec') * 1000);

COMMIT;
