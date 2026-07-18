-- Per-category colour used to colour-code the pool. Stored as a #rrggbb hex string.
ALTER TABLE stage_categories ADD COLUMN color TEXT NOT NULL DEFAULT '#6b7280';
