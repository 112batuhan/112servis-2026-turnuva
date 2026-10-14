-- Removes every dummy account (negative osu_id) and anything hanging off it.
-- Teams are deleted whole, so a real account teamed with a dummy is freed too, and
-- so are matches a dummy refereed, streamed or played in.
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
   );
DELETE FROM teams
WHERE id IN (
    SELECT m.team_id FROM team_members m JOIN osu_users u ON u.id = m.user_id WHERE u.osu_id < 0
);
DELETE FROM osu_users WHERE osu_id < 0;
