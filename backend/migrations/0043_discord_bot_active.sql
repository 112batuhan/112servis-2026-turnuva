-- Hosts can switch the bot off and on from /admin/discord without touching the config.
-- While it's off it isn't connected to Discord, runs no jobs and queues none.
ALTER TABLE discord_settings ADD COLUMN bot_active BOOLEAN NOT NULL DEFAULT true;
