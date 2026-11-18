-- The auto referee: the room's bot runs the match by itself (bancho/autoref.rs), from
-- making the room to closing it, unless a referee switches the match to manual on the
-- referee page.

-- On for every match unless switched off; switching it back on continues where it left
-- off.
ALTER TABLE match_room_settings ADD COLUMN auto_ref BOOLEAN NOT NULL DEFAULT TRUE;

-- Where the auto referee stands in each match it ran (its phase, timers, the teams'
-- -abort and -durdur uses), so a restart or a switch back from manual picks it up.
CREATE TABLE match_autoref (
    match_id UUID PRIMARY KEY REFERENCES matches(id) ON DELETE CASCADE,
    state JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- What the auto referee did and why, for the referee page's log tab.
CREATE TYPE autoref_log_level AS ENUM ('info', 'warn', 'error');
CREATE TABLE autoref_logs (
    id BIGSERIAL PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    level autoref_log_level NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX autoref_logs_match_idx ON autoref_logs (match_id, id);

-- The start time the 20-minute "the bot will invite you" reminder went out for.
ALTER TABLE match_discord ADD COLUMN autoref_reminded_for TIMESTAMPTZ;

-- Notifications from the auto referee:
-- * player_report: a player's -rapor, to the hosts and referees, with its text.
-- * emergency_stop: a team's -durdur, to the hosts and referees.
-- * autoref_stopped: the auto referee handed a match over to the staff, with why.
-- * no_show_forfeit: a bracket match was forfeited for a team that didn't show up, to
--   both teams' players (team_id: the team that forfeited).
-- * lobby_no_show: a team that didn't show up was taken out of its qualifier lobby, to
--   its players.
ALTER TYPE notification_kind ADD VALUE 'player_report';
ALTER TYPE notification_kind ADD VALUE 'emergency_stop';
ALTER TYPE notification_kind ADD VALUE 'autoref_stopped';
ALTER TYPE notification_kind ADD VALUE 'no_show_forfeit';
ALTER TYPE notification_kind ADD VALUE 'lobby_no_show';

-- player_report: what the player wrote; autoref_stopped: why it stopped.
ALTER TABLE notifications ADD COLUMN message TEXT;
