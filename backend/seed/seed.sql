-- Dummy data for local testing. Dummy accounts use negative osu_ids so they can
-- never collide with real ones; run clear.sql first to reset them.

INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, role) VALUES
    (-1,   'DevHost',    'https://a.ppy.sh/', 'TR', 52000, 900,  'host'),
    (-2,   'DevPooler',  'https://a.ppy.sh/', 'TR', 41000, 700,  'map_pooler'),
    (-3,   'DevReferee', 'https://a.ppy.sh/', 'TR', 63000, 1100, 'referee'),
    (-4,   'DevHakem',   'https://a.ppy.sh/', 'TR', 88000, 1500, 'referee'),
    (-5,   'DevYayinci', 'https://a.ppy.sh/', 'TR', 99000, 1700, 'streamer'),
    (-101, 'Kedicik',    'https://a.ppy.sh/', 'TR', 1850,  12,   'basic'),
    (-102, 'Simit',      'https://a.ppy.sh/', 'TR', 3200,  25,   'basic'),
    (-103, 'Ayran',      'https://a.ppy.sh/', 'TR', 4700,  41,   'basic'),
    (-104, 'Lokum',      'https://a.ppy.sh/', 'TR', 6400,  60,   'basic'),
    (-105, 'Baklava',    'https://a.ppy.sh/', 'TR', 8100,  78,   'basic'),
    (-106, 'Pide',       'https://a.ppy.sh/', 'TR', 9900,  95,   'basic'),
    (-107, 'Kumpir',     'https://a.ppy.sh/', 'TR', 11500, 110,  'basic'),
    (-108, 'Manti',      'https://a.ppy.sh/', 'TR', 13200, 128,  'basic'),
    (-109, 'Borek',      'https://a.ppy.sh/', 'TR', 15800, 150,  'basic'),
    (-110, 'Cay',        'https://a.ppy.sh/', 'TR', 18400, 172,  'streamer'),
    (-111, 'Lahmacun',   'https://a.ppy.sh/', 'TR', 21000, 197,  'basic'),
    (-112, 'Doner',      'https://a.ppy.sh/', 'TR', 24500, 230,  'basic'),
    (-113, 'Kunefe',     'https://a.ppy.sh/', 'TR', 28000, 262,  'basic'),
    (-114, 'Menemen',    'https://a.ppy.sh/', 'TR', 33000, 305,  'basic'),
    (-115, 'Sucuk',      'https://a.ppy.sh/', 'TR', 39000, 355,  'basic'),
    (-116, 'Helva',      'https://a.ppy.sh/', 'TR', 47000, 420,  'basic'),
    (-117, 'Pismaniye',  'https://a.ppy.sh/', 'TR', 58000, 510,  'basic'),
    (-118, 'Gozleme',    'https://a.ppy.sh/', 'TR', 72000, 620,  'basic'),
    (-119, 'Tavuk',      'https://a.ppy.sh/', 'TR', 95000, 800,  'basic'),
    (-120, 'Kokorec',    'https://a.ppy.sh/', 'TR', NULL,  NULL, 'basic'),
    (-121, 'Ayva',       'https://a.ppy.sh/', 'TR', NULL,  NULL, 'basic'),
    (-122, 'Nohut',      'https://a.ppy.sh/', 'DE', 5600,  90,   'basic'),
    (-123, 'Leblebi',    'https://a.ppy.sh/', 'NL', 26000, 410,  'basic'),
    (-124, 'Midye',      'https://a.ppy.sh/', 'TR', 64000, 560,  'basic'),
    (-125, 'Kaymak',     'https://a.ppy.sh/', 'TR', 2400,  19,   'basic'),
    (-126, 'Bal',        'https://a.ppy.sh/', 'TR', 11000, 105,  'basic'),
    -- The players of the teams that fill the bracket.
    (-201, 'Zeytin',     'https://a.ppy.sh/', 'TR', 2100,  22,   'basic'),
    (-202, 'Peynir',     'https://a.ppy.sh/', 'TR', 44600, 469,  'basic'),
    (-203, 'Hamsi',      'https://a.ppy.sh/', 'TR', 2600,  27,   'basic'),
    (-204, 'Findik',     'https://a.ppy.sh/', 'TR', 53800, 566,  'basic'),
    (-205, 'Lakerda',    'https://a.ppy.sh/', 'TR', 3000,  31,   'basic'),
    (-206, 'Tursu',      'https://a.ppy.sh/', 'TR', 37000, 389,  'basic'),
    (-207, 'Pastirma',   'https://a.ppy.sh/', 'TR', 3400,  35,   'basic'),
    (-208, 'Kavurma',    'https://a.ppy.sh/', 'TR', 52800, 555,  'basic'),
    (-209, 'Sarma',      'https://a.ppy.sh/', 'TR', 3900,  41,   'basic'),
    (-210, 'Dolma',      'https://a.ppy.sh/', 'TR', 32500, 342,  'basic'),
    (-211, 'Imam',       'https://a.ppy.sh/', 'TR', 4300,  45,   'basic'),
    (-212, 'Karniyarik', 'https://a.ppy.sh/', 'TR', 24400, 256,  'basic'),
    (-213, 'Musakka',    'https://a.ppy.sh/', 'TR', 4800,  50,   'basic'),
    (-214, 'Kofte',      'https://a.ppy.sh/', 'TR', 34400, 362,  'basic'),
    (-215, 'Sis',        'https://a.ppy.sh/', 'TR', 5300,  55,   'basic'),
    (-216, 'Beyti',      'https://a.ppy.sh/', 'TR', 22600, 237,  'basic'),
    (-217, 'Adana',      'https://a.ppy.sh/', 'TR', 5900,  62,   'basic'),
    (-218, 'Urfa',       'https://a.ppy.sh/', 'TR', 37700, 396,  'basic'),
    (-219, 'Iskender',   'https://a.ppy.sh/', 'TR', 6500,  68,   'basic'),
    (-220, 'Tantuni',    'https://a.ppy.sh/', 'TR', 15000, 157,  'basic'),
    (-221, 'Cigkofte',   'https://a.ppy.sh/', 'TR', 7200,  75,   'basic'),
    (-222, 'Bazlama',    'https://a.ppy.sh/', 'TR', 19400, 204,  'basic'),
    (-223, 'Mercimek',   'https://a.ppy.sh/', 'TR', 8000,  84,   'basic'),
    (-224, 'Ezogelin',   'https://a.ppy.sh/', 'TR', 14000, 147,  'basic'),
    (-225, 'Tarhana',    'https://a.ppy.sh/', 'TR', 8800,  92,   'basic'),
    (-226, 'Yayla',      'https://a.ppy.sh/', 'TR', 20200, 212,  'basic'),
    (-227, 'Tavukgogsu', 'https://a.ppy.sh/', 'TR', 9700,  102,  'basic'),
    (-228, 'Kazandibi',  'https://a.ppy.sh/', 'TR', 13000, 136,  'basic'),
    (-229, 'Asure',      'https://a.ppy.sh/', 'TR', 10600, 111,  'basic'),
    (-230, 'Sutlac',     'https://a.ppy.sh/', 'TR', 10900, 114,  'basic'),
    (-231, 'Revani',     'https://a.ppy.sh/', 'TR', 11800, 124,  'basic'),
    (-232, 'Sekerpare',  'https://a.ppy.sh/', 'TR', 13400, 141,  'basic'),
    (-233, 'Tulumba',    'https://a.ppy.sh/', 'TR', 13000, 136,  'basic'),
    (-234, 'Lokma',      'https://a.ppy.sh/', 'TR', 13300, 140,  'basic'),
    (-235, 'Gullac',     'https://a.ppy.sh/', 'TR', 14500, 152,  'basic'),
    (-236, 'Kadayif',    'https://a.ppy.sh/', 'TR', 14800, 155,  'basic'),
    (-237, 'Pogaca',     'https://a.ppy.sh/', 'TR', 16000, 168,  'basic'),
    (-238, 'Acma',       'https://a.ppy.sh/', 'TR', 16300, 171,  'basic'),
    (-239, 'Gevrek',     'https://a.ppy.sh/', 'TR', 17500, 184,  'basic'),
    (-240, 'Halka',      'https://a.ppy.sh/', 'TR', 17800, 187,  'basic'),
    (-241, 'Katmer',     'https://a.ppy.sh/', 'TR', 19500, 205,  'basic'),
    (-242, 'Fistik',     'https://a.ppy.sh/', 'TR', 19800, 208,  'basic'),
    (-243, 'Yufka',      'https://a.ppy.sh/', 'TR', 22000, 231,  'basic'),
    (-244, 'Hosmerim',   'https://a.ppy.sh/', 'TR', 22300, 234,  'basic'),
    (-245, 'Pestil',     'https://a.ppy.sh/', 'TR', 25000, 263,  'basic'),
    (-246, 'Cezerye',    'https://a.ppy.sh/', 'TR', 25300, 266,  'basic'),
    (-247, 'Kestane',    'https://a.ppy.sh/', 'TR', 29000, 305,  'basic'),
    (-248, 'Dut',        'https://a.ppy.sh/', 'TR', 29300, 308,  'basic'),
    (-249, 'Kayisi',     'https://a.ppy.sh/', 'TR', 34000, 357,  'basic'),
    (-250, 'Incir',      'https://a.ppy.sh/', 'TR', 34300, 361,  'basic');

