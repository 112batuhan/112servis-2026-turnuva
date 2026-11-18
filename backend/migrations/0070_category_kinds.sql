-- What a pool category is, so the rules stop reading it off the category's name:
-- * nomod: a team may pick from it twice in a row;
-- * modded: a mod of its own (HD, HR, DT…), no two picks in a row from it by one team;
-- * freemod: the players choose their mods (HD, HR, EZ or FL, one per team at least);
-- * tiebreaker: freemod, nobody bans or picks it, played when the score is tied.
CREATE TYPE category_kind AS ENUM ('nomod', 'modded', 'freemod', 'tiebreaker');

-- The category's short name ("NM", "DT"): its slots are labelled with it ("NM1", "TB"),
-- the labels players type in the room to ban and pick. Letters only, 1-6 of them,
-- capitals (the handler upper-cases what's typed).
ALTER TABLE stage_categories
    ADD COLUMN abbreviation TEXT,
    ADD COLUMN kind category_kind NOT NULL DEFAULT 'modded';

-- Existing categories keep the labels and rules their names gave them: a spelled-out
-- mod ("DoubleTime") becomes its acronym, any other name its letters (so "NM", "HDHR"
-- stay as they were), cut to 6.
UPDATE stage_categories c SET
    abbreviation = CASE
        WHEN n.u ~ '^NO ?MOD' THEN 'NM'
        WHEN n.u ~ '^FREE ?MOD' THEN 'FM'
        WHEN n.u ~ '^TIE ?BREAKER' THEN 'TB'
        WHEN n.u ~ '^HIDDEN' THEN 'HD'
        WHEN n.u ~ '^HARD ?ROCK' THEN 'HR'
        WHEN n.u ~ '^DOUBLE ?TIME' THEN 'DT'
        WHEN n.u ~ '^EASY' THEN 'EZ'
        WHEN n.u ~ '^FLASH ?LIGHT' THEN 'FL'
        WHEN n.u ~ '^HALF ?TIME' THEN 'HT'
        WHEN n.letters <> '' THEN left(n.letters, 6)
        ELSE 'X'
    END,
    kind = CASE
        WHEN n.u ~ '^(NM|NO ?MOD)' THEN 'nomod'
        WHEN n.u ~ '^(TB|TIE ?BREAKER)' THEN 'tiebreaker'
        WHEN n.u ~ '^(FM|FREE ?MOD)' THEN 'freemod'
        ELSE 'modded'
    END::category_kind
FROM (
    SELECT id, upper(trim(name)) AS u,
           regexp_replace(upper(name), '[^A-Z]', '', 'g') AS letters
    FROM stage_categories
) n
WHERE n.id = c.id;

ALTER TABLE stage_categories
    ALTER COLUMN abbreviation SET NOT NULL,
    ADD CONSTRAINT stage_categories_abbreviation_check
        CHECK (abbreviation ~ '^[A-Z]{1,6}$');

-- A map's mods can now also be FM or TB: a beatmap added for a freemod or tiebreaker
-- slot. Its stats are the nomod ones (an FM map's also under HR), and the room plays it
-- with Freemod. Maps already sitting in such a category with no mods become one.
UPDATE pool_maps pm SET mods = CASE c.kind WHEN 'tiebreaker' THEN 'TB' ELSE 'FM' END
FROM category_slots s
JOIN stage_categories c ON c.id = s.category_id
WHERE pm.slot_id = s.id AND pm.mods = '' AND c.kind IN ('freemod', 'tiebreaker');
