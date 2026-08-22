PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

DROP INDEX IF EXISTS tool_actions_session_status;
DROP INDEX IF EXISTS tool_audit_log_agent_created;
ALTER TABLE tool_session_permissions RENAME TO tool_session_permissions_old;
ALTER TABLE tool_actions RENAME TO tool_actions_old;
ALTER TABLE tool_catalog RENAME TO tool_catalog_old;
ALTER TABLE tool_audit_log RENAME TO tool_audit_log_old;

CREATE TABLE tool_catalog (
  tool_id TEXT PRIMARY KEY,
  manifest_version INTEGER NOT NULL CHECK (manifest_version = 1),
  name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
  classification TEXT NOT NULL CHECK (classification IN ('read_only', 'state_changing')),
  adapter_kind TEXT NOT NULL CHECK (
    adapter_kind IN ('workspace_mock', 'workspace_local', 'calendar_mock', 'messaging_mock')
  ),
  scope_kind TEXT NOT NULL CHECK (
    scope_kind IN ('workspace', 'workspace_root', 'calendar', 'messaging')
  ),
  requires_second_confirmation INTEGER NOT NULL CHECK (requires_second_confirmation IN (0, 1)),
  capabilities_json TEXT NOT NULL CHECK (length(capabilities_json) BETWEEN 2 AND 2048),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
INSERT INTO tool_catalog SELECT * FROM tool_catalog_old;
INSERT OR IGNORE INTO tool_catalog (
  tool_id, manifest_version, name, classification, adapter_kind, scope_kind,
  requires_second_confirmation, capabilities_json, created_at, updated_at
)
VALUES
  ('workspace.inspect_local', 1, 'Workspace local inspection', 'read_only',
   'workspace_local', 'workspace_root', 0, '["inspect_scope"]',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000),
  ('workspace.organize_local', 1, 'Workspace local organization', 'state_changing',
   'workspace_local', 'workspace_root', 1, '["preview","organize","compensate"]',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000);

CREATE TABLE tool_session_permissions (
  session_id TEXT NOT NULL REFERENCES tool_sessions(id) ON DELETE CASCADE,
  tool_id TEXT NOT NULL REFERENCES tool_catalog(tool_id) ON DELETE RESTRICT,
  permission TEXT NOT NULL CHECK (
    permission IN ('preview', 'execute_read_only', 'execute_state_changing')
  ),
  PRIMARY KEY (session_id, tool_id, permission)
);
INSERT INTO tool_session_permissions SELECT * FROM tool_session_permissions_old;

CREATE TABLE tool_actions (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES tool_sessions(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  tool_id TEXT NOT NULL REFERENCES tool_catalog(tool_id) ON DELETE RESTRICT,
  classification TEXT NOT NULL CHECK (classification IN ('read_only', 'state_changing')),
  input_json TEXT NOT NULL CHECK (length(input_json) BETWEEN 2 AND 8192),
  summary TEXT NOT NULL CHECK (length(summary) BETWEEN 1 AND 512),
  affected_resources_json TEXT NOT NULL CHECK (length(affected_resources_json) BETWEEN 2 AND 4096),
  exact_effect TEXT NOT NULL CHECK (length(exact_effect) BETWEEN 1 AND 1024),
  status TEXT NOT NULL CHECK (
    status IN ('previewed', 'approved', 'confirmed', 'dry_run', 'executed',
               'cancelled', 'failed', 'compensated', 'rejected')
  ),
  dry_run INTEGER NOT NULL CHECK (dry_run IN (0, 1)),
  requires_second_confirmation INTEGER NOT NULL CHECK (requires_second_confirmation IN (0, 1)),
  owner_approved INTEGER NOT NULL CHECK (owner_approved IN (0, 1)),
  second_confirmed INTEGER NOT NULL CHECK (second_confirmed IN (0, 1)),
  result_json TEXT CHECK (result_json IS NULL OR length(result_json) BETWEEN 2 AND 4096),
  compensation_json TEXT CHECK (compensation_json IS NULL OR length(compensation_json) BETWEEN 2 AND 2048),
  error_code TEXT CHECK (error_code IS NULL OR length(error_code) BETWEEN 1 AND 96),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  approved_at INTEGER,
  confirmed_at INTEGER,
  executed_at INTEGER,
  UNIQUE (session_id, idempotency_key)
);
INSERT INTO tool_actions SELECT * FROM tool_actions_old;
CREATE INDEX tool_actions_session_status ON tool_actions(session_id, status, updated_at DESC);

CREATE TABLE tool_audit_log (
  id TEXT PRIMARY KEY,
  action_id TEXT REFERENCES tool_actions(id) ON DELETE SET NULL,
  session_id TEXT REFERENCES tool_sessions(id) ON DELETE SET NULL,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  tool_id TEXT,
  event TEXT NOT NULL CHECK (length(event) BETWEEN 1 AND 64),
  result TEXT NOT NULL CHECK (length(result) BETWEEN 1 AND 64),
  code TEXT CHECK (code IS NULL OR length(code) BETWEEN 1 AND 96),
  details_json TEXT NOT NULL CHECK (length(details_json) BETWEEN 2 AND 4096),
  created_at INTEGER NOT NULL
);
CREATE INDEX tool_audit_log_agent_created ON tool_audit_log(agent_id, created_at DESC, id);
INSERT INTO tool_audit_log SELECT * FROM tool_audit_log_old;

DROP TABLE tool_session_permissions_old;
DROP TABLE tool_actions_old;
DROP TABLE tool_catalog_old;
DROP TABLE tool_audit_log_old;

CREATE TABLE IF NOT EXISTS workspace_roots (
  id TEXT PRIMARY KEY CHECK (length(id) BETWEEN 1 AND 64),
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  path TEXT NOT NULL CHECK (length(path) BETWEEN 1 AND 512),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (owner_user_id, idempotency_key)
);
CREATE INDEX IF NOT EXISTS workspace_roots_owner_enabled
  ON workspace_roots(owner_user_id, enabled, updated_at DESC, id);

INSERT INTO schema_migrations (version, applied_at)
VALUES (23, unixepoch('subsec') * 1000);
COMMIT;
PRAGMA foreign_keys = ON;