-- Players left out here have no Discord, so they can't send or accept invites.
INSERT INTO discord_accounts (osu_user_id, discord_id, username)
SELECT id, 'dummy' || osu_id, lower(username)
FROM osu_users
WHERE osu_id IN (-1, -2, -3, -4, -5, -101, -103, -104, -106, -107, -109, -110, -112, -113, -115, -116, -118, -122, -124, -125, -126)
   OR osu_id <= -201;

-- Rank Uçtu and Kayıp Rank no longer fit, as if ranks changed after they formed:
-- Kaymak climbed (21,203 against the 19,000 limit) and Ayva went inactive. The
-- teams after them only fill the 32-team bracket and aren't in qualifier lobbies.
DO $$
DECLARE
    team text[];
    new_team uuid;
BEGIN
    FOREACH team SLICE 1 IN ARRAY ARRAY[
        ['Sokak Kedileri',      'Kedicik',    'Gozleme'],
        ['Simitçiler',          'Simit',      'Tavuk'],
        ['Tatlı Krizi',         'Lokum',      'Kunefe'],
        ['Fırın Ekibi',         'Pide',       'Lahmacun'],
        ['Çay Ocağı',           'Manti',      'Cay'],
        ['Rank Uçtu',           'Kaymak',     'Bal'],
        ['Kayıp Rank',          'Ayva',       'Pismaniye'],
        ['Kahvaltı Sofrası',    'Zeytin',     'Peynir'],
        ['Karadeniz Fırtınası', 'Hamsi',      'Findik'],
        ['Meze Masası',         'Lakerda',    'Tursu'],
        ['Kayseri Ekspres',     'Pastirma',   'Kavurma'],
        ['Sarma Dolma',         'Sarma',      'Dolma'],
        ['Patlıcan Kardeşler',  'Imam',       'Karniyarik'],
        ['Akşam Yemeği',        'Musakka',    'Kofte'],
        ['Mangal Başı',         'Sis',        'Beyti'],
        ['Acılı Olsun',         'Adana',      'Urfa'],
        ['Tereyağlılar',        'Iskender',   'Tantuni'],
        ['Ekstra Acı',          'Cigkofte',   'Bazlama'],
        ['Çorba Kulübü',        'Mercimek',   'Ezogelin'],
        ['Kış Hazırlığı',       'Tarhana',    'Yayla'],
        ['Muhallebiciler',      'Tavukgogsu', 'Kazandibi'],
        ['Aşure Günü',          'Asure',      'Sutlac'],
        ['Şerbet Ustaları',     'Revani',     'Sekerpare'],
        ['Hamur Tatlıları',     'Tulumba',    'Lokma'],
        ['Ramazan Sofrası',     'Gullac',     'Kadayif'],
        ['Fırından Yeni Çıktı', 'Pogaca',     'Acma'],
        ['İzmir Gevreği',       'Gevrek',     'Halka'],
        ['Gaziantep Sabahı',    'Katmer',     'Fistik'],
        ['Yufka Yürekliler',    'Yufka',      'Hosmerim'],
        ['Yol Hediyesi',        'Pestil',     'Cezerye'],
        ['Sonbahar',            'Kestane',    'Dut'],
        ['Kuru Meyve',          'Kayisi',     'Incir']

    ] LOOP
        INSERT INTO teams (name) VALUES (team[1]) RETURNING id INTO new_team;
        INSERT INTO team_members (user_id, team_id)
        SELECT id, new_team FROM osu_users WHERE osu_id < 0 AND username = ANY (team[2:3]);
    END LOOP;
