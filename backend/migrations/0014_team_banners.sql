-- Team banner images, kept out of `teams` so listing teams never loads the bytes.
CREATE TABLE team_banners (
    team_id UUID PRIMARY KEY REFERENCES teams(id) ON DELETE CASCADE,
    content_type TEXT NOT NULL,
    data BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
