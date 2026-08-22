PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS extension_executions (
  id TEXT PRIMARY KEY, owner_user_id TEXT NOT NULL REFERENCES users(id), agent_id TEXT NOT NULL REFERENCES agents(id),
  extension_id TEXT NOT NULL, revision INTEGER NOT NULL, package_hash TEXT NOT NULL CHECK(length(package_hash)=64 AND package_hash GLOB '[0-9a-f]*'),
  input TEXT NOT NULL CHECK(length(input)<=4096), status TEXT NOT NULL CHECK(status IN ('running','succeeded','failed','terminated','denied')),
  output TEXT CHECK(output IS NULL OR length(output)<=8192), error TEXT CHECK(error IS NULL OR length(error)<=512),
  steps INTEGER NOT NULL DEFAULT 0 CHECK(steps BETWEEN 0 AND 32), cancellation_requested INTEGER NOT NULL DEFAULT 0 CHECK(cancellation_requested IN (0,1)), created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
  FOREIGN KEY(extension_id, revision) REFERENCES extension_manifest_revisions(extension_id, revision)
);
CREATE TABLE IF NOT EXISTS extension_execution_idempotency (owner_user_id TEXT NOT NULL REFERENCES users(id), idempotency_key TEXT NOT NULL CHECK(length(idempotency_key) BETWEEN 1 AND 128), request_hash TEXT NOT NULL CHECK(length(request_hash)=64), result_json TEXT NOT NULL CHECK(length(result_json)<=32768), created_at INTEGER NOT NULL, PRIMARY KEY(owner_user_id,idempotency_key));
CREATE INDEX IF NOT EXISTS extension_executions_owner ON extension_executions(owner_user_id, created_at DESC);
INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(24, unixepoch('subsec')*1000);
COMMIT;