END $$;

-- Kumpir -> Baklava is just over the limit (19004), as if a rank changed after
-- inviting, and Borek's team name has been taken since.
INSERT INTO team_invites (inviter_id, invitee_id, team_name, created_at)
SELECT a.id, b.id, v.team_name, now() - v.age
FROM (VALUES
    ('Ayran',   'Sucuk',   'Ayran Gönüllüleri', interval '2 days'),
    ('Ayran',   'Helva',   'Ayran Gönüllüleri', interval '1 day'),
    ('Borek',   'Sucuk',   'Tatlı Krizi',       interval '5 hours'),
    ('Baklava', 'Doner',   'Şerbetliler',       interval '3 hours'),
    ('Nohut',   'Menemen', 'Kahvaltıcılar',     interval '40 minutes'),
    ('Leblebi', 'Midye',   'Sahil Yolu',        interval '10 minutes'),
    ('Kumpir',  'Baklava', 'Ortaköy',           interval '1 hour')
) AS v (inviter, invitee, team_name, age)
JOIN osu_users a ON a.osu_id < 0 AND a.username = v.inviter
JOIN osu_users b ON b.osu_id < 0 AND b.username = v.invitee;

-- Qualifier lobbies around now: one concluded, one being played, and upcoming ones
-- with and without a referee. Every lobby has a dummy referee or dummy teams, so
-- clear.sql removes it. 'Kayıp Rank' is left without a lobby.
DO $$
DECLARE
    lobby record;
    new_match uuid;
