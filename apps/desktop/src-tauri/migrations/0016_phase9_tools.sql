PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE tool_catalog (
  tool_id TEXT PRIMARY KEY,
  manifest_version INTEGER NOT NULL CHECK (manifest_version = 1),
  name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
  classification TEXT NOT NULL CHECK (classification IN ('read_only', 'state_changing')),
  adapter_kind TEXT NOT NULL CHECK (
    adapter_kind IN ('workspace_mock', 'calendar_mock', 'messaging_mock')
  ),
  scope_kind TEXT NOT NULL CHECK (scope_kind IN ('workspace', 'calendar', 'messaging')),
  requires_second_confirmation INTEGER NOT NULL CHECK (requires_second_confirmation IN (0, 1)),
  capabilities_json TEXT NOT NULL CHECK (length(capabilities_json) BETWEEN 2 AND 2048),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE tool_sessions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  scope_ref TEXT NOT NULL CHECK (length(scope_ref) BETWEEN 1 AND 96),
  status TEXT NOT NULL CHECK (status IN ('active', 'cancelled', 'closed')),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 4096),
  temporary_chat INTEGER NOT NULL CHECK (temporary_chat = 0),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (owner_user_id, idempotency_key)
);
CREATE INDEX tool_sessions_agent_status
  ON tool_sessions(agent_id, status, updated_at DESC);

CREATE TABLE tool_session_permissions (
  session_id TEXT NOT NULL REFERENCES tool_sessions(id) ON DELETE CASCADE,
  tool_id TEXT NOT NULL REFERENCES tool_catalog(tool_id) ON DELETE RESTRICT,
  permission TEXT NOT NULL CHECK (
    permission IN ('preview', 'execute_read_only', 'execute_state_changing')
  ),
  PRIMARY KEY (session_id, tool_id, permission)
);

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
  compensation_json TEXT CHECK (
    compensation_json IS NULL OR length(compensation_json) BETWEEN 2 AND 2048
  ),
  error_code TEXT CHECK (error_code IS NULL OR length(error_code) BETWEEN 1 AND 96),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  approved_at INTEGER,
  confirmed_at INTEGER,
  executed_at INTEGER,
  UNIQUE (session_id, idempotency_key)
);
CREATE INDEX tool_actions_session_status
  ON tool_actions(session_id, status, updated_at DESC);

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
CREATE INDEX tool_audit_log_agent_created
  ON tool_audit_log(agent_id, created_at DESC, id);

INSERT INTO tool_catalog (
  tool_id, manifest_version, name, classification, adapter_kind, scope_kind,
  requires_second_confirmation, capabilities_json, created_at, updated_at
)
VALUES
  ('workspace.inspect_scope', 1, 'Workspace fixture inspection', 'read_only',
   'workspace_mock', 'workspace', 0, '{"operations":["inspect_scope"]}',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000),
  ('workspace.organize_files', 1, 'Workspace fixture organization', 'state_changing',
   'workspace_mock', 'workspace', 0, '{"operations":["preview","organize","compensate"]}',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000),
  ('calendar.list_events', 1, 'Calendar fixture listing', 'read_only',
   'calendar_mock', 'calendar', 0, '{"operations":["list_events"]}',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000),
  ('calendar.create_event', 1, 'Calendar fixture event', 'state_changing',
   'calendar_mock', 'calendar', 1, '{"operations":["preview","create","compensate"]}',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000),
  ('messaging.preview_message', 1, 'Messaging fixture preview', 'read_only',
   'messaging_mock', 'messaging', 0, '{"operations":["preview_message"]}',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000),
  ('messaging.send_message', 1, 'Messaging fixture send', 'state_changing',
   'messaging_mock', 'messaging', 1, '{"operations":["preview","send","compensate"]}',
   unixepoch('subsec') * 1000, unixepoch('subsec') * 1000);

INSERT INTO schema_migrations (version, applied_at)
VALUES (16, unixepoch('subsec') * 1000);
COMMIT;
