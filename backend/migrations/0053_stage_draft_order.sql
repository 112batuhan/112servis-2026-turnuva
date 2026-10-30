-- The order a stage's bracket matches pick and ban in (crate::draft), as hosts write
-- it: "p1-p2-b2-b1-p1-*" is a pick, a pick, two bans, a pick, then picks taking turns.
-- 1 is the team that marks the first ban or pick on the referee page (the roll
-- decides it), 2 the other. Null until the hosts set it: the matches then ban twice
-- and pick in turn (draft::DEFAULT_ORDER).
ALTER TABLE stages
    ADD COLUMN draft_order TEXT
        CHECK (draft_order IS NULL OR draft_order ~ '^[pb][12](-[pb][12])*(-\*)?$');