BEGIN
    FOR lobby IN
        SELECT * FROM (VALUES
            (interval '-2 days',  'DevReferee', 112233441::bigint, true,
             'Simitçiler 10 dakika geç geldi, ilk map yeniden oynandı.',
             ARRAY['Sokak Kedileri', 'Simitçiler']),
            (interval '-1 hour',  'DevReferee', 112233442, false, '', ARRAY['Tatlı Krizi']),
            (interval '1 day',    'DevHakem',   NULL, false, '', ARRAY['Fırın Ekibi', 'Çay Ocağı']),
            (interval '2 days',   NULL,         NULL, false, '', ARRAY['Rank Uçtu']),
            (interval '3 days',   'DevHakem',   NULL, false, '', ARRAY[]::text[])
        ) AS v (offset_by, referee, mp_id, concluded, notes, teams)
    LOOP
        INSERT INTO matches (kind, starts_at, referee_id, mp_id, referee_notes, concluded_at)
        VALUES (
            'qualifier',
            date_trunc('hour', now()) + lobby.offset_by,
            (SELECT id FROM osu_users WHERE osu_id < 0 AND username = lobby.referee),
            lobby.mp_id,
            lobby.notes,
            CASE WHEN lobby.concluded THEN now() - interval '1 day' END
        )
        RETURNING id INTO new_match;
        INSERT INTO qualifier_entries (team_id, match_id)
        SELECT t.id, new_match FROM teams t WHERE t.name = ANY (lobby.teams);
    END LOOP;
END $$;


-- Stages from the qualifiers to the grand final, the first three with their map pool
-- published, and Qualifiers as the qualifier lobbies' pool unless another stage
-- already is. Stages don't belong to a dummy account, so ones that already exist by
-- name are reused rather than duplicated, and clear.sql only removes them while they
-- have no categories. Keep the names in step with clear.sql.
INSERT INTO stages (name, position, published, is_qualifier)
SELECT v.name, COALESCE((SELECT max(position) FROM stages), -1) + v.ord, v.ord <= 3,
       v.name = 'Qualifiers' AND NOT EXISTS (SELECT 1 FROM stages WHERE is_qualifier)
FROM unnest(ARRAY[
    'Qualifiers', 'Round of 32', 'Round of 16', 'Quarterfinals', 'Semifinals',
    'Winners Final', 'Losers Final', 'Grand Final'
]) WITH ORDINALITY AS v (name, ord)
WHERE NOT EXISTS (SELECT 1 FROM stages s WHERE s.name = v.name);

