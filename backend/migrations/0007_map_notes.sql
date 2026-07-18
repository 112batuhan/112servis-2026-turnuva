-- Per-map notes: public_notes is shown on the public page; editor_notes is editor-only
-- (never sent to the public API).
ALTER TABLE pool_maps ADD COLUMN public_notes TEXT NOT NULL DEFAULT '';
ALTER TABLE pool_maps ADD COLUMN editor_notes TEXT NOT NULL DEFAULT '';
