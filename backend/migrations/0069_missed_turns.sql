-- The bans and picks a bracket match's teams let run out on the auto referee's clock
-- (bancho/autoref). A missed ban takes its step of the stage's order with no map banned;
-- a missed pick passes that step to the other team. A referee can take one back.
CREATE TYPE draft_step_kind AS ENUM ('ban', 'pick');
CREATE TABLE match_missed_turns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    kind draft_step_kind NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX match_missed_turns_match_idx ON match_missed_turns (match_id, created_at);
