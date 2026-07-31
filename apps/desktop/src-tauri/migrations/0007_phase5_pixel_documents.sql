PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE pixel_documents (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL UNIQUE REFERENCES agents(id) ON DELETE CASCADE,
  owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  width INTEGER NOT NULL CHECK (width = 64),
  height INTEGER NOT NULL CHECK (height = 64),
  source_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

INSERT INTO pixel_documents (id, agent_id, owner_user_id, schema_version, width, height, source_json, created_at, updated_at)
SELECT lower(hex(randomblob(16))), id, owner_user_id, 1, 64, 64,
       '{"layers":[{"id":"body","name":"Body","visible":true,"locked":false,"pixels":[]}],"attachmentPoints":{}}',
       unixepoch('subsec') * 1000, unixepoch('subsec') * 1000 FROM agents;

INSERT INTO schema_migrations (version, applied_at) VALUES (7, unixepoch('subsec') * 1000);
COMMIT;
