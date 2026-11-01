-- Hosts can stop the bot from opening new match channels (/admin/discord) while it keeps
-- updating, archiving and closing the ones that exist.
ALTER TABLE discord_settings ADD COLUMN match_channels_paused BOOLEAN NOT NULL DEFAULT false;
