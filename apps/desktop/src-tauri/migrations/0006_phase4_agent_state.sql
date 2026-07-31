PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE agent_simulated_states (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  sleep INTEGER NOT NULL CHECK (sleep BETWEEN 0 AND 100),
  energy INTEGER NOT NULL CHECK (energy BETWEEN 0 AND 100),
  mood INTEGER NOT NULL CHECK (mood BETWEEN 0 AND 100),
  focus INTEGER NOT NULL CHECK (focus BETWEEN 0 AND 100),
  curiosity INTEGER NOT NULL CHECK (curiosity BETWEEN 0 AND 100),
  social_fatigue INTEGER NOT NULL CHECK (social_fatigue BETWEEN 0 AND 100),
  mode TEXT NOT NULL CHECK (mode IN ('normal', 'voice_muted', 'silent', 'safe')),
  suspended INTEGER NOT NULL DEFAULT 0 CHECK (suspended IN (0, 1)),
  wake_now_until INTEGER,
  last_simulated_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO agent_simulated_states (agent_id, sleep, energy, mood, focus, curiosity, social_fatigue, mode, suspended, last_simulated_at, updated_at)
SELECT id, 20, 80, 70, 70, 70, 20, 'normal', 0, unixepoch('subsec') * 1000, unixepoch('subsec') * 1000 FROM agents;

INSERT INTO schema_migrations (version, applied_at) VALUES (6, unixepoch('subsec') * 1000);
COMMIT;
