-- The Discord bot (src/discord). Only its token and the server's id are in the config;
-- everything it points at in the server is picked by hosts on /admin/discord.

-- Ids are Discord snowflakes. Unset means the bot skips whatever needs it.
CREATE TABLE discord_settings (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    -- Given to every linked account, and to members of a formed team.
    verified_role_id BIGINT,
    player_role_id BIGINT,
    -- Where new teams, matches starting soon and match results are posted.
    team_announce_channel_id BIGINT,
    match_announce_channel_id BIGINT,
    results_channel_id BIGINT,
    -- The hosts' overview of everything, and where a declined referee or streamer slot
    -- is offered to the others.
    host_channel_id BIGINT,
    referee_channel_id BIGINT,
    streamer_channel_id BIGINT,
    -- Where each match's private channel lives while it's open and once it's concluded.
    qualifier_category_id BIGINT,
    qualifier_archive_category_id BIGINT,
    bracket_category_id BIGINT,
    bracket_archive_category_id BIGINT
);
INSERT INTO discord_settings DEFAULT VALUES;

-- A Discord category holds at most 50 channels. When one of the four above is full,
-- the bot creates another next to it and keeps using it for that base category.
CREATE TABLE discord_overflow_categories (
    category_id BIGINT PRIMARY KEY,
    base_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX discord_overflow_categories_base_idx ON discord_overflow_categories (base_id, created_at);

-- Whether the account is in the tournament server: null until the bot first checks.
-- Going from true to false is a player leaving, which their teammate and the hosts
-- are told about.
ALTER TABLE discord_accounts ADD COLUMN in_guild BOOLEAN;

-- A match's private channel and what the bot has posted about it. `synced` is what was
-- last applied to the channel, so syncing an unchanged match makes no API calls.
-- `reminded_for` and `announced_for` are the start times the reminder and the public
-- announcement were sent for, so a moved match gets them again.
CREATE TABLE match_discord (
    match_id UUID PRIMARY KEY REFERENCES matches(id) ON DELETE CASCADE,
    channel_id BIGINT UNIQUE,
    synced TEXT,
    reminded_for TIMESTAMPTZ,
    announced_for TIMESTAMPTZ,
    results_channel_id BIGINT,
    results_message_id BIGINT
);

-- When each player last appeared in a team announcement. A new team is announced
-- unless both of its players were in the last week.
CREATE TABLE discord_announced_players (
    user_id UUID PRIMARY KEY REFERENCES osu_users(id) ON DELETE CASCADE,
    announced_at TIMESTAMPTZ NOT NULL
);

-- Spectators who want to be pinged when a bracket match is about to start.
CREATE TABLE match_subscriptions (
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (match_id, user_id)
);
CREATE INDEX match_subscriptions_user_id_idx ON match_subscriptions (user_id);

-- What the bot still has to do, written in the same transaction as the change it's
-- about and run in order by one worker. A failed job is retried later.
CREATE TABLE discord_outbox (
    id BIGSERIAL PRIMARY KEY,
    job JSONB NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    run_after TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX discord_outbox_run_after_idx ON discord_outbox (run_after, id);

-- discord_left: a team member left the Discord server, to their teammate and the hosts.
ALTER TYPE notification_kind ADD VALUE 'discord_left';
