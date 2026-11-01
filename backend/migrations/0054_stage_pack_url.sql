-- Where a stage's beatmap pack can be downloaded: a link to wherever the map poolers
-- uploaded it (the site stores no files). The public mappool page offers it as
-- "Beatmap pack indir". Null: no pack yet.
ALTER TABLE stages
    ADD COLUMN pack_url TEXT
        CHECK (pack_url IS NULL OR (pack_url ~ '^https?://' AND length(pack_url) <= 2048));
