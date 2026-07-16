CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE osu_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osu_id BIGINT UNIQUE NOT NULL,
    username TEXT NOT NULL,
    avatar_url TEXT NOT NULL,
    country_code TEXT,
    global_rank BIGINT,
    country_rank BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- At most one Discord account per osu! user (osu_user_id UNIQUE) and vice versa (discord_id UNIQUE).
CREATE TABLE discord_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osu_user_id UUID UNIQUE NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    discord_id TEXT UNIQUE NOT NULL,
    username TEXT NOT NULL,
    avatar TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
