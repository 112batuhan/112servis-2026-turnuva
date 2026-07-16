CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Roles are additive; to introduce a new one later, add it in its own migration:
--   ALTER TYPE user_role ADD VALUE 'new_role';
CREATE TYPE user_role AS ENUM ('host', 'map_pooler', 'basic');

CREATE TABLE osu_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    osu_id BIGINT UNIQUE NOT NULL,
    username TEXT NOT NULL,
    avatar_url TEXT NOT NULL,
    country_code TEXT,
    global_rank BIGINT,
    country_rank BIGINT,
    role user_role NOT NULL DEFAULT 'basic',
    -- Bumped whenever a user's role changes; embedded in their JWT so existing
    -- tokens can be detected as stale and rejected (see db::user::set_role).
    token_version INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Bootstrap the first host by hand (there's no host yet to grant it via the API):
--   UPDATE osu_users SET role = 'host', token_version = token_version + 1 WHERE osu_id = <your_osu_id>;

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