-- A 32-team double-elimination bracket, partway through: winners round 1 and losers
-- round 1 are over, winners round 2 and losers round 2 are being played (one match
-- of each is live), the first winners round 3 match is done, and the rest is
-- upcoming, the last rounds still unscheduled. Teams are seeded by their
-- players' combined rank in the standard order (1v32, 16v17, ...). Losers rounds
-- alternate between pairing up the survivors and taking the teams that drop from
-- winners round r + 1, uncrossed. The grand final is the last winners match, with no
-- bracket reset.
--
-- Each round is played in the stage of its week: losers rounds share the stage of the
-- winners round they run alongside, and the last two losers rounds make up the losers
-- final.
-- Every match has a dummy referee or dummy teams, so clear.sql removes it. Numbers
-- continue after any bracket that's already there, and it's placed to its right.
-- Positions are in board units (20px); a card is 12x5, rounds are 15 apart and the
-- losers side sits under the winners side, one round to the right.
DO $$
DECLARE
    base timestamptz := date_trunc('hour', now());
    x0 int := COALESCE((SELECT max(pos_x) + 30 FROM bracket_matches), 0);
    w_num int := COALESCE((SELECT max(number) FROM bracket_matches WHERE side = 'winners'), 0);
    l_num int := COALESCE((SELECT max(number) FROM bracket_matches WHERE side = 'losers'), 0);
    seeds uuid[] := ARRAY(
        SELECT t.id
        FROM teams t
        JOIN team_members tm ON tm.team_id = t.id
        JOIN osu_users u ON u.id = tm.user_id
        GROUP BY t.id
        HAVING bool_and(u.osu_id < 0)
        ORDER BY sum(COALESCE(u.global_rank, 1000000)), t.name
    );
    -- The better seed of each winners round 1 match, top to bottom; it meets 33 - seed.
    tops int[] := ARRAY[1, 16, 9, 8, 5, 12, 13, 4, 3, 14, 11, 6, 7, 10, 15, 2];
    -- Match ids by round and index: 'w1-0', 'l3-2', 'gf-0'.
    ids jsonb := '{}';
    rnd record;
    idx int;
    num int;
    starts timestamptz;
    new_match uuid;
    played record;
    upset boolean;
    winner uuid;
    loser uuid;
