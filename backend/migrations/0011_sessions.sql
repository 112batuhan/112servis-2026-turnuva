-- Server-side login sessions, replacing JWTs. The cookie holds a random token; only
-- its SHA-256 is stored, so a leaked database doesn't hand out usable sessions.
-- Expired rows are swept by a background task (see auth::spawn_session_cleanup).
CREATE TABLE sessions (
    token_hash BYTEA PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

-- Role and Discord status are now read from the database on every request, so the
-- JWT revocation counter has nothing left to do.
ALTER TABLE osu_users DROP COLUMN token_version;
