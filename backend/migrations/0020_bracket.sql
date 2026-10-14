-- Bracket matches can be laid out before they're scheduled, so only qualifier lobbies
-- still need a start time.
ALTER TABLE matches ALTER COLUMN starts_at DROP NOT NULL;
ALTER TABLE matches ADD CONSTRAINT matches_qualifier_starts_at_check
    CHECK (kind <> 'qualifier' OR starts_at IS NOT NULL);

CREATE TYPE bracket_side AS ENUM ('winners', 'losers');

-- The bracket half of a 'bracket' match. Each one is numbered within its side (shown
-- as W1, L3, ...) and sits in one cell of the editor's grid. The unique constraints are
-- deferrable so one statement can swap two matches' numbers or cells.
CREATE TABLE bracket_matches (
    match_id UUID PRIMARY KEY REFERENCES matches(id) ON DELETE CASCADE,
    side bracket_side NOT NULL,
    number INT NOT NULL CHECK (number > 0),
    grid_col INT NOT NULL CHECK (grid_col >= 0),
    grid_row INT NOT NULL CHECK (grid_row >= 0),
    team1_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    team2_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    team1_score INT CHECK (team1_score >= 0),
    team2_score INT CHECK (team2_score >= 0),
    streamer_id UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    CONSTRAINT bracket_matches_code_key UNIQUE (side, number) DEFERRABLE,
    CONSTRAINT bracket_matches_cell_key UNIQUE (grid_col, grid_row) DEFERRABLE
);

CREATE TYPE bracket_outcome AS ENUM ('winner', 'loser');

-- Where a match's winner and loser go next. The primary key gives each outcome one
-- destination and the unique key gives each team slot one source. Concluding the
-- source match fills the target slot (handlers/bracket.rs).
CREATE TABLE bracket_links (
    source_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    outcome bracket_outcome NOT NULL,
    target_id UUID NOT NULL REFERENCES bracket_matches(match_id) ON DELETE CASCADE,
    slot SMALLINT NOT NULL CHECK (slot IN (1, 2)),
    PRIMARY KEY (source_id, outcome),
    CONSTRAINT bracket_links_target_key UNIQUE (target_id, slot),
    CHECK (source_id <> target_id)
);
