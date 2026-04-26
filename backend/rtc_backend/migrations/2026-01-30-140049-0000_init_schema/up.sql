CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS citext;

DO $$ BEGIN
  CREATE TYPE member_role AS ENUM ('OWNER', 'ADMIN', 'MEMBER');
EXCEPTION
  WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS users (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
  email         citext UNIQUE NOT NULL,
  username      citext UNIQUE NOT NULL,
  password_hash text NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS servers (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
  name          text NOT NULL,
  description   text,
  created_by    uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS server_members (
  server_id   uuid NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role        member_role NOT NULL DEFAULT 'MEMBER',
  joined_at   timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (server_id, user_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_one_owner_per_server
  ON server_members(server_id)
  WHERE role = 'OWNER';

CREATE INDEX IF NOT EXISTS idx_server_members_user ON server_members(user_id);
CREATE INDEX IF NOT EXISTS idx_server_members_server ON server_members(server_id);

CREATE TABLE IF NOT EXISTS channels (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
  server_id   uuid NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  name        text NOT NULL,
  topic       text,
  position    int NOT NULL DEFAULT 0,
  created_by  uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  created_at  timestamptz NOT NULL DEFAULT now(),
  updated_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE (server_id, name)
);

CREATE INDEX IF NOT EXISTS idx_channels_server ON channels(server_id);

CREATE TABLE IF NOT EXISTS invites (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
  server_id     uuid NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
  code          text UNIQUE NOT NULL,
  created_by    uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  expires_at    timestamptz,
  max_uses      int,
  uses_count    int NOT NULL DEFAULT 0,
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_invites_server ON invites(server_id);

CREATE TABLE IF NOT EXISTS invite_uses (
  invite_id   uuid NOT NULL REFERENCES invites(id) ON DELETE CASCADE,
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  used_at     timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (invite_id, user_id)
);

CREATE TABLE IF NOT EXISTS messages (
  id           uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
  channel_id   uuid NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
  author_id    uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  content      text NOT NULL,
  created_at   timestamptz NOT NULL DEFAULT now(),
  edited_at    timestamptz,
  deleted_at   timestamptz,
  deleted_by   uuid REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_channel_created
  ON messages(channel_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_messages_author
  ON messages(author_id);

CREATE TABLE IF NOT EXISTS refresh_tokens (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  token_hash  text NOT NULL UNIQUE,        -- hash(refresh_token)
  created_at  timestamptz NOT NULL DEFAULT now(),
  expires_at  timestamptz NOT NULL,

  revoked_at  timestamptz,                 -- logout / manual revoke
  replaced_by uuid REFERENCES refresh_tokens(id) ON DELETE SET NULL,

  user_agent  text,
  ip          inet
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user
  ON refresh_tokens(user_id);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires
  ON refresh_tokens(expires_at);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_revoked
  ON refresh_tokens(revoked_at);

