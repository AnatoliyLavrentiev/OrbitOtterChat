ALTER TABLE messages
  ADD COLUMN IF NOT EXISTS pinned_at timestamptz,
  ADD COLUMN IF NOT EXISTS pinned_by uuid REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_messages_channel_pinned
  ON messages(channel_id, pinned_at DESC)
  WHERE pinned_at IS NOT NULL AND deleted_at IS NULL;
