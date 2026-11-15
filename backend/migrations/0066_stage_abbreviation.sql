-- A stage's short name ("QF", "GF"): the bracket board writes it on each match card's
-- head, with the full name on hover. Null: none, and the card shows only its code.
ALTER TABLE stages
    ADD COLUMN abbreviation TEXT
        CHECK (abbreviation IS NULL OR char_length(abbreviation) BETWEEN 1 AND 6);
