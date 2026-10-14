-- The editor's board has no edges, so positions can be negative too.
ALTER TABLE bracket_matches DROP CONSTRAINT bracket_matches_grid_col_check;
ALTER TABLE bracket_matches DROP CONSTRAINT bracket_matches_grid_row_check;
