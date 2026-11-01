-- Qualifier lobbies' saved results and osu!'s accuracy move into the tables bracket
-- matches already save theirs in (0057 kept them apart). match_map_results now holds any
-- match's finished pool maps, a lobby's without team totals (its teams don't play each
-- other), and each player score carries its accuracy.
ALTER TABLE match_map_results
    DROP CONSTRAINT match_map_results_match_id_fkey,
    ADD CONSTRAINT match_map_results_match_id_fkey
        FOREIGN KEY (match_id) REFERENCES matches(id) ON DELETE CASCADE,
    ALTER COLUMN team1_score DROP NOT NULL,
    ALTER COLUMN team2_score DROP NOT NULL,
    ADD CONSTRAINT match_map_results_totals CHECK ((team1_score IS NULL) = (team2_score IS NULL));

-- 0..1, from osu!'s record of the room (the chat doesn't say it); null until it's read.
-- Saving a match's maps again keeps it for the scores that stay the same.
ALTER TABLE match_map_scores ADD COLUMN accuracy DOUBLE PRECISION;

INSERT INTO match_map_results (match_id, position, pool_map_id)
SELECT match_id, position, pool_map_id FROM qualifier_map_results;

INSERT INTO match_map_scores (match_id, position, user_id, score, mods, passed)
SELECT match_id, position, user_id, score, mods, passed FROM qualifier_map_scores;

UPDATE match_map_scores s
SET accuracy = rs.accuracy
FROM match_map_results r, pool_maps pm, osu_users u, match_rooms mr, room_scores rs
WHERE r.match_id = s.match_id AND r.position = s.position
  AND pm.id = r.pool_map_id
  AND u.id = s.user_id
  AND mr.match_id = s.match_id
  AND rs.mp_id = mr.mp_id AND rs.beatmap_id = pm.beatmap_id
  AND rs.osu_id = u.osu_id AND rs.score = s.score;

DROP TABLE qualifier_map_scores, qualifier_map_results, room_scores;
