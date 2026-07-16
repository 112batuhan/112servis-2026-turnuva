-- A beatmap now lives in exactly one place: the generic pool, or one category in
-- one stage. Previously UNIQUE was per-stage, so the same map could be categorised
-- in several stages at once (and the generic pool only hid it in the current one).

-- Collapse any existing multi-stage placements to a single one, keeping the earliest.
DELETE FROM pool_entries pe
WHERE EXISTS (
    SELECT 1 FROM pool_entries other
    WHERE other.beatmap_id = pe.beatmap_id
      AND (other.created_at < pe.created_at
           OR (other.created_at = pe.created_at AND other.id < pe.id))
);

-- Enforce single placement across the whole tournament.
ALTER TABLE pool_entries DROP CONSTRAINT IF EXISTS pool_entries_stage_id_beatmap_id_key;
ALTER TABLE pool_entries ADD CONSTRAINT pool_entries_beatmap_id_key UNIQUE (beatmap_id);
