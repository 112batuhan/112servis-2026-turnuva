-- A nomod map's stats under HR, for freemod picks, where a player may take HR: the star
-- rating from osu! and CS, AR and OD computed like the mod stats (BPM and length don't
-- change). Filled in when a nomod map is added; null for maps added with mods, and for
-- maps added before this until their stats are recomputed.
ALTER TABLE pool_maps
    ADD COLUMN hr_star_rating DOUBLE PRECISION,
    ADD COLUMN hr_cs DOUBLE PRECISION,
    ADD COLUMN hr_ar DOUBLE PRECISION,
    ADD COLUMN hr_od DOUBLE PRECISION;
