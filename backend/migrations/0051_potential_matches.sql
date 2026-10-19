-- The pairings an undecided bracket match may still turn out to be, kept in step with
-- the bracket by crate::potentials: when a slot of the match waits on another match of
-- the same stage whose teams are both known, every combination of the teams that can
-- reach it gets a row, so the players can agree on a time (and get a Discord channel)
-- before they know which one they'll play. A row whose pairing can no longer happen is
-- deleted; once the match has both its teams, the row for that pairing hands its time,
-- staff, proposals and pick'em predictions to the match and goes too. The match row
-- (kind 'potential') carries the time and the referee, like any match.
CREATE TABLE potential_matches (
    match_id UUID PRIMARY KEY REFERENCES matches(id) ON DELETE CASCADE,
    bracket_match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    team1_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    team2_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    streamer_id UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    CHECK (team1_id <> team2_id),
    CONSTRAINT potential_matches_pair_key UNIQUE (bracket_match_id, team1_id, team2_id)
);

CREATE INDEX potential_matches_team1_idx ON potential_matches (team1_id);
CREATE INDEX potential_matches_team2_idx ON potential_matches (team2_id);

-- A row that goes with its bracket match or one of its teams takes its match row along,
-- so no 'potential' match is left without its pairing.
CREATE FUNCTION potential_matches_drop_match() RETURNS trigger AS $$
BEGIN
    DELETE FROM matches WHERE id = OLD.match_id;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER potential_matches_drop_match
    AFTER DELETE ON potential_matches
    FOR EACH ROW EXECUTE FUNCTION potential_matches_drop_match();

-- The teams of a potential match propose times for it like for a bracket match.
ALTER TABLE match_reschedules
    DROP CONSTRAINT match_reschedules_match_id_fkey,
    ADD CONSTRAINT match_reschedules_match_id_fkey
        FOREIGN KEY (match_id) REFERENCES matches(id) ON DELETE CASCADE;
