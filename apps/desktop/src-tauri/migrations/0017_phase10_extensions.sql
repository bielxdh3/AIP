PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE extension_catalog (
  extension_id TEXT PRIMARY KEY CHECK (length(extension_id) BETWEEN 1 AND 96),
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  catalog_scope TEXT NOT NULL CHECK (catalog_scope = 'private_local'),
  source_kind TEXT NOT NULL CHECK (source_kind IN ('administrator_selected', 'agent_created')),
  lifecycle TEXT NOT NULL CHECK (
    lifecycle IN ('review_required', 'approved', 'active', 'disabled', 'rejected', 'recovery_required')
  ),
  current_revision INTEGER NOT NULL CHECK (current_revision >= 1),
  active_revision INTEGER,
  untrusted INTEGER NOT NULL CHECK (untrusted = 1),
  created_by_agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE extension_manifest_revisions (
  extension_id TEXT NOT NULL REFERENCES extension_catalog(extension_id) ON DELETE CASCADE,
  revision INTEGER NOT NULL CHECK (revision >= 1),
  manifest_version INTEGER NOT NULL CHECK (manifest_version = 1),
  extension_version TEXT NOT NULL CHECK (length(extension_version) BETWEEN 5 AND 32),
  sdk_version TEXT NOT NULL CHECK (length(sdk_version) BETWEEN 1 AND 64),
  name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 160),
  sandbox_policy TEXT NOT NULL CHECK (sandbox_policy = 'metadata_only'),
  admission_policy TEXT NOT NULL CHECK (admission_policy = 'local_fixture_only'),
  capabilities_json TEXT NOT NULL CHECK (length(capabilities_json) BETWEEN 2 AND 2048),
  local_fixture_ref TEXT CHECK (
    local_fixture_ref IS NULL OR length(local_fixture_ref) BETWEEN 1 AND 160
  ),
  manifest_json TEXT NOT NULL CHECK (length(manifest_json) BETWEEN 2 AND 8192),
  compatible INTEGER NOT NULL CHECK (compatible IN (0, 1)),
  review_status TEXT NOT NULL CHECK (review_status IN ('pending', 'approved', 'rejected')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (extension_id, revision)
);

CREATE TABLE extension_proposals (
  id TEXT PRIMARY KEY,
  extension_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('administrator_selected', 'agent_created')),
  proposer_agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'withdrawn')),
  approved_capabilities_json TEXT,
  review_reason TEXT CHECK (review_reason IS NULL OR length(review_reason) BETWEEN 1 AND 512),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (extension_id, revision),
  UNIQUE (owner_user_id, idempotency_key),
  FOREIGN KEY (extension_id, revision)
    REFERENCES extension_manifest_revisions(extension_id, revision) ON DELETE CASCADE
);

CREATE TABLE extension_permission_requests (
  proposal_id TEXT NOT NULL REFERENCES extension_proposals(id) ON DELETE CASCADE,
  capability TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'denied')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (proposal_id, capability)
);

CREATE TABLE extension_audit_log (
  id TEXT PRIMARY KEY,
  extension_id TEXT,
  proposal_id TEXT,
  revision INTEGER,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  event TEXT NOT NULL CHECK (length(event) BETWEEN 1 AND 64),
  result TEXT NOT NULL CHECK (length(result) BETWEEN 1 AND 64),
  code TEXT CHECK (code IS NULL OR length(code) BETWEEN 1 AND 96),
  details_json TEXT NOT NULL CHECK (length(details_json) BETWEEN 2 AND 2048),
  created_at INTEGER NOT NULL,
  FOREIGN KEY (extension_id) REFERENCES extension_catalog(extension_id) ON DELETE SET NULL,
  FOREIGN KEY (proposal_id) REFERENCES extension_proposals(id) ON DELETE SET NULL
);

CREATE TABLE extension_idempotency (
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  operation TEXT NOT NULL CHECK (
    operation IN ('create', 'review', 'activate', 'update', 'rollback', 'disable')
  ),
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_json TEXT NOT NULL CHECK (length(request_json) BETWEEN 2 AND 16384),
  result_kind TEXT NOT NULL CHECK (result_kind IN ('proposal', 'catalog')),
  result_json TEXT NOT NULL CHECK (length(result_json) BETWEEN 2 AND 32768),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (owner_user_id, operation, idempotency_key)
);
CREATE INDEX extension_catalog_lifecycle
  ON extension_catalog(owner_user_id, lifecycle, updated_at DESC);
CREATE INDEX extension_proposals_status
  ON extension_proposals(owner_user_id, status, updated_at DESC);
CREATE INDEX extension_audit_created
  ON extension_audit_log(owner_user_id, created_at DESC, id);
CREATE INDEX extension_idempotency_created
  ON extension_idempotency(owner_user_id, created_at DESC);

INSERT INTO schema_migrations (version, applied_at)
VALUES (17, unixepoch('subsec') * 1000);
COMMIT;
