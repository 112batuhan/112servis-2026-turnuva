-- Categories have a size: a set of slots. Each slot holds at most one beatmap and
-- carries editor-only notes (never shown on the public page).
CREATE TABLE category_slots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id UUID NOT NULL REFERENCES stage_categories(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    editor_notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Maps are assigned to slots, not directly to categories. slot_id NULL = generic pool;
-- otherwise the map fills that slot. At most one map per slot (UNIQUE); deleting a slot
-- sends its map back to the generic pool (ON DELETE SET NULL).
ALTER TABLE pool_maps ADD COLUMN slot_id UUID REFERENCES category_slots(id) ON DELETE SET NULL;
ALTER TABLE pool_maps ADD CONSTRAINT pool_maps_slot_id_key UNIQUE (slot_id);
ALTER TABLE pool_maps DROP COLUMN category_id;
