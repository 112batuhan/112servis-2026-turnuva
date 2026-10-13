-- Set by hosts once teams are final: players can no longer form, rename, re-banner
-- or disband teams.
ALTER TABLE app_settings ADD COLUMN teams_frozen BOOLEAN NOT NULL DEFAULT false;