BEGIN
    IF cardinality(seeds) <> 32 THEN
        RAISE EXCEPTION 'the bracket needs 32 dummy teams, found %', cardinality(seeds);
    END IF;

    -- Each round's matches start `step` apart from `starts` (null: unscheduled). The
    -- first `done` are concluded, and with `live` the next one has a room but no result.
    FOR rnd IN
        SELECT * FROM (VALUES
            ('w1', 'winners'::bracket_side, 16, 0,   0,   6,  interval '-3 days',  interval '1 hour',  16, false, 'Round of 32'),
            ('w2', 'winners', 8,  15,  3,   12, interval '-7 hours',   interval '1 hour',  6,  true,  'Round of 16'),
            ('w3', 'winners', 4,  30,  9,   24, interval '-2 hours',   interval '3 hours', 1,  false, 'Quarterfinals'),
            ('w4', 'winners', 2,  45,  21,  48, interval '2 days',     interval '2 hours', 0,  false, 'Semifinals'),
            ('w5', 'winners', 1,  60,  45,  0,  interval '3 days 4 hours', interval '0', 0, false, 'Winners Final'),
            ('gf', 'winners', 1,  135, 84,  0,  NULL,                  NULL,               0,  false, 'Grand Final'),
            ('l1', 'losers',  8,  15,  102, 6,  interval '-2 days',    interval '1 hour',  8,  false, 'Round of 16'),
            ('l2', 'losers',  8,  30,  102, 6,  interval '-4 hours',   interval '1 hour',  3,  true,  'Quarterfinals'),
            ('l3', 'losers',  4,  45,  105, 12, interval '1 day',      interval '1 hour',  0,  false, 'Quarterfinals'),
            ('l4', 'losers',  4,  60,  105, 12, interval '1 day 6 hours', interval '1 hour', 0, false, 'Semifinals'),
            ('l5', 'losers',  2,  75,  111, 24, interval '2 days 6 hours', interval '1 hour', 0, false, 'Semifinals'),
            ('l6', 'losers',  2,  90,  111, 24, interval '3 days',     interval '1 hour',  0,  false, 'Winners Final'),
            ('l7', 'losers',  1,  105, 123, 0,  NULL,                  NULL,               0,  false, 'Losers Final'),
            ('l8', 'losers',  1,  120, 123, 0,  NULL,                  NULL,               0,  false, 'Losers Final')
        ) AS v (key, side, matches, x, y, pitch, starts, step, done, live, stage)
    LOOP
        FOR idx IN 0 .. rnd.matches - 1 LOOP
            IF rnd.side = 'winners' THEN
                w_num := w_num + 1;
                num := w_num;
            ELSE
                l_num := l_num + 1;
                num := l_num;
            END IF;
            starts := base + rnd.starts + idx * rnd.step;
            INSERT INTO matches (kind, starts_at, referee_id, mp_id, concluded_at)
            VALUES (
                'bracket',
                starts,
                (SELECT id FROM osu_users
                 WHERE osu_id = CASE WHEN num % 2 = 0 THEN -3 ELSE -4 END),
                CASE WHEN idx < rnd.done OR (idx = rnd.done AND rnd.live)
                    THEN CASE WHEN rnd.side = 'winners' THEN 112234000 ELSE 112235000 END + num
                END,
                CASE WHEN idx < rnd.done THEN starts + interval '1 hour' END
            )
            RETURNING id INTO new_match;
            INSERT INTO bracket_matches (match_id, side, number, pos_x, pos_y, team1_id, team2_id,
                                         stage_id)
            VALUES (
                new_match,
                rnd.side,
                num,
                x0 + rnd.x,
                rnd.y + idx * rnd.pitch,
                CASE WHEN rnd.key = 'w1' THEN seeds[tops[idx + 1]] END,
                CASE WHEN rnd.key = 'w1' THEN seeds[33 - tops[idx + 1]] END,
                (SELECT id FROM stages WHERE name = rnd.stage ORDER BY position LIMIT 1)
            );
            ids := ids || jsonb_build_object(rnd.key || '-' || idx, new_match);
        END LOOP;
    END LOOP;

    INSERT INTO bracket_links (source_id, outcome, target_id, slot)
    SELECT (ids ->> v.source)::uuid, v.outcome::bracket_outcome, (ids ->> v.target)::uuid, v.slot
    FROM (
        -- Winners advance in pairs, and the winners final's winner to the grand final.
        SELECT 'w' || r || '-' || i, 'winner', 'w' || r + 1 || '-' || i / 2, i % 2 + 1
        FROM generate_series(1, 4) r, generate_series(0, (16 >> (r - 1)) - 1) i
        UNION ALL
        SELECT 'w5-0', 'winner', 'gf-0', 1
        -- Winners round 1's losers pair up in losers round 1.
        UNION ALL
        SELECT 'w1-' || i, 'loser', 'l1-' || i / 2, i % 2 + 1
        FROM generate_series(0, 15) i
        -- Later winners rounds' losers drop into the even losers rounds.
        UNION ALL
        SELECT 'w' || r || '-' || i, 'loser', 'l' || 2 * (r - 1) || '-' || i, 2
        FROM generate_series(2, 5) r, generate_series(0, (16 >> (r - 1)) - 1) i
        -- Odd losers rounds' winners meet a dropped team...
        UNION ALL
        SELECT 'l' || k || '-' || i, 'winner', 'l' || k + 1 || '-' || i, 1
        FROM generate_series(1, 7, 2) k, generate_series(0, (8 >> ((k - 1) / 2)) - 1) i
        -- ...and even ones' winners pair up.
        UNION ALL
        SELECT 'l' || k || '-' || i, 'winner', 'l' || k + 1 || '-' || i / 2, i % 2 + 1
        FROM generate_series(2, 6, 2) k, generate_series(0, (8 >> ((k - 1) / 2)) - 1) i
        UNION ALL
        SELECT 'l8-0', 'winner', 'gf-0', 2
    ) AS v (source, outcome, target, slot);

    -- Play the concluded matches in order, moving their teams on like concluding does.
    -- The better seed (team 1) wins a first-to-5 unless the number makes it an upset.
    FOR played IN
        SELECT b.match_id, b.number
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        WHERE b.match_id IN (SELECT value::uuid FROM jsonb_each_text(ids))
          AND m.concluded_at IS NOT NULL
        ORDER BY m.starts_at
    LOOP
        upset := played.number % 5 = 0;
        UPDATE bracket_matches
        SET team1_score = CASE WHEN upset THEN played.number * 3 % 5 ELSE 5 END,
            team2_score = CASE WHEN upset THEN 5 ELSE played.number * 3 % 5 END
        WHERE match_id = played.match_id
        RETURNING CASE WHEN upset THEN team2_id ELSE team1_id END,
                  CASE WHEN upset THEN team1_id ELSE team2_id END
        INTO winner, loser;
        UPDATE bracket_matches b
        SET team1_id = CASE WHEN k.slot = 1 THEN k.team ELSE b.team1_id END,
            team2_id = CASE WHEN k.slot = 2 THEN k.team ELSE b.team2_id END
        FROM (
            SELECT target_id, slot, CASE WHEN outcome = 'winner' THEN winner ELSE loser END
            FROM bracket_links
            WHERE source_id = played.match_id
        ) AS k (target_id, slot, team)
        WHERE b.match_id = k.target_id;
    END LOOP;

    -- Some upcoming matches with a team still need a referee.
    UPDATE matches m
    SET referee_id = NULL
    FROM bracket_matches b
    WHERE b.match_id = m.id
      AND m.id IN (SELECT value::uuid FROM jsonb_each_text(ids))
      AND m.mp_id IS NULL
      AND b.number % 3 = 0
      AND (b.team1_id IS NOT NULL OR b.team2_id IS NOT NULL);

    -- Streamers on some matches, but never one their own team plays in. Cay plays in
    -- Çay Ocağı.
    UPDATE bracket_matches b
    SET streamer_id = s.id
    FROM osu_users s
    WHERE b.match_id IN (SELECT value::uuid FROM jsonb_each_text(ids))
      AND b.number % 4 = 1
      AND s.osu_id = CASE WHEN b.number % 8 = 1 THEN -5 ELSE -110 END
      AND NOT EXISTS (
          SELECT 1 FROM team_members tm
          WHERE tm.user_id = s.id AND tm.team_id IN (b.team1_id, b.team2_id)
      );
