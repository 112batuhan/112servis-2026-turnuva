-- How many maps a stage's bracket matches are played to (Bo9: the first team to win 5
-- takes it, and at 4-4 the tiebreaker is played). Odd, so a match can't end level; null
-- until the hosts set it (the qualifier stage has none).
ALTER TABLE stages
    ADD COLUMN best_of INTEGER CHECK (best_of IS NULL OR (best_of BETWEEN 1 AND 25 AND best_of % 2 = 1));
