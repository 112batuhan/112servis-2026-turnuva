-- The stage a bracket match is played in, so its map pool is known. Deleting a stage
-- leaves its matches without one.
ALTER TABLE bracket_matches ADD COLUMN stage_id UUID REFERENCES stages(id) ON DELETE SET NULL;
