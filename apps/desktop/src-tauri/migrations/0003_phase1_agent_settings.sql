PRAGMA foreign_keys = ON;

BEGIN IMMEDIATE;

CREATE TABLE agent_phase1_settings (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  selected_model_ref TEXT,
  keep_alive_minutes INTEGER NOT NULL DEFAULT 15 CHECK (keep_alive_minutes BETWEEN 0 AND 120),
  updated_at INTEGER NOT NULL
);

INSERT INTO agent_phase1_settings (agent_id, selected_model_ref, keep_alive_minutes, updated_at)
SELECT
  a.id,
  CASE
    WHEN json_valid(COALESCE(model.value_json, 'null'))
      AND json_type(model.value_json) = 'text'
    THEN json_extract(model.value_json, '$')
    ELSE NULL
  END,
  CASE
    WHEN json_valid(COALESCE(keep_alive.value_json, 'null'))
      AND json_type(keep_alive.value_json) = 'integer'
      AND json_extract(keep_alive.value_json, '$') BETWEEN 0 AND 120
    THEN json_extract(keep_alive.value_json, '$')
    ELSE 15
  END,
  unixepoch('subsec') * 1000
FROM agents a
LEFT JOIN app_settings model ON model.key = 'phase1_selected_model_ref'
LEFT JOIN app_settings keep_alive ON keep_alive.key = 'phase1_keep_alive_minutes';

INSERT INTO schema_migrations (version, applied_at)
VALUES (3, unixepoch('subsec') * 1000);

COMMIT;
