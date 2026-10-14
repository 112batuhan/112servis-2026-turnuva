-- The maps picked in a bracket match, each by one of its two teams, as its referee
-- marks them on the referee page. A map is picked at most once per match, and a
-- banned map can't be picked (nor a picked one banned); the handlers check that.
CREATE TABLE match_picks (
    match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    pool_map_id UUID NOT NULL REFERENCES pool_maps(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (match_id, pool_map_id)
);
