-- The third match kind: a pairing a bracket match may still turn out to be, played the
-- same week as the matches that decide it. Its tables are in the next migration, since
-- a new enum value can't be used in the transaction that adds it.
ALTER TYPE match_kind ADD VALUE 'potential';
