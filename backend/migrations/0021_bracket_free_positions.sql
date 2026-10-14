-- Bracket matches move freely on the editor's board instead of taking one grid cell
-- each. Positions are in board units (20px); cards are 12x5 units and may sit anywhere,
-- so cells are no longer unique. Old cells are spread out by a card plus a gap.
ALTER TABLE bracket_matches DROP CONSTRAINT bracket_matches_cell_key;
ALTER TABLE bracket_matches RENAME COLUMN grid_col TO pos_x;
ALTER TABLE bracket_matches RENAME COLUMN grid_row TO pos_y;
UPDATE bracket_matches SET pos_x = pos_x * 15, pos_y = pos_y * 6;
