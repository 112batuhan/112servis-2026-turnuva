-- Users the hosts banned from taking part: they can still sign in and look around, but
-- can't form or change a team, join a qualifier lobby, reschedule or predict. One row per
-- banned user; unbanning deletes it.
CREATE TABLE user_bans (
    user_id    UUID PRIMARY KEY REFERENCES osu_users(id) ON DELETE CASCADE,
    -- Shown to the banned user; empty when the host gave none.
    reason     TEXT NOT NULL DEFAULT '',
    banned_by  UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
