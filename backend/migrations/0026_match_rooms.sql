-- A match's osu! multiplayer rooms, replacing matches.mp_id. Usually one, but a room
-- that closes or breaks mid-match is replaced by a new one, and the old one stays on
-- record with its chat. A room belongs to one match.
CREATE TABLE match_rooms (
    mp_id BIGINT PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX match_rooms_match_id_idx ON match_rooms (match_id, added_at);

-- Should two matches share a room, the older match keeps it.
INSERT INTO match_rooms (mp_id, match_id, added_at)
SELECT DISTINCT ON (mp_id) mp_id, id, created_at
FROM matches
WHERE mp_id IS NOT NULL
ORDER BY mp_id, created_at;

ALTER TABLE matches DROP COLUMN mp_id;
