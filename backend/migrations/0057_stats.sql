-- The finished pool maps of a qualifier lobby with each player's score, as the room's
-- bot read them (from the chat, or from osu! once the referee synced the room), saved
-- whenever they change and replaced as a whole, like a bracket match's map results.
-- Every attempt is kept in the order played (a map can be played again for a late
-- team); the statistics count each team's latest one. The teams come from the players.
CREATE TABLE qualifier_map_results (
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    position INT NOT NULL,
    pool_map_id UUID REFERENCES pool_maps(id) ON DELETE SET NULL,
    PRIMARY KEY (match_id, position)
);

CREATE TABLE qualifier_map_scores (
    match_id UUID NOT NULL,
    position INT NOT NULL,
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    score BIGINT NOT NULL,
    -- Acronyms, the room's included ("HDNF"); null when they weren't read.
    mods TEXT,
    passed BOOLEAN NOT NULL,
    PRIMARY KEY (match_id, position, user_id),
    FOREIGN KEY (match_id, position)
        REFERENCES qualifier_map_results(match_id, position) ON DELETE CASCADE
);

-- osu!'s record of every score in the site's rooms, read after each saved map: the
-- chat has no accuracy, so the statistics take it from here, matched to a saved score
-- by room, beatmap, player and score. Kept by room, so replacing saved results never
-- loses it.
CREATE TABLE room_scores (
    mp_id BIGINT NOT NULL,
    beatmap_id BIGINT NOT NULL,
    osu_id BIGINT NOT NULL,
    score BIGINT NOT NULL,
    -- 0..1
    accuracy DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (mp_id, beatmap_id, osu_id, score)
);

-- When osu!'s record of the room was last read, and how many reads in a row found
-- nothing new (each one waits twice as long as the one before).
ALTER TABLE match_rooms
    ADD COLUMN scores_read_at TIMESTAMPTZ,
    ADD COLUMN scores_misses INT NOT NULL DEFAULT 0;
