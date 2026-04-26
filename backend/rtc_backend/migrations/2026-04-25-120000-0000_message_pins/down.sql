DROP INDEX IF EXISTS idx_messages_channel_pinned;

ALTER TABLE messages
  DROP COLUMN IF EXISTS pinned_by,
  DROP COLUMN IF EXISTS pinned_at;
