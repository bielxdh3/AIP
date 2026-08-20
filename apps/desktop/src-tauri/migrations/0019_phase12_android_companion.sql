PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE companion_devices (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  device_id TEXT NOT NULL CHECK (length(device_id) BETWEEN 1 AND 96),
  platform TEXT NOT NULL CHECK (platform = 'android'),
  app_version TEXT NOT NULL CHECK (length(app_version) BETWEEN 1 AND 64),
  protocol_version INTEGER NOT NULL CHECK (protocol_version BETWEEN 1 AND 16),
  status TEXT NOT NULL CHECK (status IN ('pairing_requested', 'paired', 'expired', 'revoked')),
  fingerprint TEXT NOT NULL CHECK (length(fingerprint) BETWEEN 1 AND 192),
  pairing_nonce_metadata TEXT NOT NULL CHECK (length(pairing_nonce_metadata) BETWEEN 1 AND 192),
  key_version INTEGER NOT NULL CHECK (key_version BETWEEN 1 AND 1024),
  pairing_expires_at INTEGER,
  paired_at INTEGER,
  revoked_at INTEGER,
  last_seen_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (owner_user_id, device_id)
);
CREATE INDEX companion_devices_agent_status
  ON companion_devices(agent_id, status, updated_at DESC);

CREATE TABLE companion_sessions (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES companion_devices(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  status TEXT NOT NULL CHECK (status IN ('connected', 'disconnected', 'revoked', 'expired')),
  protocol_version INTEGER NOT NULL CHECK (protocol_version BETWEEN 1 AND 16),
  app_version TEXT NOT NULL CHECK (length(app_version) BETWEEN 1 AND 64),
  negotiated_protocol_version INTEGER NOT NULL CHECK (negotiated_protocol_version BETWEEN 1 AND 16),
  key_fingerprint TEXT NOT NULL CHECK (length(key_fingerprint) BETWEEN 1 AND 192),
  session_nonce_metadata TEXT NOT NULL CHECK (length(session_nonce_metadata) BETWEEN 1 AND 192),
  last_replay_counter INTEGER NOT NULL CHECK (last_replay_counter >= 0),
  connected_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  disconnected_at INTEGER,
  updated_at INTEGER NOT NULL
);
CREATE INDEX companion_sessions_agent_status
  ON companion_sessions(agent_id, status, last_seen_at DESC);
CREATE INDEX companion_sessions_device_status
  ON companion_sessions(device_id, status, last_seen_at DESC);

CREATE TABLE companion_replay_guards (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES companion_devices(id) ON DELETE CASCADE,
  session_id TEXT REFERENCES companion_sessions(id) ON DELETE CASCADE,
  message_nonce_metadata TEXT NOT NULL CHECK (length(message_nonce_metadata) BETWEEN 1 AND 192),
  replay_counter INTEGER NOT NULL CHECK (replay_counter >= 0),
  message_kind TEXT NOT NULL CHECK (length(message_kind) BETWEEN 1 AND 64),
  created_at INTEGER NOT NULL,
  UNIQUE (device_id, message_nonce_metadata)
);
CREATE INDEX companion_replay_guards_session_counter
  ON companion_replay_guards(session_id, replay_counter DESC);

CREATE TABLE companion_queue (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES companion_devices(id) ON DELETE CASCADE,
  session_id TEXT NOT NULL REFERENCES companion_sessions(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  kind TEXT NOT NULL CHECK (kind IN ('text', 'audio', 'image', 'file', 'task')),
  status TEXT NOT NULL CHECK (status IN ('previewed', 'queued', 'cancelled', 'failed')),
  payload_json TEXT NOT NULL CHECK (length(payload_json) BETWEEN 2 AND 12288),
  summary TEXT NOT NULL CHECK (length(summary) BETWEEN 1 AND 512),
  metadata_only INTEGER NOT NULL CHECK (metadata_only = 1),
  media_bytes_persisted INTEGER NOT NULL CHECK (media_bytes_persisted = 0),
  approval_required INTEGER NOT NULL CHECK (approval_required = 1),
  retry_count INTEGER NOT NULL CHECK (retry_count BETWEEN 0 AND 8),
  error_code TEXT,
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  previewed_at INTEGER NOT NULL,
  approved_at INTEGER,
  cancelled_at INTEGER,
  updated_at INTEGER NOT NULL,
  UNIQUE (owner_user_id, idempotency_key)
);
CREATE INDEX companion_queue_agent_status
  ON companion_queue(agent_id, status, updated_at DESC);
CREATE INDEX companion_queue_session_created
  ON companion_queue(session_id, created_at DESC);

CREATE TABLE companion_history (
  id TEXT PRIMARY KEY,
  device_id TEXT REFERENCES companion_devices(id) ON DELETE SET NULL,
  session_id TEXT REFERENCES companion_sessions(id) ON DELETE SET NULL,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  direction TEXT NOT NULL CHECK (direction IN ('incoming', 'outgoing', 'system')),
  kind TEXT NOT NULL CHECK (length(kind) BETWEEN 1 AND 64),
  summary TEXT NOT NULL CHECK (length(summary) BETWEEN 1 AND 512),
  metadata_json TEXT NOT NULL CHECK (length(metadata_json) BETWEEN 2 AND 2048),
  metadata_only INTEGER NOT NULL CHECK (metadata_only = 1),
  media_bytes_persisted INTEGER NOT NULL CHECK (media_bytes_persisted = 0),
  created_at INTEGER NOT NULL
);
CREATE INDEX companion_history_agent_created
  ON companion_history(agent_id, created_at DESC, id);

CREATE TABLE companion_key_rotations (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES companion_devices(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  old_fingerprint TEXT NOT NULL CHECK (length(old_fingerprint) BETWEEN 1 AND 192),
  new_fingerprint TEXT NOT NULL CHECK (length(new_fingerprint) BETWEEN 1 AND 192),
  old_key_version INTEGER NOT NULL CHECK (old_key_version >= 1),
  new_key_version INTEGER NOT NULL CHECK (new_key_version > old_key_version),
  nonce_metadata TEXT NOT NULL CHECK (length(nonce_metadata) BETWEEN 1 AND 192),
  status TEXT NOT NULL CHECK (status = 'completed'),
  reason TEXT NOT NULL CHECK (length(reason) BETWEEN 1 AND 512),
  created_at INTEGER NOT NULL
);
CREATE INDEX companion_key_rotations_device_created
  ON companion_key_rotations(device_id, created_at DESC);

CREATE TABLE companion_revocations (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES companion_devices(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  previous_status TEXT NOT NULL CHECK (length(previous_status) BETWEEN 1 AND 32),
  reason TEXT NOT NULL CHECK (length(reason) BETWEEN 1 AND 512),
  created_at INTEGER NOT NULL
);
CREATE INDEX companion_revocations_device_created
  ON companion_revocations(device_id, created_at DESC);

CREATE TABLE companion_audit_log (
  id TEXT PRIMARY KEY,
  device_id TEXT REFERENCES companion_devices(id) ON DELETE SET NULL,
  session_id TEXT REFERENCES companion_sessions(id) ON DELETE SET NULL,
  queue_id TEXT REFERENCES companion_queue(id) ON DELETE SET NULL,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  event TEXT NOT NULL CHECK (length(event) BETWEEN 1 AND 64),
  result TEXT NOT NULL CHECK (length(result) BETWEEN 1 AND 64),
  code TEXT,
  details_json TEXT NOT NULL CHECK (length(details_json) BETWEEN 2 AND 2048),
  created_at INTEGER NOT NULL
);
CREATE INDEX companion_audit_agent_created
  ON companion_audit_log(agent_id, created_at DESC, id);

CREATE TABLE companion_idempotency (
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  operation TEXT NOT NULL CHECK (
    operation IN ('pairing_start', 'pairing_confirm', 'session_connect', 'session_reconnect',
                  'queue_preview', 'queue_approve', 'queue_cancel', 'queue_retry',
                  'key_rotate', 'device_revoke')
  ),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 16384),
  result_kind TEXT NOT NULL CHECK (result_kind IN ('device', 'session', 'queue', 'rotation', 'revocation')),
  result_id TEXT NOT NULL CHECK (length(result_id) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (owner_user_id, operation, idempotency_key)
);
CREATE INDEX companion_idempotency_created
  ON companion_idempotency(owner_user_id, created_at DESC);

INSERT INTO schema_migrations (version, applied_at)
VALUES (19, unixepoch('subsec') * 1000);
COMMIT;
