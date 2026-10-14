-- The maps banned in a bracket match, each by one of its two teams, as its referee
-- marks them on the referee page. A map is banned at most once per match.
CREATE TABLE match_bans (
    match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    pool_map_id UUID NOT NULL REFERENCES pool_maps(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (match_id, pool_map_id)
);
