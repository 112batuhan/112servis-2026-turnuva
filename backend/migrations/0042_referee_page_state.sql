-- What the referee page keeps about a match besides its bans, picks and results, so
-- every referee and host who has it open sees the same: how the room is set up, where
-- the finished maps are read from and whose pick the referee said is next. A match
-- without a row has the defaults.
CREATE TABLE match_room_settings (
    match_id UUID PRIMARY KEY REFERENCES matches(id) ON DELETE CASCADE,
    -- !mp set's score mode (0 score, 1 accuracy, 2 combo, 3 Score V2).
    score_mode SMALLINT NOT NULL DEFAULT 3 CHECK (score_mode BETWEEN 0 AND 3),
    no_fail BOOLEAN NOT NULL DEFAULT TRUE,
    auto_place BOOLEAN NOT NULL DEFAULT TRUE,
    auto_kick BOOLEAN NOT NULL DEFAULT TRUE,
    -- When the results were last synced from osu!'s record of the rooms; null reads
    -- them from the rooms' chat.
    osu_synced_at TIMESTAMPTZ,
    -- Bracket matches: the team the referee said picks next, overriding the rules
    -- until the next ban or pick (marking one clears it).
    next_pick_team_id UUID REFERENCES teams(id) ON DELETE SET NULL
);

-- The finished maps the referee left out of the score ("Sayma"), by the referee
-- page's key for them ("<source>:<room>:<n>").
CREATE TABLE match_ignored_maps (
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    map_key TEXT NOT NULL,
    PRIMARY KEY (match_id, map_key)
);
