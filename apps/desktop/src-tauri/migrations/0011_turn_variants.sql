PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

ALTER TABLE conversation_messages ADD COLUMN turn_group_id TEXT;

UPDATE conversation_messages SET turn_group_id = id WHERE author_type = 'user';
UPDATE conversation_messages
SET turn_group_id = COALESCE((
  SELECT user_message.turn_group_id
  FROM conversation_messages AS user_message
  WHERE user_message.conversation_id = conversation_messages.conversation_id
    AND user_message.branch_id = conversation_messages.branch_id
    AND user_message.author_type = 'user'
    AND (user_message.created_at, user_message.id) < (conversation_messages.created_at, conversation_messages.id)
  ORDER BY user_message.created_at DESC, user_message.id DESC LIMIT 1
), conversation_messages.id)
WHERE author_type = 'agent';

CREATE INDEX conversation_messages_turn_variants
ON conversation_messages(conversation_id, agent_id, turn_group_id, author_type, created_at, id);

INSERT INTO schema_migrations (version, applied_at)
VALUES (11, unixepoch('subsec') * 1000);

COMMIT;
