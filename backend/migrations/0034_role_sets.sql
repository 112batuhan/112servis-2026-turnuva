-- Roles become a set instead of a ladder: a user holds any of host, map pooler, referee
-- and streamer, and none means a regular player. Host still opens everything, but only
-- the users who hold referee or streamer are offered for, take and are pinged about
-- those slots. Everyone keeps exactly the one role they had.
CREATE TYPE staff_role AS ENUM ('host', 'map_pooler', 'referee', 'streamer');

ALTER TABLE osu_users ADD COLUMN roles staff_role[] NOT NULL DEFAULT '{}';
UPDATE osu_users SET roles = ARRAY[role::text::staff_role] WHERE role <> 'basic';
ALTER TABLE osu_users DROP COLUMN role;
DROP TYPE user_role;
