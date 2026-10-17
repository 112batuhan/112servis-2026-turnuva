-- Qualifier lobbies get a fixed name instead of a number by start time, which shifted
-- whenever a lobby was added, moved or deleted while its osu! room, Discord channel
-- and announcements kept the old one. lobby_index is its letter (1 = A, 26 = Z,
-- 27 = AA), the next one after the last lobby's when it's created; null for bracket
-- matches.
ALTER TABLE matches ADD COLUMN lobby_index INT;

UPDATE matches m SET lobby_index = n.index
FROM (
    SELECT id, row_number() OVER (ORDER BY starts_at, created_at) AS index
    FROM matches
    WHERE kind = 'qualifier'
) n
WHERE n.id = m.id;

ALTER TABLE matches ADD CONSTRAINT matches_lobby_index_check
    CHECK ((kind = 'qualifier') = (lobby_index IS NOT NULL));
ALTER TABLE matches ADD CONSTRAINT matches_lobby_index_positive CHECK (lobby_index > 0);

CREATE UNIQUE INDEX matches_lobby_index_key ON matches (lobby_index);
