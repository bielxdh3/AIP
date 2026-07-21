PRAGMA foreign_keys = ON;

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  role TEXT NOT NULL CHECK (role = 'owner'),
  display_name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,
  profile_key TEXT NOT NULL UNIQUE CHECK (profile_key IN ('owner', 'companion')),
  sprite_key TEXT NOT NULL CHECK (sprite_key IN ('astra', 'luma')),
  status TEXT NOT NULL CHECK (status = 'active'),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_screen_preferences (
  agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  preferred_x REAL NOT NULL,
  preferred_y REAL NOT NULL,
  always_on_top INTEGER NOT NULL DEFAULT 1 CHECK (always_on_top IN (0, 1)),
  hide_fullscreen INTEGER NOT NULL DEFAULT 1 CHECK (hide_fullscreen IN (0, 1)),
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO schema_migrations (version, applied_at)
VALUES (1, unixepoch('subsec') * 1000);

COMMIT;
