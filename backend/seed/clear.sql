-- Removes every dummy account (negative osu_id) and anything hanging off it.
-- Teams are deleted whole, so a real account teamed with a dummy is freed too.
DELETE FROM teams
WHERE id IN (
    SELECT m.team_id FROM team_members m JOIN osu_users u ON u.id = m.user_id WHERE u.osu_id < 0
);
DELETE FROM osu_users WHERE osu_id < 0;
