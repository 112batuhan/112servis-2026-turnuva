-- A pairing that can no longer happen (or that its bracket match took over) is kept,
-- marked removed, so it comes back as it was (time, staff, proposals, predictions and
-- its Discord channel) when the match that decided it is reopened. Only deleting its
-- bracket match or one of its teams deletes the row.
ALTER TABLE potential_matches ADD COLUMN removed_at TIMESTAMPTZ;
