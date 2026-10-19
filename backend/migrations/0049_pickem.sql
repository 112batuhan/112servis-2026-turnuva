-- Pick'em: players predict each bracket match of a stage (the winner and the score)
-- before the stage starts. A stage's pick'em opens once every match of the stage before
-- it is concluded and locks when its first match starts (handlers/pickem.rs); hosts can
-- open it early or lock it by hand.
CREATE TYPE pickem_mode AS ENUM ('auto', 'open', 'closed');

-- 'open' still locks once a match of the stage starts; 'closed' locks it at once.
ALTER TABLE stages ADD COLUMN pickem_mode pickem_mode NOT NULL DEFAULT 'auto';

-- One prediction per user per match: the team they think wins and the score they think
-- it ends on, the winner's maps first. The team rather than its slot, so a team moved
-- out of the match by a host doesn't leave the prediction on whoever took its place.
CREATE TABLE pickem_predictions (
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    winner_score INT NOT NULL CHECK (winner_score > 0),
    loser_score INT NOT NULL CHECK (loser_score >= 0 AND loser_score < winner_score),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, match_id)
);

CREATE INDEX pickem_predictions_match_idx ON pickem_predictions (match_id);
