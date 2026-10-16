-- The finished pool maps that count towards a bracket match's score, with each team's
-- summed score, as the referee page read them (from the room's chat, or from osu! when
-- the referee synced them from the room's link). Saved with the score and replaced as
-- a whole, so the results post needs nothing but the site's own data. The same map can
-- be played twice, so rows are keyed by their place in the match.
CREATE TABLE match_map_results (
    match_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    position INT NOT NULL,
    pool_map_id UUID NOT NULL REFERENCES pool_maps(id) ON DELETE CASCADE,
    team1_score BIGINT NOT NULL,
    team2_score BIGINT NOT NULL,
    PRIMARY KEY (match_id, position)
);

-- status_message_id: the match channel's latest "concluded" or "reopened" message,
-- deleted when the next one goes out, so a reopened match doesn't keep its old result.
-- archived_for: the conclusion the channel was archived for. A concluded match's
-- channel stays open for an hour so the players can still talk, then the scheduler
-- archives it.
ALTER TABLE match_discord
    ADD COLUMN status_message_id BIGINT,
    ADD COLUMN archived_for TIMESTAMPTZ;

-- Channels of matches concluded before this were archived right away.
UPDATE match_discord md SET archived_for = m.concluded_at
FROM matches m
WHERE m.id = md.match_id AND m.concluded_at IS NOT NULL;
