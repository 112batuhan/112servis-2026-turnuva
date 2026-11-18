-- Auto referee test with real people, run on its own (no reseed): four accounts, real
-- or dummies to fill in (`dev.autoref_players`, team 1's two then team 2's, set by
-- db/dev.rs, which has checked them) in two teams, in a bracket match of
-- `dev.autoref_stage` starting in 10 minutes, so the room opens at once while auto
-- rooms are on (/admin/bancho) and the auto referee runs it. No referee: the staff
-- watch it from the referee page. The teams are known by their names, and the test
-- made before goes first; clear.sql removes them too (keep the names in step with it
-- and AUTOREF_TEAMS in db/dev.rs). A dummy in them makes clear.sql's own rules remove
-- the test as well.
DELETE FROM room_chat_messages
WHERE mp_id IN (
    SELECT r.mp_id FROM match_rooms r
    JOIN bracket_matches b ON b.match_id = r.match_id
    JOIN teams t ON t.id IN (b.team1_id, b.team2_id)
    WHERE t.name IN ('Oto Hakem Test Mavi', 'Oto Hakem Test Kırmızı')
);
DELETE FROM matches m
USING bracket_matches b, teams t
WHERE b.match_id = m.id
  AND t.id IN (b.team1_id, b.team2_id)
  AND t.name IN ('Oto Hakem Test Mavi', 'Oto Hakem Test Kırmızı');
DELETE FROM teams WHERE name IN ('Oto Hakem Test Mavi', 'Oto Hakem Test Kırmızı');

DO $$
DECLARE
    players uuid[] := string_to_array(current_setting('dev.autoref_players'), ',')::uuid[];
    stage uuid := current_setting('dev.autoref_stage')::uuid;
    home uuid;
    away uuid;
    new_match uuid;
BEGIN
    INSERT INTO teams (name) VALUES ('Oto Hakem Test Mavi') RETURNING id INTO home;
    INSERT INTO team_members (user_id, team_id) VALUES (players[1], home), (players[2], home);
    INSERT INTO teams (name) VALUES ('Oto Hakem Test Kırmızı') RETURNING id INTO away;
    INSERT INTO team_members (user_id, team_id) VALUES (players[3], away), (players[4], away);

    INSERT INTO matches (kind, starts_at)
    VALUES ('bracket', date_trunc('second', now()) + interval '10 minutes')
    RETURNING id INTO new_match;
    -- Placed right of the bracket, like live.sql's.
    INSERT INTO bracket_matches (match_id, side, number, pos_x, pos_y, team1_id, team2_id, stage_id)
    VALUES (
        new_match,
        'winners',
        (SELECT COALESCE(max(number), 0) + 1 FROM bracket_matches WHERE side = 'winners'),
        (SELECT COALESCE(max(pos_x), 0) + 15 FROM bracket_matches),
        (SELECT COALESCE(min(pos_y), 0) FROM bracket_matches),
        home,
        away,
        stage
    );
END $$;
