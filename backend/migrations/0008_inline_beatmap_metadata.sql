-- Store maps self-contained in pool_maps: each row keeps the metadata and the
-- mod-adjusted stats it was added with. The nomod `beatmaps` cache is dropped — mod
-- stats are computed from the osu! API response at add time, so the nomod values need no
-- persistence, and the same map under different mods is simply separate pool_maps rows.
ALTER TABLE pool_maps
    ADD COLUMN beatmapset_id BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN artist TEXT NOT NULL DEFAULT '',
    ADD COLUMN title TEXT NOT NULL DEFAULT '',
    ADD COLUMN version TEXT NOT NULL DEFAULT '',
    ADD COLUMN creator TEXT,
    ADD COLUMN mode TEXT NOT NULL DEFAULT '',
    ADD COLUMN cover_url TEXT;

-- Backfill existing placements from the cache before dropping it.
UPDATE pool_maps pm SET
    beatmapset_id = b.beatmapset_id,
    artist = b.artist,
    title = b.title,
    version = b.version,
    creator = b.creator,
    mode = b.mode,
    cover_url = b.cover_url
FROM beatmaps b
WHERE b.id = pm.beatmap_id;

-- The defaults existed only to backfill existing rows; new inserts always supply values.
-- beatmap_id stays as the plain osu! id (used to link back to osu.ppy.sh), sans FK.
ALTER TABLE pool_maps
    ALTER COLUMN beatmapset_id DROP DEFAULT,
    ALTER COLUMN artist DROP DEFAULT,
    ALTER COLUMN title DROP DEFAULT,
    ALTER COLUMN version DROP DEFAULT,
    ALTER COLUMN mode DROP DEFAULT;

ALTER TABLE pool_maps DROP CONSTRAINT pool_maps_beatmap_id_fkey;

DROP TABLE beatmaps;
