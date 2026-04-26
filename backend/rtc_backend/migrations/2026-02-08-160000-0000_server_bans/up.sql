CREATE TABLE IF NOT EXISTS server_bans (
  server_id   uuid NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  banned_by   uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  reason      text,
  expires_at  timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (server_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_server_bans_server ON server_bans(server_id);
CREATE INDEX IF NOT EXISTS idx_server_bans_user ON server_bans(user_id);
CREATE INDEX IF NOT EXISTS idx_server_bans_expires ON server_bans(expires_at);
