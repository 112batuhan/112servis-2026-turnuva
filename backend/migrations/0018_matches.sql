-- Matches run by a referee. Only qualifier lobbies exist for now; other kinds will
-- add their own value and tables.
CREATE TYPE match_kind AS ENUM ('qualifier');

CREATE TABLE matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind match_kind NOT NULL,
    starts_at TIMESTAMPTZ NOT NULL,
    referee_id UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    -- The osu! multiplayer room: osu.ppy.sh/community/matches/<mp_id>.
    mp_id BIGINT,
    -- Staff-only; never sent to the public routes.
    referee_notes TEXT NOT NULL DEFAULT '',
    concluded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX matches_kind_starts_at_idx ON matches (kind, starts_at);

-- The qualifier lobby a team signed up for. team_id as the primary key keeps every
-- team in at most one lobby.
CREATE TABLE qualifier_entries (
    team_id UUID PRIMARY KEY REFERENCES teams(id) ON DELETE CASCADE,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX qualifier_entries_match_id_idx ON qualifier_entries (match_id);
