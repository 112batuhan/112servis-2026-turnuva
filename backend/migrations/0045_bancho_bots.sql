-- The osu! accounts the site runs multiplayer rooms with (backend/src/bancho). Hosts add
-- them on /admin/bancho with their IRC password and switch each one on or off; every
-- active one stays connected to Bancho. The password is write-only through the API.
CREATE TABLE bancho_bots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The osu! username, as it logs in to IRC.
    username TEXT NOT NULL,
    irc_password TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX bancho_bots_username_key ON bancho_bots (lower(username));

-- The bot that runs each room: the one that made it (bot_made), or the one a room made
-- elsewhere was handed to, which its maker has to add as a referee. Null for rooms
-- from before the bots, or whose bot was deleted. Being a room's maker or one of its
-- referees counts against an account's limit of open rooms, so a room counts against
-- its bot until it's closed (closed_at), or until a bot that didn't make it leaves.
ALTER TABLE match_rooms
    ADD COLUMN bot_id UUID REFERENCES bancho_bots(id) ON DELETE SET NULL,
    ADD COLUMN bot_made BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN closed_at TIMESTAMPTZ;

CREATE INDEX match_rooms_open_idx ON match_rooms (bot_id) WHERE closed_at IS NULL;

-- Room lines are now saved by the room's bot rather than a referee's relay.
ALTER TABLE room_chat_messages
    ADD COLUMN bot_id UUID REFERENCES bancho_bots(id) ON DELETE SET NULL;
