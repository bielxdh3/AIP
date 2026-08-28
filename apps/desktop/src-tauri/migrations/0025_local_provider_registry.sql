PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS local_providers (
  id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 96),
  kind TEXT NOT NULL CHECK(kind IN ('stt','tts','visual')),
  display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 120),
  executable_path TEXT NOT NULL CHECK(length(executable_path) BETWEEN 1 AND 1024),
  protocol_version TEXT NOT NULL CHECK(length(protocol_version) BETWEEN 1 AND 64),
  enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
  validation_status TEXT NOT NULL CHECK(validation_status IN ('pending','ready','unavailable','invalid')),
  validation_result TEXT NOT NULL CHECK(length(validation_result) BETWEEN 1 AND 256),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS local_providers_kind_enabled ON local_providers(kind, enabled, updated_at DESC);
INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(25, unixepoch('subsec')*1000);
COMMIT;
