-- Times a bracket match's teams propose to each other for it. A player of either team
-- proposes (as many times as they like, each a separate row); any player of the other
-- team accepts one, which moves the match there and drops every other proposal for it
-- (handlers/reschedule.rs). A team proposes a time at most once per match.
CREATE TABLE match_reschedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    proposed_by UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    starts_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT match_reschedules_time_key UNIQUE (match_id, team_id, starts_at)
);
