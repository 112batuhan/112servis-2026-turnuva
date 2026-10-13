-- Team names, picked by the inviter and carried on the invite until it's accepted.
-- Existing teams are named after their players and existing invites after the inviter.
ALTER TABLE teams ADD COLUMN name TEXT;
UPDATE teams t
SET name = (
    SELECT string_agg(u.username, ' & ' ORDER BY u.username)
    FROM team_members m JOIN osu_users u ON u.id = m.user_id
    WHERE m.team_id = t.id
);
UPDATE teams SET name = 'Team ' || left(id::text, 8) WHERE name IS NULL;
ALTER TABLE teams ALTER COLUMN name SET NOT NULL;
CREATE UNIQUE INDEX teams_name_key ON teams (lower(name));

ALTER TABLE team_invites ADD COLUMN team_name TEXT;
UPDATE team_invites i SET team_name = u.username || '''s team'
FROM osu_users u WHERE u.id = i.inviter_id;
ALTER TABLE team_invites ALTER COLUMN team_name SET NOT NULL;
