-- Removes every dummy account (negative osu_id) and anything hanging off it.
-- Teams are deleted whole, so a real account teamed with a dummy is freed too, and
-- so are matches a dummy refereed, streamed or played in. Seeded stages go too while
-- they're still empty. A deleted match's room chat goes with it.
WITH gone AS (
    DELETE FROM matches m
    WHERE EXISTS (SELECT 1 FROM osu_users u WHERE u.id = m.referee_id AND u.osu_id < 0)
       OR EXISTS (
           SELECT 1 FROM qualifier_entries e
           JOIN team_members tm ON tm.team_id = e.team_id
           JOIN osu_users u ON u.id = tm.user_id
           WHERE e.match_id = m.id AND u.osu_id < 0
       )
       OR EXISTS (
           SELECT 1 FROM bracket_matches b
           JOIN osu_users u ON u.id = b.streamer_id
           WHERE b.match_id = m.id AND u.osu_id < 0
       )
       OR EXISTS (
           SELECT 1 FROM bracket_matches b
           JOIN team_members tm ON tm.team_id IN (b.team1_id, b.team2_id)
           JOIN osu_users u ON u.id = tm.user_id
           WHERE b.match_id = m.id AND u.osu_id < 0
       )
    RETURNING m.mp_id
)
DELETE FROM room_chat_messages WHERE mp_id IN (SELECT mp_id FROM gone);
DELETE FROM teams
WHERE id IN (
    SELECT m.team_id FROM team_members m JOIN osu_users u ON u.id = m.user_id WHERE u.osu_id < 0
);
DELETE FROM osu_users WHERE osu_id < 0;
-- The stages seed.sql adds, while they're still empty: one with categories is
-- someone's work, so it stays. Keep the names in step with seed.sql.
DELETE FROM stages s
WHERE s.name IN ('Qualifiers', 'Round of 32', 'Round of 16', 'Quarterfinals', 'Semifinals',
                 'Winners Final', 'Losers Final', 'Grand Final')
  AND NOT EXISTS (SELECT 1 FROM stage_categories c WHERE c.stage_id = s.id);
