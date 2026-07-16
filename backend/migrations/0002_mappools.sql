-- Tournament stages — one map pool per stage (e.g. a week / round).
CREATE TABLE stages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Mod categories within a stage (e.g. NoMod, Hidden, DoubleTime). `modifier` is an
-- optional mod acronym (DT, HR, ...) whose stat effects are applied when displaying
-- the maps in that category; NULL means the maps show their nomod stats.
CREATE TABLE stage_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stage_id UUID NOT NULL REFERENCES stages(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    modifier TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Beatmaps pulled from the osu! API, cached so a pool doesn't depend on the API
-- staying reachable. Stats here are nomod; modifiers are applied at display time.
CREATE TABLE beatmaps (
    id BIGINT PRIMARY KEY,          -- osu! beatmap id
    beatmapset_id BIGINT NOT NULL,
    artist TEXT NOT NULL,
    title TEXT NOT NULL,
    version TEXT NOT NULL,          -- difficulty name
    creator TEXT,
    star_rating DOUBLE PRECISION NOT NULL,
    bpm DOUBLE PRECISION NOT NULL,
    total_length INTEGER NOT NULL,  -- seconds
    cs DOUBLE PRECISION NOT NULL,
    ar DOUBLE PRECISION NOT NULL,
    od DOUBLE PRECISION NOT NULL,
    hp DOUBLE PRECISION NOT NULL,
    mode TEXT NOT NULL,
    cover_url TEXT,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The generic pool: a global, cross-stage library of candidate beatmaps. Adding a
-- map here makes it available to categorise in every stage.
CREATE TABLE generic_pool (
    beatmap_id BIGINT PRIMARY KEY REFERENCES beatmaps(id) ON DELETE CASCADE,
    added_by UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- A categorised placement of a library map within one stage. Each stage draws from
-- the shared generic pool above; a map appears at most once per stage
-- (UNIQUE stage_id, beatmap_id). ON DELETE SET NULL is a safety net — categories are
-- normally deleted together with their entries in the app layer.
CREATE TABLE pool_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stage_id UUID NOT NULL REFERENCES stages(id) ON DELETE CASCADE,
    beatmap_id BIGINT NOT NULL REFERENCES beatmaps(id),
    category_id UUID REFERENCES stage_categories(id) ON DELETE SET NULL,
    position INTEGER NOT NULL DEFAULT 0,
    added_by UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (stage_id, beatmap_id)
);
