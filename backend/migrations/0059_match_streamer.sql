-- Who streams a match sat on each kind's own table (bracket_matches, potential_matches)
-- while its referee was on matches. Both are on matches now, so taking, giving up and
-- assigning a stream is one piece of code for every kind, like refereeing. Qualifier
-- lobbies have no streamer.
ALTER TABLE matches
    ADD COLUMN streamer_id UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    ADD CONSTRAINT matches_streamer_kind_check CHECK (streamer_id IS NULL OR kind <> 'qualifier');

UPDATE matches m SET streamer_id = b.streamer_id
FROM bracket_matches b
WHERE b.match_id = m.id AND b.streamer_id IS NOT NULL;

UPDATE matches m SET streamer_id = p.streamer_id
FROM potential_matches p
WHERE p.match_id = m.id AND p.streamer_id IS NOT NULL;

ALTER TABLE bracket_matches DROP COLUMN streamer_id;
ALTER TABLE potential_matches DROP COLUMN streamer_id;
