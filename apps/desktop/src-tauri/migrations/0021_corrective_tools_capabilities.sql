PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

UPDATE tool_catalog
SET capabilities_json = CASE tool_id
      WHEN 'workspace.inspect_scope' THEN '["inspect_scope"]'
      WHEN 'workspace.organize_files' THEN '["preview","organize","compensate"]'
      WHEN 'calendar.list_events' THEN '["list_events"]'
      WHEN 'calendar.create_event' THEN '["preview","create","compensate"]'
      WHEN 'messaging.preview_message' THEN '["preview_message"]'
      WHEN 'messaging.send_message' THEN '["preview","send","compensate"]'
      ELSE capabilities_json
    END,
    updated_at = unixepoch('subsec') * 1000
WHERE capabilities_json LIKE '{"operations":%';

INSERT INTO schema_migrations (version, applied_at)
VALUES (21, unixepoch('subsec') * 1000);
COMMIT;
