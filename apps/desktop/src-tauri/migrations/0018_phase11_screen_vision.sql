PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE screen_vision_sessions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  monitor_id TEXT NOT NULL CHECK (length(monitor_id) BETWEEN 1 AND 64),
  fixture_id TEXT NOT NULL CHECK (length(fixture_id) BETWEEN 1 AND 160),
  status TEXT NOT NULL CHECK (status IN ('active', 'cancelled', 'closed')),
  max_jobs INTEGER NOT NULL CHECK (max_jobs BETWEEN 1 AND 8),
  max_duration_ms INTEGER NOT NULL CHECK (max_duration_ms BETWEEN 100 AND 15000),
  privacy_json TEXT NOT NULL CHECK (length(privacy_json) BETWEEN 2 AND 4096),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 16384),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  closed_at INTEGER,
  UNIQUE (owner_user_id, idempotency_key)
);
CREATE INDEX screen_vision_sessions_agent_status
  ON screen_vision_sessions(agent_id, status, updated_at DESC);

CREATE TABLE screen_vision_session_permissions (
  session_id TEXT NOT NULL REFERENCES screen_vision_sessions(id) ON DELETE CASCADE,
  permission TEXT NOT NULL CHECK (permission IN ('capture_fixture', 'analyze_fixture')),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (session_id, permission)
);

CREATE TABLE screen_vision_jobs (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES screen_vision_sessions(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  monitor_id TEXT NOT NULL CHECK (length(monitor_id) BETWEEN 1 AND 64),
  fixture_id TEXT NOT NULL CHECK (length(fixture_id) BETWEEN 1 AND 160),
  model_fixture_id TEXT NOT NULL CHECK (length(model_fixture_id) BETWEEN 1 AND 160),
  resource_key TEXT NOT NULL CHECK (resource_key = 'reference-gpu'),
  resource_status TEXT NOT NULL CHECK (resource_status IN ('available', 'reserved', 'released')),
  status TEXT NOT NULL CHECK (
    status IN ('previewed', 'queued', 'running', 'completed', 'cancelled', 'failed', 'cleaned')
  ),
  terminal_status TEXT CHECK (
    terminal_status IS NULL OR terminal_status IN ('completed', 'cancelled', 'failed', 'expired', 'cleaned')
  ),
  model_lifecycle TEXT NOT NULL CHECK (
    model_lifecycle IN ('not_loaded', 'loading', 'ready', 'running', 'unloaded', 'unavailable')
  ),
  model_loaded_at INTEGER,
  model_run_at INTEGER,
  model_cleanup_at INTEGER,
  cleanup_status TEXT NOT NULL CHECK (cleanup_status IN ('pending', 'complete')),
  preview_json TEXT NOT NULL CHECK (length(preview_json) BETWEEN 2 AND 4096),
  redaction_json TEXT NOT NULL CHECK (length(redaction_json) BETWEEN 2 AND 4096),
  frame_metadata_json TEXT CHECK (
    frame_metadata_json IS NULL OR length(frame_metadata_json) BETWEEN 2 AND 2048
  ),
  error_code TEXT CHECK (error_code IS NULL OR length(error_code) BETWEEN 1 AND 96),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 16384),
  created_at INTEGER NOT NULL,
  queued_at INTEGER,
  running_at INTEGER,
  completed_at INTEGER,
  cleaned_at INTEGER,
  updated_at INTEGER NOT NULL,
  UNIQUE (session_id, idempotency_key)
);
CREATE INDEX screen_vision_jobs_agent_status
  ON screen_vision_jobs(agent_id, status, updated_at DESC);
CREATE INDEX screen_vision_jobs_session_created
  ON screen_vision_jobs(session_id, created_at DESC);
CREATE UNIQUE INDEX screen_vision_one_active_job
  ON screen_vision_jobs(resource_key)
  WHERE status IN ('queued', 'running');

CREATE TABLE screen_vision_audit_log (
  id TEXT PRIMARY KEY,
  session_id TEXT REFERENCES screen_vision_sessions(id) ON DELETE SET NULL,
  job_id TEXT REFERENCES screen_vision_jobs(id) ON DELETE SET NULL,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  event TEXT NOT NULL CHECK (length(event) BETWEEN 1 AND 64),
  result TEXT NOT NULL CHECK (length(result) BETWEEN 1 AND 64),
  code TEXT CHECK (code IS NULL OR length(code) BETWEEN 1 AND 96),
  details_json TEXT NOT NULL CHECK (length(details_json) BETWEEN 2 AND 2048),
  created_at INTEGER NOT NULL
);
CREATE INDEX screen_vision_audit_agent_created
  ON screen_vision_audit_log(agent_id, created_at DESC, id);

CREATE TABLE screen_vision_idempotency (
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  operation TEXT NOT NULL CHECK (
    operation IN ('session_create', 'job_preview', 'job_confirm', 'job_cancel',
                  'job_cleanup', 'session_cancel')
  ),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 16384),
  result_kind TEXT NOT NULL CHECK (result_kind IN ('session', 'job')),
  result_id TEXT NOT NULL CHECK (length(result_id) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (owner_user_id, operation, idempotency_key)
);
CREATE INDEX screen_vision_idempotency_created
  ON screen_vision_idempotency(owner_user_id, created_at DESC);

INSERT INTO schema_migrations (version, applied_at)
VALUES (18, unixepoch('subsec') * 1000);
COMMIT;
