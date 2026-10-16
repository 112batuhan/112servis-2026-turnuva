-- Each player's score on a saved map result, with the mods they played it with (as
-- acronyms, the room's included: "HDNF"; null when the referee page couldn't read
-- them). Saved and replaced with the map results; the results post lists them for
-- freemod and tiebreaker maps.
CREATE TABLE match_map_scores (
    match_id UUID NOT NULL,
    position INT NOT NULL,
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    score BIGINT NOT NULL,
    mods TEXT,
    passed BOOLEAN NOT NULL,
    PRIMARY KEY (match_id, position, user_id),
    FOREIGN KEY (match_id, position)
        REFERENCES match_map_results(match_id, position) ON DELETE CASCADE
);
