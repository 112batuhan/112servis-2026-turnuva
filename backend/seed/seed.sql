-- Dummy data for local testing. Dummy accounts use negative osu_ids so they can
-- never collide with real ones; run clear.sql first to reset them.

INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, role) VALUES
    (-1,   'DevHost',    'https://a.ppy.sh/', 'TR', 52000, 900,  'host'),
    (-2,   'DevPooler',  'https://a.ppy.sh/', 'TR', 41000, 700,  'map_pooler'),
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
    (-124, 'Midye',      'https://a.ppy.sh/', 'TR', 64000, 560,  'basic');

-- Players left out here have no Discord, so they can't send or accept invites.
INSERT INTO discord_accounts (osu_user_id, discord_id, username)
SELECT id, 'dummy' || osu_id, lower(username)
FROM osu_users
WHERE osu_id IN (-1, -2, -101, -103, -104, -106, -107, -109, -110, -112, -113, -115, -116, -118, -122, -124);

-- All of these pairs are within the rank limit.
DO $$
DECLARE
    pair text[];
    new_team uuid;
BEGIN
    FOREACH pair SLICE 1 IN ARRAY ARRAY[
        ['Kedicik', 'Gozleme'],
        ['Simit', 'Tavuk'],
        ['Lokum', 'Kunefe'],
        ['Pide', 'Lahmacun'],
        ['Manti', 'Cay']
    ] LOOP
        INSERT INTO teams DEFAULT VALUES RETURNING id INTO new_team;
        INSERT INTO team_members (user_id, team_id)
        SELECT id, new_team FROM osu_users WHERE osu_id < 0 AND username = ANY (pair);
    END LOOP;
END $$;

-- Kumpir -> Baklava is just over the limit (19004), as if a rank changed after inviting.
INSERT INTO team_invites (inviter_id, invitee_id, created_at)
SELECT a.id, b.id, now() - v.age
FROM (VALUES
    ('Ayran',   'Sucuk',   interval '2 days'),
    ('Ayran',   'Helva',   interval '1 day'),
    ('Borek',   'Sucuk',   interval '5 hours'),
    ('Baklava', 'Doner',   interval '3 hours'),
    ('Nohut',   'Menemen', interval '40 minutes'),
    ('Leblebi', 'Midye',   interval '10 minutes'),
    ('Kumpir',  'Baklava', interval '1 hour')
) AS v (inviter, invitee, age)
JOIN osu_users a ON a.osu_id < 0 AND a.username = v.inviter
JOIN osu_users b ON b.osu_id < 0 AND b.username = v.invitee;
