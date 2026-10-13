-- When each user's rank was last pulled from osu!, so lookups can skip fresh ones.
ALTER TABLE osu_users ADD COLUMN rank_updated_at TIMESTAMPTZ;
UPDATE osu_users SET rank_updated_at = updated_at;

-- Site-wide settings, a single row.
CREATE TABLE app_settings (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    -- Last full refresh of every team member's rank.
    ranks_updated_at TIMESTAMPTZ
);
INSERT INTO app_settings DEFAULT VALUES;
