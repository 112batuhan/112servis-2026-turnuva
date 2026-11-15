-- A small tournament still forming teams, run after clear.sql instead of seed.sql: the
-- dev staff accounts and seven teams, with no qualifier lobby, score, stage, pool or
-- bracket, so the team lists show no qualifier places. It keeps what the lists show
-- either way: banners on five teams (db/dev.rs adds them by name), Rank Uçtu over the
-- limit, Kayıp Rank with an unranked player, and two players without a team, one of
-- them invited.

INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, roles) VALUES
    (-1,   'DevHost',    'https://a.ppy.sh/', 'TR', 52000, 900,  '{host}'),
    (-2,   'DevPooler',  'https://a.ppy.sh/', 'TR', 41000, 700,  '{map_pooler}'),
    (-3,   'DevReferee', 'https://a.ppy.sh/', 'TR', 63000, 1100, '{referee}'),
    (-4,   'DevHakem',   'https://a.ppy.sh/', 'TR', 88000, 1500, '{referee,streamer}'),
    (-5,   'DevYayinci', 'https://a.ppy.sh/', 'TR', 99000, 1700, '{streamer}'),
    (-101, 'Kedicik',    'https://a.ppy.sh/', 'TR', 1850,  12,   '{}'),
    (-118, 'Gozleme',    'https://a.ppy.sh/', 'TR', 72000, 620,  '{}'),
    (-102, 'Simit',      'https://a.ppy.sh/', 'TR', 3200,  25,   '{}'),
    (-119, 'Tavuk',      'https://a.ppy.sh/', 'TR', 95000, 800,  '{}'),
    (-104, 'Lokum',      'https://a.ppy.sh/', 'TR', 6400,  60,   '{}'),
    (-113, 'Kunefe',     'https://a.ppy.sh/', 'TR', 28000, 262,  '{}'),
    (-106, 'Pide',       'https://a.ppy.sh/', 'TR', 9900,  95,   '{}'),
    (-111, 'Lahmacun',   'https://a.ppy.sh/', 'TR', 21000, 197,  '{}'),
    (-108, 'Manti',      'https://a.ppy.sh/', 'TR', 13200, 128,  '{}'),
    (-110, 'Cay',        'https://a.ppy.sh/', 'TR', 18400, 172,  '{streamer}'),
    (-125, 'Kaymak',     'https://a.ppy.sh/', 'TR', 2400,  19,   '{}'),
    (-126, 'Bal',        'https://a.ppy.sh/', 'TR', 11000, 105,  '{}'),
    (-121, 'Ayva',       'https://a.ppy.sh/', 'TR', NULL,  NULL, '{}'),
    (-117, 'Pismaniye',  'https://a.ppy.sh/', 'TR', 58000, 510,  '{}'),
    (-103, 'Ayran',      'https://a.ppy.sh/', 'TR', 4700,  41,   '{}'),
    (-115, 'Sucuk',      'https://a.ppy.sh/', 'TR', 39000, 355,  '{}');

INSERT INTO discord_accounts (osu_user_id, discord_id, username)
SELECT id, 'dummy' || osu_id, lower(username)
FROM osu_users
WHERE osu_id < 0 AND osu_id <> -121;

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

-- Ayran has invited Sucuk; neither is in a team yet.
INSERT INTO team_invites (inviter_id, invitee_id, team_name, created_at)
SELECT a.id, b.id, 'Ayran Gönüllüleri', now() - interval '1 hour'
FROM osu_users a, osu_users b
WHERE a.osu_id = -103 AND b.osu_id = -115;
