PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE agent_identity_profiles (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  birthday TEXT NOT NULL,
  fictive_age INTEGER NOT NULL CHECK (fictive_age BETWEEN 0 AND 10000),
  age_category TEXT NOT NULL,
  species TEXT NOT NULL,
  pronouns TEXT NOT NULL,
  personality_summary TEXT NOT NULL,
  traits_json TEXT NOT NULL,
  appearance_preset TEXT NOT NULL CHECK (appearance_preset IN ('astra', 'luma')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

ALTER TABLE conversations ADD COLUMN model_override_ref TEXT;

INSERT INTO agent_identity_profiles
 (agent_id,birthday,fictive_age,age_category,species,pronouns,personality_summary,traits_json,appearance_preset,created_at,updated_at)
SELECT id, '2000-01-01', 18, 'adult', 'agent', 'they/them', '', '{}', sprite_key, created_at, updated_at FROM agents;

INSERT OR IGNORE INTO app_settings (key, value_json, updated_at)
SELECT 'phase2_onboarding_complete',
       CASE WHEN EXISTS (SELECT 1 FROM agents) THEN 'true' ELSE 'false' END,
       unixepoch('subsec') * 1000;

INSERT INTO schema_migrations (version, applied_at) VALUES (4, unixepoch('subsec') * 1000);
COMMIT;
