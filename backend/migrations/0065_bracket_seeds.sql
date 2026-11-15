-- The order hosts seed the bracket in, from the qualifier's results (/admin/seeding).
-- Left empty, the qualifier's order is the seeding; a team not listed comes after the
-- listed ones, in the qualifier's order. Making the bracket from it keeps it, as the
-- record of who got which seed.
CREATE TABLE bracket_seeds (
    team_id UUID PRIMARY KEY REFERENCES teams(id) ON DELETE CASCADE,
    position INT NOT NULL
);
