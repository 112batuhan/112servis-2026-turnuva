-- The tournament just after its qualifier, before the bracket: run after model.sql and
-- seed.sql (and model.sql again, since seed.sql drops its functions at its end). It
-- takes seed.sql's bracket away and plays every team's qualifier instead, so the
-- seeding page (/admin/seeding) has a complete qualifier to seed a Round of 32 or a
-- Round of 16 from, and teams under the cut to leave out.
--
-- 40 dummy teams: seed.sql's 32 and eight more. 39 of them played one of six concluded
-- lobbies (A-F, seven teams each but the last); Kayıp Rank never signed up, so the
-- qualifier gives it no place. Lobby A keeps seed.sql's chat and results; the rest get
-- results made up by the same model, every map of the qualifier pool once. Rank Uçtu
-- missed the pool's last two maps and Gece Kuşları didn't show up at all, so their
-- missing maps count as a 0. Yol Hediyesi's Cezerye is still banned, so the seeding
-- puts that team last. Everything seed.sql adds besides the bracket (stages and pools,
-- invites, notices, the ban) stays.

-- seed.sql's bracket matches and their potential matches, with their rooms' chat.
WITH gone AS (
    DELETE FROM matches m
    WHERE m.kind IN ('bracket', 'potential')
      AND (
          EXISTS (SELECT 1 FROM osu_users u WHERE u.id = m.referee_id AND u.osu_id < 0)
          OR EXISTS (SELECT 1 FROM osu_users u WHERE u.id = m.streamer_id AND u.osu_id < 0)
          OR EXISTS (
              SELECT 1 FROM bracket_matches b
              JOIN team_members tm ON tm.team_id IN (b.team1_id, b.team2_id)
              JOIN osu_users u ON u.id = tm.user_id
              WHERE b.match_id = m.id AND u.osu_id < 0
          )
          OR EXISTS (
              SELECT 1 FROM potential_matches p
              JOIN team_members tm ON tm.team_id IN (p.team1_id, p.team2_id)
              JOIN osu_users u ON u.id = tm.user_id
              WHERE p.match_id = m.id AND u.osu_id < 0
          )
      )
    RETURNING m.id
)
DELETE FROM room_chat_messages
WHERE mp_id IN (SELECT r.mp_id FROM match_rooms r WHERE r.match_id IN (SELECT id FROM gone));

-- No stage is being played yet, so no pick'em is opened by hand.
UPDATE stages SET pickem_mode = 'auto' WHERE name = 'Semifinals' AND NOT is_qualifier;

-- The eight teams that only played the qualifier, each within the rank limit.
INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, roles) VALUES
    (-251, 'Midyeci',     'https://a.ppy.sh/', 'TR', 4100,  43,  '{}'),
    (-252, 'Balikekmek',  'https://a.ppy.sh/', 'TR', 31000, 326, '{}'),
    (-253, 'Kumru',       'https://a.ppy.sh/', 'TR', 6800,  71,  '{}'),
    (-254, 'Boyoz',       'https://a.ppy.sh/', 'TR', 27500, 289, '{}'),
    (-255, 'Zeytinyagli', 'https://a.ppy.sh/', 'TR', 9400,  98,  '{}'),
    (-256, 'Enginar',     'https://a.ppy.sh/', 'TR', 16800, 176, '{}'),
    (-257, 'Salep',       'https://a.ppy.sh/', 'TR', 12100, 127, '{}'),
    (-258, 'Boza',        'https://a.ppy.sh/', 'TR', 15200, 160, '{}'),
    (-259, 'Simitci',     'https://a.ppy.sh/', 'TR', 3600,  38,  '{}'),
    (-260, 'Kasar',       'https://a.ppy.sh/', 'TR', 47000, 494, '{}'),
    (-261, 'Lahana',      'https://a.ppy.sh/', 'TR', 7700,  81,  '{}'),
    (-262, 'Hamsili',     'https://a.ppy.sh/', 'TR', 22800, 240, '{}'),
    (-263, 'Tost',        'https://a.ppy.sh/', 'TR', 15500, 163, '{}'),
    (-264, 'Sucuklu',     'https://a.ppy.sh/', 'TR', 17900, 188, '{}'),
    (-265, 'Ciborek',     'https://a.ppy.sh/', 'TR', 5000,  52,  '{}'),
    (-266, 'Haydari',     'https://a.ppy.sh/', 'TR', 39500, 415, '{}');

INSERT INTO discord_accounts (osu_user_id, discord_id, username)
SELECT id, 'dummy' || osu_id, lower(username)
FROM osu_users
WHERE osu_id BETWEEN -266 AND -251;

DO $$
DECLARE
    team text[];
    new_team uuid;
BEGIN
    FOREACH team SLICE 1 IN ARRAY ARRAY[
        ['Boğaz Esintisi',   'Midyeci',     'Balikekmek'],
        ['Kumru Kardeşler',  'Kumru',       'Boyoz'],
        ['Ege Sabahı',       'Zeytinyagli', 'Enginar'],
        ['Gece Kuşları',     'Salep',       'Boza'],
        ['Pazar Kahvaltısı', 'Simitci',     'Kasar'],
        ['Lahana Sarması',   'Lahana',      'Hamsili'],
        ['Ayvalık Tostu',    'Tost',        'Sucuklu'],
        ['Çiğ Börek',        'Ciborek',     'Haydari']
    ] LOOP
        INSERT INTO teams (name) VALUES (team[1]) RETURNING id INTO new_team;
        INSERT INTO team_members (user_id, team_id)
        SELECT id, new_team FROM osu_users WHERE osu_id < 0 AND username = ANY (team[2:3]);
    END LOOP;
END $$;

