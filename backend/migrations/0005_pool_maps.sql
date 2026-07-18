-- Mods belong to individual maps (locked when the map is added), not to categories.
-- This introduces `pool_maps` — a map is a beatmap + mods + locked stats, sitting in
-- the generic pool (category_id NULL) or one category — replacing the old
-- generic_pool + pool_entries split, and drops the per-category modifier.
CREATE TABLE pool_maps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    beatmap_id BIGINT NOT NULL REFERENCES beatmaps(id) ON DELETE CASCADE,
    mods TEXT NOT NULL DEFAULT '',
    star_rating DOUBLE PRECISION NOT NULL,
    ar DOUBLE PRECISION NOT NULL,
    od DOUBLE PRECISION NOT NULL,
    cs DOUBLE PRECISION NOT NULL,
    hp DOUBLE PRECISION NOT NULL,
    bpm DOUBLE PRECISION NOT NULL,
    total_length INTEGER NOT NULL,
    category_id UUID REFERENCES stage_categories(id) ON DELETE SET NULL,
    position INTEGER NOT NULL DEFAULT 0,
    added_by UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Migrate categorised placements, taking each map's mods from its category's modifier.
-- Stats are the beatmap's nomod values — mod-accurate stats need an osu! API call, so
-- they resolve when a map is next re-added; the mod intent is preserved meanwhile.
INSERT INTO pool_maps (beatmap_id, mods, star_rating, ar, od, cs, hp, bpm, total_length,
                       category_id, position, added_by, created_at)
SELECT pe.beatmap_id, COALESCE(c.modifier, ''),
       b.star_rating, b.ar, b.od, b.cs, b.hp, b.bpm, b.total_length,
       pe.category_id, pe.position, pe.added_by, pe.created_at
FROM pool_entries pe
JOIN beatmaps b ON b.id = pe.beatmap_id
JOIN stage_categories c ON c.id = pe.category_id
WHERE pe.category_id IS NOT NULL;

-- Migrate the uncategorised generic-pool maps (nomod), skipping any already categorised.
INSERT INTO pool_maps (beatmap_id, mods, star_rating, ar, od, cs, hp, bpm, total_length,
                       category_id, position, added_by, created_at)
SELECT g.beatmap_id, '', b.star_rating, b.ar, b.od, b.cs, b.hp, b.bpm, b.total_length,
       NULL, 0, g.added_by, g.created_at
FROM generic_pool g
JOIN beatmaps b ON b.id = g.beatmap_id
WHERE NOT EXISTS (
    SELECT 1 FROM pool_entries pe WHERE pe.beatmap_id = g.beatmap_id AND pe.category_id IS NOT NULL
);

DROP TABLE pool_entries;
DROP TABLE generic_pool;

-- Categories are plain named groups now.
ALTER TABLE stage_categories DROP COLUMN modifier;
