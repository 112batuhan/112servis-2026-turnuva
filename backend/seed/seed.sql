-- Dummy data for local testing. Dummy accounts use negative osu_ids so they can
-- never collide with real ones; run clear.sql first to reset them.

INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, role) VALUES
    (-1,   'DevHost',    'https://a.ppy.sh/', 'TR', 52000, 900,  'host'),
    (-2,   'DevPooler',  'https://a.ppy.sh/', 'TR', 41000, 700,  'map_pooler'),
    (-3,   'DevReferee', 'https://a.ppy.sh/', 'TR', 63000, 1100, 'referee'),
    (-4,   'DevHakem',   'https://a.ppy.sh/', 'TR', 88000, 1500, 'referee'),
    (-101, 'Kedicik',    'https://a.ppy.sh/', 'TR', 1850,  12,   'basic'),
    (-102, 'Simit',      'https://a.ppy.sh/', 'TR', 3200,  25,   'basic'),
    (-103, 'Ayran',      'https://a.ppy.sh/', 'TR', 4700,  41,   'basic'),
    (-104, 'Lokum',      'https://a.ppy.sh/', 'TR', 6400,  60,   'basic'),
    (-105, 'Baklava',    'https://a.ppy.sh/', 'TR', 8100,  78,   'basic'),
    (-106, 'Pide',       'https://a.ppy.sh/', 'TR', 9900,  95,   'basic'),
    (-107, 'Kumpir',     'https://a.ppy.sh/', 'TR', 11500, 110,  'basic'),
    (-108, 'Manti',      'https://a.ppy.sh/', 'TR', 13200, 128,  'basic'),
    (-109, 'Borek',      'https://a.ppy.sh/', 'TR', 15800, 150,  'basic'),
    (-110, 'Cay',        'https://a.ppy.sh/', 'TR', 18400, 172,  'basic'),
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
WHERE osu_id IN (-1, -2, -3, -4, -101, -103, -104, -106, -107, -109, -110, -112, -113, -115, -116, -118, -122, -124, -125, -126);

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
