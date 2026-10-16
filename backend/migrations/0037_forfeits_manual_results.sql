-- A bracket match decided by forfeit: the slot (1 or 2) of the team that forfeited,
-- because it didn't show up or the match couldn't be finished. The other team wins
-- whatever the score says. Set when the referee or a host concludes the match that
-- way, cleared when it's reopened.
ALTER TABLE bracket_matches
    ADD COLUMN forfeit_slot SMALLINT CHECK (forfeit_slot IN (1, 2));

-- Map wins a bracket match's referee awards by hand, for a map the room didn't settle
-- (a disconnect, a result Bancho never reported) or a point given as a penalty. Each
-- row counts as one map won by the team, on top of the ones read from the room; the
-- map is optional and goes back to null if it leaves the pool.
CREATE TABLE match_manual_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    pool_map_id UUID REFERENCES pool_maps(id) ON DELETE SET NULL,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    note TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX match_manual_results_match_idx ON match_manual_results (match_id, created_at);
