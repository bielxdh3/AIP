PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE gateway_accounts (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  local_account_id TEXT NOT NULL UNIQUE CHECK (length(local_account_id) BETWEEN 1 AND 96),
  external_account_id_metadata TEXT NOT NULL CHECK (length(external_account_id_metadata) BETWEEN 1 AND 192),
  ownership_scope TEXT NOT NULL CHECK (ownership_scope = 'owner_only'),
  status TEXT NOT NULL CHECK (status IN ('metadata_only', 'revoked')),
  metadata_only INTEGER NOT NULL CHECK (metadata_only = 1),
  external_effect_performed INTEGER NOT NULL CHECK (external_effect_performed = 0),
  standalone_fallback INTEGER NOT NULL CHECK (standalone_fallback = 1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE gateway_transfers (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES gateway_accounts(id) ON DELETE CASCADE,
  source_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  destination_account_metadata TEXT NOT NULL CHECK (length(destination_account_metadata) BETWEEN 1 AND 192),
  integrity_hash TEXT NOT NULL CHECK (length(integrity_hash) BETWEEN 1 AND 256),
  status TEXT NOT NULL CHECK (status IN ('previewed', 'approved', 'revoked')),
  authorization_status TEXT NOT NULL CHECK (authorization_status IN ('pending_owner_approval', 'owner_approved', 'revoked')),
  approval_required INTEGER NOT NULL CHECK (approval_required = 1),
  metadata_only INTEGER NOT NULL CHECK (metadata_only = 1),
  external_effect_performed INTEGER NOT NULL CHECK (external_effect_performed = 0),
  standalone_fallback INTEGER NOT NULL CHECK (standalone_fallback = 1),
  created_at INTEGER NOT NULL,
  approved_at INTEGER,
  updated_at INTEGER NOT NULL
);
CREATE INDEX gateway_transfers_owner_status
  ON gateway_transfers(owner_user_id, status, updated_at DESC);

CREATE TABLE gateway_sessions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES gateway_accounts(id) ON DELETE CASCADE,
  transfer_id TEXT NOT NULL REFERENCES gateway_transfers(id) ON DELETE CASCADE,
  source_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  client_id TEXT NOT NULL CHECK (length(client_id) BETWEEN 1 AND 96),
  status TEXT NOT NULL CHECK (status IN ('connected', 'disconnected', 'revoked', 'expired')),
  protocol_version INTEGER NOT NULL CHECK (protocol_version BETWEEN 1 AND 16),
  app_version TEXT NOT NULL CHECK (length(app_version) BETWEEN 1 AND 64),
  negotiated_protocol_version INTEGER NOT NULL CHECK (negotiated_protocol_version BETWEEN 1 AND 16),
  session_nonce_metadata TEXT NOT NULL CHECK (length(session_nonce_metadata) BETWEEN 1 AND 192),
  auth_proof_metadata TEXT NOT NULL CHECK (length(auth_proof_metadata) BETWEEN 1 AND 192),
  last_replay_counter INTEGER NOT NULL CHECK (last_replay_counter >= 0),
  scope TEXT NOT NULL CHECK (scope = 'administrative_recovery'),
  authenticated INTEGER NOT NULL CHECK (authenticated = 1),
  local_loopback_only INTEGER NOT NULL CHECK (local_loopback_only = 1),
  standalone_fallback INTEGER NOT NULL CHECK (standalone_fallback = 1),
  connected_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  disconnected_at INTEGER,
  updated_at INTEGER NOT NULL
);
CREATE INDEX gateway_sessions_owner_status
  ON gateway_sessions(owner_user_id, status, last_seen_at DESC);
CREATE INDEX gateway_sessions_client_status
  ON gateway_sessions(client_id, status, last_seen_at DESC);

CREATE TABLE gateway_replay_guards (
  id TEXT PRIMARY KEY,
  client_id TEXT NOT NULL CHECK (length(client_id) BETWEEN 1 AND 96),
  session_id TEXT REFERENCES gateway_sessions(id) ON DELETE CASCADE,
  message_nonce_metadata TEXT NOT NULL CHECK (length(message_nonce_metadata) BETWEEN 1 AND 192),
  replay_counter INTEGER NOT NULL CHECK (replay_counter >= 0),
  message_kind TEXT NOT NULL CHECK (length(message_kind) BETWEEN 1 AND 64),
  created_at INTEGER NOT NULL,
  UNIQUE (client_id, message_nonce_metadata)
);
CREATE INDEX gateway_replay_guards_client_counter
  ON gateway_replay_guards(client_id, replay_counter DESC);

CREATE TABLE gateway_recoveries (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES gateway_accounts(id) ON DELETE CASCADE,
  transfer_id TEXT NOT NULL REFERENCES gateway_transfers(id) ON DELETE CASCADE,
  session_id TEXT NOT NULL REFERENCES gateway_sessions(id) ON DELETE CASCADE,
  source_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  client_id TEXT NOT NULL CHECK (length(client_id) BETWEEN 1 AND 96),
  kind TEXT NOT NULL CHECK (kind = 'mobile_administrative'),
  status TEXT NOT NULL CHECK (status IN ('pending_approval', 'approved', 'revoked')),
  target_metadata TEXT NOT NULL CHECK (length(target_metadata) BETWEEN 1 AND 192),
  approval_required INTEGER NOT NULL CHECK (approval_required = 1),
  metadata_only INTEGER NOT NULL CHECK (metadata_only = 1),
  external_effect_performed INTEGER NOT NULL CHECK (external_effect_performed = 0),
  created_at INTEGER NOT NULL,
  approved_at INTEGER,
  updated_at INTEGER NOT NULL
);
CREATE INDEX gateway_recoveries_owner_status
  ON gateway_recoveries(owner_user_id, status, updated_at DESC);

CREATE TABLE gateway_audit_log (
  id TEXT PRIMARY KEY,
  account_id TEXT REFERENCES gateway_accounts(id) ON DELETE SET NULL,
  transfer_id TEXT REFERENCES gateway_transfers(id) ON DELETE SET NULL,
  session_id TEXT REFERENCES gateway_sessions(id) ON DELETE SET NULL,
  recovery_id TEXT REFERENCES gateway_recoveries(id) ON DELETE SET NULL,
  source_agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  event TEXT NOT NULL CHECK (length(event) BETWEEN 1 AND 64),
  result TEXT NOT NULL CHECK (length(result) BETWEEN 1 AND 64),
  code TEXT,
  details_json TEXT NOT NULL CHECK (length(details_json) BETWEEN 2 AND 2048),
  created_at INTEGER NOT NULL
);
CREATE INDEX gateway_audit_owner_created
  ON gateway_audit_log(owner_user_id, created_at DESC, id);

CREATE TABLE gateway_revocations (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES gateway_accounts(id) ON DELETE CASCADE,
  transfer_id TEXT REFERENCES gateway_transfers(id) ON DELETE SET NULL,
  session_id TEXT REFERENCES gateway_sessions(id) ON DELETE SET NULL,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  target_kind TEXT NOT NULL CHECK (target_kind IN ('transfer', 'session')),
  target_id TEXT NOT NULL CHECK (length(target_id) BETWEEN 1 AND 128),
  previous_status TEXT NOT NULL CHECK (length(previous_status) BETWEEN 1 AND 32),
  reason TEXT NOT NULL CHECK (length(reason) BETWEEN 1 AND 512),
  created_at INTEGER NOT NULL
);
CREATE INDEX gateway_revocations_owner_created
  ON gateway_revocations(owner_user_id, created_at DESC, id);

CREATE TABLE gateway_idempotency (
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  operation TEXT NOT NULL CHECK (
    operation IN ('transfer_prepare', 'transfer_approve', 'session_connect',
                  'session_reconnect', 'recovery_request', 'recovery_approve',
                  'session_revoke', 'transfer_revoke')
  ),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 16384),
  result_kind TEXT NOT NULL CHECK (result_kind IN ('transfer', 'session', 'recovery', 'revocation')),
  result_id TEXT NOT NULL CHECK (length(result_id) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (owner_user_id, operation, idempotency_key)
);
CREATE INDEX gateway_idempotency_created
  ON gateway_idempotency(owner_user_id, created_at DESC);

INSERT INTO schema_migrations (version, applied_at)
VALUES (20, unixepoch('subsec') * 1000);
COMMIT;