END $$;

-- A room chat for W1, so the referee page has a concluded match with a log to show.
-- Team 1 plays red and takes five maps to blue's three, like the seeded 5-3.
WITH w1 AS (
    SELECT m.mp_id, m.starts_at AS t0, b.team1_id, b.team2_id, r.username AS referee
    FROM bracket_matches b
    JOIN matches m ON m.id = b.match_id
    LEFT JOIN osu_users r ON r.id = m.referee_id
    WHERE b.side = 'winners' AND b.number = 1 AND m.mp_id IS NOT NULL
),
players AS (
    SELECT u.username, tm.team_id = w1.team1_id AS red,
           row_number() OVER (ORDER BY tm.team_id = w1.team1_id DESC, u.username) AS slot
    FROM w1
    JOIN team_members tm ON tm.team_id IN (w1.team1_id, w1.team2_id)
    JOIN osu_users u ON u.id = tm.user_id
),
lines (at, sender, text) AS (
    SELECT interval '20 seconds' * p.slot, 'BanchoBot',
           p.username || ' joined in slot ' || p.slot || ' for team '
               || CASE WHEN p.red THEN 'red' ELSE 'blue' END || '.'
    FROM players p
    UNION ALL
    SELECT interval '2 minutes', COALESCE(w1.referee, 'BanchoBot'), 'glhf! İlk pick kırmızı takımda.'
    FROM w1
    UNION ALL
    SELECT interval '3 minutes' + (k - 1) * interval '6 minutes', 'BanchoBot',
           'Changed beatmap to https://osu.ppy.sh/b/' || (4000000 + k)
               || ' Seed Artist - Map ' || k || ' [Tournament]'
    FROM generate_series(1, 8) k
    UNION ALL
    SELECT interval '3 minutes 30 seconds' + (k - 1) * interval '6 minutes', 'BanchoBot',
           'The match has started!'
    FROM generate_series(1, 8) k
    UNION ALL
    SELECT interval '7 minutes' + (k - 1) * interval '6 minutes' + p.slot * interval '1 second',
           'BanchoBot',
           p.username || ' finished playing (Score: '
               || CASE WHEN p.red = (k NOT IN (2, 4, 6)) THEN 700000 ELSE 450000 END
                  + p.slot * 13579 + k * 2468
               || ', PASSED).'
    FROM generate_series(1, 8) k, players p
    UNION ALL
    SELECT interval '7 minutes 10 seconds' + (k - 1) * interval '6 minutes', 'BanchoBot',
           'The match has finished!'
    FROM generate_series(1, 8) k
    UNION ALL
    SELECT interval '52 minutes' + p.slot * interval '2 seconds', p.username, 'gg'
    FROM players p
)
INSERT INTO room_chat_messages (mp_id, sent_at, sender, text)
SELECT w1.mp_id, w1.t0 + l.at, l.sender, l.text
FROM w1, lines l
ORDER BY l.at;