-- Every team but Kayıp Rank in a lobby: seed.sql's lobbies B-D filled up to seven, then
-- two more. Teams sign up a minute apart in the order listed, after the ones there.
DO $$
DECLARE
    lobby record;
    target uuid;
BEGIN
    FOR lobby IN
        SELECT * FROM (VALUES
            (112233442::bigint, NULL::interval, NULL::text,
             ARRAY['Patlıcan Kardeşler', 'Akşam Yemeği', 'Mangal Başı', 'Acılı Olsun',
                   'Tereyağlılar', 'Ekstra Acı']),
            (112233443, NULL, NULL,
             ARRAY['Çorba Kulübü', 'Kış Hazırlığı', 'Muhallebiciler', 'Aşure Günü', 'Şerbet Ustaları']),
            (112233444, NULL, NULL,
             ARRAY['Hamur Tatlıları', 'Ramazan Sofrası', 'Fırından Yeni Çıktı', 'İzmir Gevreği',
                   'Gaziantep Sabahı', 'Yufka Yürekliler']),
            (112233445, interval '-8 days -16 hours', 'DevReferee',
             ARRAY['Yol Hediyesi', 'Sonbahar', 'Kuru Meyve', 'Boğaz Esintisi', 'Kumru Kardeşler',
                   'Ege Sabahı', 'Pazar Kahvaltısı']),
            (112233446, interval '-8 days -14 hours', 'DevHakem',
             ARRAY['Gece Kuşları', 'Lahana Sarması', 'Ayvalık Tostu', 'Çiğ Börek'])
        ) AS v (mp_id, offset_by, referee, teams)
    LOOP
        IF lobby.offset_by IS NULL THEN
            SELECT match_id INTO target FROM match_rooms WHERE mp_id = lobby.mp_id;
        ELSE
            INSERT INTO matches (kind, starts_at, referee_id, concluded_at, lobby_index)
            VALUES (
                'qualifier',
                date_trunc('hour', now()) + lobby.offset_by,
                (SELECT id FROM osu_users WHERE osu_id < 0 AND username = lobby.referee),
                date_trunc('hour', now()) + lobby.offset_by + interval '1 hour',
                (SELECT COALESCE(max(lobby_index), 0) + 1 FROM matches)
            )
            RETURNING id INTO target;
            INSERT INTO match_rooms (mp_id, match_id) VALUES (lobby.mp_id, target);
        END IF;
        INSERT INTO qualifier_entries (team_id, match_id, joined_at)
        SELECT t.id, target,
               now() - interval '3 days' + interval '10 minutes' + v.n * interval '1 minute'
        FROM unnest(lobby.teams) WITH ORDINALITY AS v (name, n)
        JOIN teams t ON t.name = v.name;
    END LOOP;
END $$;

-- Gece Kuşları told the referee they couldn't make it.
UPDATE matches m
SET referee_notes = 'Gece Kuşları gelmedi; bütün mapleri 0 sayılıyor.'
FROM match_rooms r
WHERE r.match_id = m.id AND r.mp_id = 112233446;

-- Every lobby but A (which keeps its chat's results) saved afresh with all its teams:
-- every map of the qualifier pool once, made up like seed.sql's, Rank Uçtu without the
-- pool's last two maps and Gece Kuşları without any.
DELETE FROM match_map_results q
USING match_rooms r
WHERE r.match_id = q.match_id AND r.mp_id IN (112233442, 112233443, 112233444);

WITH lobbies AS (
    SELECT m.id AS match_id
    FROM matches m
    JOIN match_rooms r ON r.match_id = m.id
    WHERE m.kind = 'qualifier' AND r.mp_id BETWEEN 112233442 AND 112233446
),
pool AS (
    SELECT row_number() OVER (ORDER BY c.position, s.position)::int AS n, pm.id AS map_id,
           c.name AS category
    FROM stages st
    JOIN stage_categories c ON c.stage_id = st.id
    JOIN category_slots s ON s.category_id = c.id
    JOIN pool_maps pm ON pm.slot_id = s.id
    WHERE st.is_qualifier
),
results AS (
    INSERT INTO match_map_results (match_id, position, pool_map_id)
    SELECT l.match_id, p.n, p.map_id FROM lobbies l CROSS JOIN pool p
)
INSERT INTO match_map_scores (match_id, position, user_id, score, mods, passed)
SELECT l.match_id, p.n, u.id,
       pg_temp.seed_score(u.username, p.map_id::text, 'q' || p.n),
       CASE p.category WHEN 'HD' THEN 'HDNF' WHEN 'HR' THEN 'HRNF' WHEN 'DT' THEN 'DTNF' ELSE 'NF' END,
       true
FROM lobbies l
CROSS JOIN pool p
JOIN qualifier_entries e ON e.match_id = l.match_id
JOIN teams t ON t.id = e.team_id
JOIN team_members tm ON tm.team_id = t.id
JOIN osu_users u ON u.id = tm.user_id
WHERE t.name <> 'Gece Kuşları'
  AND NOT (t.name = 'Rank Uçtu' AND p.n > (SELECT max(n) FROM pool) - 2);

-- Their accuracy, as the bots would have read it from osu!'s record of the room.
UPDATE match_map_scores s
SET accuracy = round((0.86 + LEAST(s.score, 1000000) / 1000000.0 * 0.12
                      + abs(hashtext(u.username || '#' || s.position)) % 150 / 10000.0)::numeric, 4)::float8
FROM osu_users u
WHERE u.id = s.user_id AND u.osu_id < 0 AND s.accuracy IS NULL;

DROP FUNCTION pg_temp.seed_versus_score, pg_temp.seed_score, pg_temp.seed_normal, pg_temp.seed_uniform;
