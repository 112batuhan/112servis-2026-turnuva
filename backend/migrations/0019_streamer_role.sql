-- Streamers broadcast bracket matches. They sit between basic and referee on the role
-- ladder (role.rs) and, unlike the rest of the staff, can still play.
ALTER TYPE user_role ADD VALUE 'streamer' BEFORE 'basic';

-- The second match kind. Its tables are in the next migration, since a new enum value
-- can't be used in the transaction that adds it.
ALTER TYPE match_kind ADD VALUE 'bracket';
