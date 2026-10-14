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
    (-126, 'Bal',        'https://a.ppy.sh/', 'TR', 11000, 105,  'basic');

-- Players left out here have no Discord, so they can't send or accept invites.
INSERT INTO discord_accounts (osu_user_id, discord_id, username)
SELECT id, 'dummy' || osu_id, lower(username)
FROM osu_users
WHERE osu_id IN (-1, -2, -3, -4, -5, -101, -103, -104, -106, -107, -109, -110, -112, -113, -115, -116, -118, -122, -124, -125, -126);

-- The last two no longer fit, as if ranks changed after they formed: Kaymak
-- climbed (21,203 against the 19,000 limit) and Ayva went inactive.
DO $$
DECLARE
    team text[];
    new_team uuid;
BEGIN
    FOREACH team SLICE 1 IN ARRAY ARRAY[
        ['Sokak Kedileri', 'Kedicik', 'Gozleme'],
        ['Simitçiler',     'Simit',   'Tavuk'],
        ['Tatlı Krizi',    'Lokum',   'Kunefe'],
        ['Fırın Ekibi',    'Pide',    'Lahmacun'],
        ['Çay Ocağı',      'Manti',   'Cay'],
        ['Rank Uçtu',      'Kaymak',  'Bal'],
        ['Kayıp Rank',     'Ayva',    'Pismaniye']
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

-- A double-elimination bracket for four teams. W1 is concluded, so its winner and
-- loser have already moved on to W3 and L1. Every match has a dummy referee or dummy
-- teams, so clear.sql removes it. Cay streams but also plays, in Çay Ocağı.
-- Numbers continue after any bracket that's already there, and it's placed to its
-- right. Positions are in board units (20px); a card is 12x5.
DO $$
DECLARE
    x0 int := COALESCE((SELECT max(pos_x) + 30 FROM bracket_matches), 0);
    w0 int := COALESCE((SELECT max(number) FROM bracket_matches WHERE side = 'winners'), 0);
    l0 int := COALESCE((SELECT max(number) FROM bracket_matches WHERE side = 'losers'), 0);
    ids uuid[] := ARRAY[]::uuid[];
    spec record;
    new_match uuid;
BEGIN
    FOR spec IN
        SELECT * FROM (VALUES
            (1, 'winners'::bracket_side, 1, 0, 0, interval '-1 day', 'DevReferee', 'DevYayinci',
             'Sokak Kedileri', 'Simitçiler', 4, 2, 112233443::bigint, true),
            (2, 'winners', 2, 0, 6, interval '2 hours', 'DevHakem', NULL,
             'Tatlı Krizi', 'Fırın Ekibi', NULL, NULL, NULL, false),
            (3, 'winners', 3, 15, 3, interval '1 day', NULL, 'Cay',
             'Sokak Kedileri', NULL, NULL, NULL, NULL, false),
            (4, 'losers', 1, 15, 13, interval '1 day 2 hours', 'DevReferee', NULL,
             'Simitçiler', NULL, NULL, NULL, NULL, false),
            (5, 'losers', 2, 30, 13, interval '2 days', 'DevHakem', NULL,
             NULL, NULL, NULL, NULL, NULL, false),
            (6, 'winners', 4, 45, 8, NULL, 'DevHakem', NULL,
             NULL, NULL, NULL, NULL, NULL, false)
        ) AS v (key, side, number, x, y, offset_by, referee, streamer,
                team1, team2, score1, score2, mp_id, concluded)
    LOOP
        INSERT INTO matches (kind, starts_at, referee_id, mp_id, concluded_at)
        VALUES (
            'bracket',
            date_trunc('hour', now()) + spec.offset_by,
            (SELECT id FROM osu_users WHERE osu_id < 0 AND username = spec.referee),
            spec.mp_id,
            CASE WHEN spec.concluded THEN now() - interval '20 hours' END
        )
        RETURNING id INTO new_match;
        INSERT INTO bracket_matches (match_id, side, number, pos_x, pos_y,
                                     team1_id, team2_id, team1_score, team2_score, streamer_id)
        VALUES (
            new_match,
            spec.side,
            spec.number + CASE WHEN spec.side = 'winners' THEN w0 ELSE l0 END,
            x0 + spec.x,
            spec.y,
            (SELECT id FROM teams WHERE name = spec.team1),
            (SELECT id FROM teams WHERE name = spec.team2),
            spec.score1,
            spec.score2,
            (SELECT id FROM osu_users WHERE osu_id < 0 AND username = spec.streamer)
        );
        ids[spec.key] := new_match;
    END LOOP;

    INSERT INTO bracket_links (source_id, outcome, target_id, slot)
    SELECT ids[v.source], v.outcome::bracket_outcome, ids[v.target], v.slot
    FROM (VALUES
        (1, 'winner', 3, 1), (1, 'loser', 4, 1),
        (2, 'winner', 3, 2), (2, 'loser', 4, 2),
        (3, 'winner', 6, 1), (3, 'loser', 5, 1),
        (4, 'winner', 5, 2),
        (5, 'winner', 6, 2)
    ) AS v (source, outcome, target, slot);
END $$;
