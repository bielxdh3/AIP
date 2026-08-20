PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE agent_voice_settings (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  base_voice_id TEXT NOT NULL CHECK (base_voice_id = 'aip-base-v1'),
  custom_voice_ref TEXT CHECK (
    custom_voice_ref IS NULL OR length(custom_voice_ref) BETWEEN 1 AND 160
  ),
  custom_voice_consent TEXT NOT NULL CHECK (
    custom_voice_consent IN ('not_granted', 'granted', 'revoked')
  ),
  recognition_model_ref TEXT CHECK (
    recognition_model_ref IS NULL OR length(recognition_model_ref) BETWEEN 1 AND 160
  ),
  synthesis_model_ref TEXT CHECK (
    synthesis_model_ref IS NULL OR length(synthesis_model_ref) BETWEEN 1 AND 160
  ),
  input_device_ref TEXT CHECK (
    input_device_ref IS NULL OR length(input_device_ref) BETWEEN 1 AND 160
  ),
  output_device_ref TEXT CHECK (
    output_device_ref IS NULL OR length(output_device_ref) BETWEEN 1 AND 160
  ),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  CHECK (
    (custom_voice_consent = 'granted' AND custom_voice_ref IS NOT NULL)
    OR (custom_voice_consent <> 'granted' AND custom_voice_ref IS NULL)
  )
);

CREATE TABLE voice_mutation_events (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  operation TEXT NOT NULL CHECK (operation IN ('settings', 'custom_consent')),
  idempotency_key TEXT NOT NULL,
  request_json TEXT NOT NULL CHECK (length(request_json) <= 4096),
  result_json TEXT NOT NULL CHECK (length(result_json) <= 8192),
  created_at INTEGER NOT NULL,
  UNIQUE (agent_id, idempotency_key)
);
CREATE INDEX voice_mutation_events_agent_created
  ON voice_mutation_events(agent_id, created_at DESC, id);

INSERT INTO schema_migrations (version, applied_at)
VALUES (15, unixepoch('subsec') * 1000);
COMMIT;
