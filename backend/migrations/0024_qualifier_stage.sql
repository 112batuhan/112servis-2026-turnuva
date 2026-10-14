-- The stage whose map pool the qualifier lobbies play. At most one stage is it, so
-- referee tools can look it up without asking.
ALTER TABLE stages ADD COLUMN is_qualifier BOOLEAN NOT NULL DEFAULT false;
CREATE UNIQUE INDEX stages_qualifier_key ON stages (is_qualifier) WHERE is_qualifier;
