ALTER TABLE users
  ADD COLUMN IF NOT EXISTS display_name_mode text NOT NULL DEFAULT 'nickname';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_constraint
    WHERE conname = 'users_display_name_mode_check'
  ) THEN
    ALTER TABLE users
      ADD CONSTRAINT users_display_name_mode_check
      CHECK (display_name_mode IN ('nickname', 'username'));
  END IF;
END
$$;
