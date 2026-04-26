ALTER TABLE users
  DROP CONSTRAINT IF EXISTS users_display_name_mode_check,
  DROP COLUMN IF EXISTS display_name_mode;
