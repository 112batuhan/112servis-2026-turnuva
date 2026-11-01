-- Dummy data for local testing. Dummy accounts use negative osu_ids so they can
-- never collide with real ones; run clear.sql first to reset them.

INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, roles) VALUES
    (-1,   'DevHost',    'https://a.ppy.sh/', 'TR', 52000, 900,  '{host}'),
    (-2,   'DevPooler',  'https://a.ppy.sh/', 'TR', 41000, 700,  '{map_pooler}'),
    (-3,   'DevReferee', 'https://a.ppy.sh/', 'TR', 63000, 1100, '{referee}'),
    (-4,   'DevHakem',   'https://a.ppy.sh/', 'TR', 88000, 1500, '{referee,streamer}'),
    (-5,   'DevYayinci', 'https://a.ppy.sh/', 'TR', 99000, 1700, '{streamer}'),
    (-6,   'DevTester',  'https://a.ppy.sh/', 'TR', 71000, 1300, '{pool_tester}'),
    (-7,   'DevMapper',  'https://a.ppy.sh/', 'TR', 77000, 1400, '{custom_mapper}'),
    (-8,   'DevTasarim', 'https://a.ppy.sh/', 'TR', 82000, 1450, '{graphics_designer}'),
    (-9,   'DevYorumcu', 'https://a.ppy.sh/', 'TR', 86000, 1480, '{commentator}'),
    (-101, 'Kedicik',    'https://a.ppy.sh/', 'TR', 1850,  12,   '{}'),
    (-102, 'Simit',      'https://a.ppy.sh/', 'TR', 3200,  25,   '{}'),
    (-103, 'Ayran',      'https://a.ppy.sh/', 'TR', 4700,  41,   '{}'),
    (-104, 'Lokum',      'https://a.ppy.sh/', 'TR', 6400,  60,   '{}'),
    (-105, 'Baklava',    'https://a.ppy.sh/', 'TR', 8100,  78,   '{}'),
    (-106, 'Pide',       'https://a.ppy.sh/', 'TR', 9900,  95,   '{}'),
    (-107, 'Kumpir',     'https://a.ppy.sh/', 'TR', 11500, 110,  '{}'),
    (-108, 'Manti',      'https://a.ppy.sh/', 'TR', 13200, 128,  '{}'),
    (-109, 'Borek',      'https://a.ppy.sh/', 'TR', 15800, 150,  '{}'),
    (-110, 'Cay',        'https://a.ppy.sh/', 'TR', 18400, 172,  '{streamer}'),
    (-111, 'Lahmacun',   'https://a.ppy.sh/', 'TR', 21000, 197,  '{}'),
    (-112, 'Doner',      'https://a.ppy.sh/', 'TR', 24500, 230,  '{}'),
    (-113, 'Kunefe',     'https://a.ppy.sh/', 'TR', 28000, 262,  '{}'),
    (-114, 'Menemen',    'https://a.ppy.sh/', 'TR', 33000, 305,  '{}'),
    (-115, 'Sucuk',      'https://a.ppy.sh/', 'TR', 39000, 355,  '{}'),
    (-116, 'Helva',      'https://a.ppy.sh/', 'TR', 47000, 420,  '{}'),
    (-117, 'Pismaniye',  'https://a.ppy.sh/', 'TR', 58000, 510,  '{}'),
    (-118, 'Gozleme',    'https://a.ppy.sh/', 'TR', 72000, 620,  '{}'),
    (-119, 'Tavuk',      'https://a.ppy.sh/', 'TR', 95000, 800,  '{}'),
    (-120, 'Kokorec',    'https://a.ppy.sh/', 'TR', NULL,  NULL, '{}'),
    (-121, 'Ayva',       'https://a.ppy.sh/', 'TR', NULL,  NULL, '{}'),
    (-122, 'Nohut',      'https://a.ppy.sh/', 'DE', 5600,  90,   '{}'),
    (-123, 'Leblebi',    'https://a.ppy.sh/', 'NL', 26000, 410,  '{}'),
    (-124, 'Midye',      'https://a.ppy.sh/', 'TR', 64000, 560,  '{}'),
    (-125, 'Kaymak',     'https://a.ppy.sh/', 'TR', 2400,  19,   '{}'),
    (-126, 'Bal',        'https://a.ppy.sh/', 'TR', 11000, 105,  '{}'),
    -- The players of the teams that fill the bracket.
    (-201, 'Zeytin',     'https://a.ppy.sh/', 'TR', 2100,  22,   '{}'),
    (-202, 'Peynir',     'https://a.ppy.sh/', 'TR', 44600, 469,  '{}'),
    (-203, 'Hamsi',      'https://a.ppy.sh/', 'TR', 2600,  27,   '{}'),
    (-204, 'Findik',     'https://a.ppy.sh/', 'TR', 53800, 566,  '{}'),
    (-205, 'Lakerda',    'https://a.ppy.sh/', 'TR', 3000,  31,   '{}'),
    (-206, 'Tursu',      'https://a.ppy.sh/', 'TR', 37000, 389,  '{}'),
    (-207, 'Pastirma',   'https://a.ppy.sh/', 'TR', 3400,  35,   '{}'),
    (-208, 'Kavurma',    'https://a.ppy.sh/', 'TR', 52800, 555,  '{}'),
    (-209, 'Sarma',      'https://a.ppy.sh/', 'TR', 3900,  41,   '{}'),
    (-210, 'Dolma',      'https://a.ppy.sh/', 'TR', 32500, 342,  '{}'),
    (-211, 'Imam',       'https://a.ppy.sh/', 'TR', 4300,  45,   '{}'),
    (-212, 'Karniyarik', 'https://a.ppy.sh/', 'TR', 24400, 256,  '{}'),
    (-213, 'Musakka',    'https://a.ppy.sh/', 'TR', 4800,  50,   '{}'),
    (-214, 'Kofte',      'https://a.ppy.sh/', 'TR', 34400, 362,  '{}'),
    (-215, 'Sis',        'https://a.ppy.sh/', 'TR', 5300,  55,   '{}'),
    (-216, 'Beyti',      'https://a.ppy.sh/', 'TR', 22600, 237,  '{}'),
    (-217, 'Adana',      'https://a.ppy.sh/', 'TR', 5900,  62,   '{}'),
    (-218, 'Urfa',       'https://a.ppy.sh/', 'TR', 37700, 396,  '{}'),
    (-219, 'Iskender',   'https://a.ppy.sh/', 'TR', 6500,  68,   '{}'),
    (-220, 'Tantuni',    'https://a.ppy.sh/', 'TR', 15000, 157,  '{}'),
    (-221, 'Cigkofte',   'https://a.ppy.sh/', 'TR', 7200,  75,   '{}'),
    (-222, 'Bazlama',    'https://a.ppy.sh/', 'TR', 19400, 204,  '{}'),
    (-223, 'Mercimek',   'https://a.ppy.sh/', 'TR', 8000,  84,   '{}'),
    (-224, 'Ezogelin',   'https://a.ppy.sh/', 'TR', 14000, 147,  '{}'),
    (-225, 'Tarhana',    'https://a.ppy.sh/', 'TR', 8800,  92,   '{}'),
    (-226, 'Yayla',      'https://a.ppy.sh/', 'TR', 20200, 212,  '{}'),
    (-227, 'Tavukgogsu', 'https://a.ppy.sh/', 'TR', 9700,  102,  '{}'),
    (-228, 'Kazandibi',  'https://a.ppy.sh/', 'TR', 13000, 136,  '{}'),
    (-229, 'Asure',      'https://a.ppy.sh/', 'TR', 10600, 111,  '{}'),
    (-230, 'Sutlac',     'https://a.ppy.sh/', 'TR', 10900, 114,  '{}'),
    (-231, 'Revani',     'https://a.ppy.sh/', 'TR', 11800, 124,  '{}'),
    (-232, 'Sekerpare',  'https://a.ppy.sh/', 'TR', 13400, 141,  '{}'),
    (-233, 'Tulumba',    'https://a.ppy.sh/', 'TR', 13000, 136,  '{}'),
    (-234, 'Lokma',      'https://a.ppy.sh/', 'TR', 13300, 140,  '{}'),
    (-235, 'Gullac',     'https://a.ppy.sh/', 'TR', 14500, 152,  '{}'),
    (-236, 'Kadayif',    'https://a.ppy.sh/', 'TR', 14800, 155,  '{}'),
    (-237, 'Pogaca',     'https://a.ppy.sh/', 'TR', 16000, 168,  '{}'),
    (-238, 'Acma',       'https://a.ppy.sh/', 'TR', 16300, 171,  '{}'),
    (-239, 'Gevrek',     'https://a.ppy.sh/', 'TR', 17500, 184,  '{}'),
    (-240, 'Halka',      'https://a.ppy.sh/', 'TR', 17800, 187,  '{}'),
    (-241, 'Katmer',     'https://a.ppy.sh/', 'TR', 19500, 205,  '{}'),
    (-242, 'Fistik',     'https://a.ppy.sh/', 'TR', 19800, 208,  '{}'),
    (-243, 'Yufka',      'https://a.ppy.sh/', 'TR', 22000, 231,  '{}'),
    (-244, 'Hosmerim',   'https://a.ppy.sh/', 'TR', 22300, 234,  '{}'),
    (-245, 'Pestil',     'https://a.ppy.sh/', 'TR', 25000, 263,  '{}'),
    (-246, 'Cezerye',    'https://a.ppy.sh/', 'TR', 25300, 266,  '{}'),
    (-247, 'Kestane',    'https://a.ppy.sh/', 'TR', 29000, 305,  '{}'),
    (-248, 'Dut',        'https://a.ppy.sh/', 'TR', 29300, 308,  '{}'),
    (-249, 'Kayisi',     'https://a.ppy.sh/', 'TR', 34000, 357,  '{}'),
    (-250, 'Incir',      'https://a.ppy.sh/', 'TR', 34300, 361,  '{}');

-- Players left out here have no Discord, so they can't send or accept invites.
INSERT INTO discord_accounts (osu_user_id, discord_id, username)
SELECT id, 'dummy' || osu_id, lower(username)
FROM osu_users
WHERE osu_id IN (-1, -2, -3, -4, -5, -101, -103, -104, -106, -107, -109, -110, -112, -113, -115, -116, -118, -122, -124, -125, -126)
   OR osu_id <= -201;

-- Rank Uçtu and Kayıp Rank no longer fit, as if ranks changed after they formed:
-- Kaymak climbed (21,203 against the 19,000 limit) and Ayva went inactive. Of the
-- teams after them, the next five play in the concluded qualifier lobby; the rest only
-- fill the 32-team bracket.
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

-- Qualifier lobbies, all played the week before the bracket so only one stage has
-- matches going on (the pick'em of the first bracket stage waited for them): the first
-- full, seven teams, its chat further down, the rest smaller. Every lobby has a dummy
-- referee or dummy teams, so clear.sql removes it. 'Kayıp Rank' is left without a
-- lobby. Teams sign up a minute apart in the order listed. Each lobby takes the letter
-- after the last one, after any real lobbies.
DO $$
DECLARE
    lobby record;
    new_match uuid;
BEGIN
    FOR lobby IN
        SELECT * FROM (VALUES
            (interval '-9 days',           'DevReferee', 112233441::bigint,
             'Simitçiler 10 dakika geç geldi, ilk map yeniden oynandı.',
             ARRAY['Sokak Kedileri', 'Simitçiler', 'Kahvaltı Sofrası', 'Karadeniz Fırtınası',
                   'Meze Masası', 'Kayseri Ekspres', 'Sarma Dolma']),
            (interval '-8 days -22 hours', 'DevReferee', 112233442, '', ARRAY['Tatlı Krizi']),
            (interval '-8 days -20 hours', 'DevHakem',   112233443, '', ARRAY['Fırın Ekibi', 'Çay Ocağı']),
            (interval '-8 days -18 hours', 'DevHakem',   112233444, '', ARRAY['Rank Uçtu'])
        ) AS v (offset_by, referee, mp_id, notes, teams)
    LOOP
        INSERT INTO matches (kind, starts_at, referee_id, referee_notes, concluded_at, lobby_index)
        VALUES (
            'qualifier',
            date_trunc('hour', now()) + lobby.offset_by,
            (SELECT id FROM osu_users WHERE osu_id < 0 AND username = lobby.referee),
            lobby.notes,
            date_trunc('hour', now()) + lobby.offset_by + interval '1 hour',
            (SELECT COALESCE(max(lobby_index), 0) + 1 FROM matches)
        )
        RETURNING id INTO new_match;
        INSERT INTO match_rooms (mp_id, match_id)
        SELECT lobby.mp_id, new_match WHERE lobby.mp_id IS NOT NULL;
        INSERT INTO qualifier_entries (team_id, match_id, joined_at)
        SELECT t.id, new_match, now() - interval '3 days' + v.n * interval '1 minute'
        FROM unnest(lobby.teams) WITH ORDINALITY AS v (name, n)
        JOIN teams t ON t.name = v.name;
    END LOOP;
END $$;


-- Stages from the qualifiers to the grand final, the first four with their map pool
-- published, and Qualifiers as the qualifier lobbies' pool unless another stage
-- already is. Stages don't belong to a dummy account, so ones that already exist by
-- name are reused rather than duplicated, and clear.sql only removes them once they
-- have no categories left (it takes the seeded pools out first). Keep the names in
-- step with clear.sql. The bracket stages are played to Bo9 (the seeded results are
-- all first to 5) up to the quarterfinals, Bo11 from the semifinals and Bo13 in the
-- grand final; a reused stage keeps the best of a host gave it.
INSERT INTO stages (name, position, published, is_qualifier, best_of)
SELECT v.name, COALESCE((SELECT max(position) FROM stages), -1) + v.ord, v.ord <= 4,
       v.name = 'Qualifiers' AND NOT EXISTS (SELECT 1 FROM stages WHERE is_qualifier),
       v.best_of
FROM unnest(
    ARRAY['Qualifiers', 'Round of 32', 'Round of 16', 'Quarterfinals', 'Semifinals',
          'Finals', 'Grand Final'],
    ARRAY[NULL, 9, 9, 9, 11, 11, 13]::int[]
) WITH ORDINALITY AS v (name, best_of, ord)
WHERE NOT EXISTS (SELECT 1 FROM stages s WHERE s.name = v.name);

UPDATE stages s
SET best_of = v.best_of
FROM (VALUES ('Round of 32', 9), ('Round of 16', 9), ('Quarterfinals', 9), ('Semifinals', 11),
             ('Finals', 11), ('Grand Final', 13)) AS v (name, best_of)
WHERE s.name = v.name AND s.best_of IS NULL AND NOT s.is_qualifier;

-- Their pick and ban order: each team picks a map before the bans, one ban each up to
-- the quarterfinals and two from the semifinals, then picks in turn. A reused stage
-- keeps an order a host gave it.
UPDATE stages s
SET draft_order = v.draft_order
FROM (VALUES ('Round of 32', 'p1-p2-b2-b1-p1-*'), ('Round of 16', 'p1-p2-b2-b1-p1-*'),
             ('Quarterfinals', 'p1-p2-b2-b1-p1-*'), ('Semifinals', 'p1-p2-b2-b1-b2-b1-p1-*'),
             ('Finals', 'p1-p2-b2-b1-b2-b1-p1-*'), ('Grand Final', 'p1-p2-b2-b1-b2-b1-p1-*'))
    AS v (name, draft_order)
WHERE s.name = v.name AND s.draft_order IS NULL AND NOT s.is_qualifier;

-- Map pools for the seeded stages, from real ranked maps: for each stage and mod, the
-- most played map whose star rating fits that stage's band (rising from 5.0* in the
-- qualifiers to 5.9* in the grand final, with HR, DT and TB picked lower or higher to
-- suit), never the same mapset twice. The stats are what the app works out when the
-- maps are added with their mods, and the FM maps carry their HR stats. A stage gets each category it doesn't have yet (by
-- name), after its own, so a pool someone built stays as it is. DevPooler adds the
-- maps, which is how clear.sql tells seeded categories apart.
WITH pool (stage, category, color, cat_pos, slot_pos, mods, beatmap_id, beatmapset_id, artist,
           title, version, creator, cover_url, star_rating, ar, od, cs, bpm, total_length,
           hr_star_rating, hr_cs, hr_ar, hr_od) AS (
    VALUES
    ('Qualifiers', 'NM', '#5b8def', 0, 0, '', 1093507, 513590, 'RADWIMPS', 'Zen Zen Zense (movie ver.)', 'Haruto''s Insane', 'Monstrata', 'https://assets.ppy.sh/beatmaps/513590/covers/cover.jpg?1650644032', 5.0227, 9, 8.2, 3.9, 190, 107, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'NM', '#5b8def', 0, 1, '', 1932649, 921889, 'Fujiwara Chika (CV: Kohara Konomi)', 'Chikatto Chika Chika (TV Size)', 'Hazu''s Expert', 'Lami', 'https://assets.ppy.sh/beatmaps/921889/covers/cover.jpg?1650675795', 5.0006, 9.2, 8.5, 4, 178, 92, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'NM', '#5b8def', 0, 2, '', 847314, 128931, 'Feint', 'Tower Of Heaven (You Are Slaves)', 'Extra', 'eLy', 'https://assets.ppy.sh/beatmaps/128931/covers/cover.jpg?1645785502', 5.1247, 9, 8, 4, 175, 165, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'NM', '#5b8def', 0, 3, '', 853926, 384772, 'Yuaru', 'Asu no Yozora Shoukaihan', 'Insane', 'Akitoshi', 'https://assets.ppy.sh/beatmaps/384772/covers/cover.jpg?1650716263', 5.0611, 9, 7, 4, 185, 173, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'HD', '#e8b84a', 1, 0, 'HD', 847387, 387700, 'toby fox', 'MEGALOVANIA', 'Insane', 'Kyshiro', 'https://assets.ppy.sh/beatmaps/387700/covers/cover.jpg?1650636202', 5.0932, 9, 8.2, 4, 240, 144, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'HD', '#e8b84a', 1, 1, 'HD', 814293, 371128, 'Station Earth', 'Cold Green Eyes ft. Roos Denayer', 'Divine', 'Bearizm', 'https://assets.ppy.sh/beatmaps/371128/covers/cover.jpg?1650634565', 5.1884, 9.2, 8.5, 4, 174, 132, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'HR', '#f3877e', 2, 0, 'HR', 1385399, 653534, 'Panda Eyes', 'ILY', 'Fanteer''s Insane', 'M a r v o l l o', 'https://assets.ppy.sh/beatmaps/653534/covers/cover.jpg?1650652798', 4.7469, 10, 10, 4.55, 110, 96, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'HR', '#f3877e', 2, 1, 'HR', 785982, 352570, 'beatMARIO', 'Night of Knights', 'N a s y a''s Insane', 'alacat', 'https://assets.ppy.sh/beatmaps/352570/covers/cover.jpg?1650634611', 4.8267, 10, 9.8, 3.9, 180, 170, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'DT', '#b48cf2', 3, 0, 'DT', 397535, 163112, 'Kuba Oms', 'My Love', 'Insane', 'W h i t e', 'https://assets.ppy.sh/beatmaps/163112/covers/cover.jpg?1631509131', 5.3953, 9.67, 9.11, 4, 192, 149, NULL, NULL, NULL, NULL),
    ('Qualifiers', 'DT', '#b48cf2', 3, 1, 'DT', 760034, 332532, 'Panda Eyes & Teminite', 'Highscore', 'LGV''s Insane', 'Fort', 'https://assets.ppy.sh/beatmaps/332532/covers/cover.jpg?1650631844', 5.3206, 9.67, 8.98, 4, 165, 160, NULL, NULL, NULL, NULL),
    ('Round of 32', 'NM', '#5b8def', 0, 0, '', 962979, 444335, 'HO-KAGO TEA TIME', 'Kira Kira Days', 'Milan-''s Extra!!', 'Kagetsu', 'https://assets.ppy.sh/beatmaps/444335/covers/cover.jpg?1650638834', 5.2838, 9.3, 8.6, 4.3, 200, 90, NULL, NULL, NULL, NULL),
    ('Round of 32', 'NM', '#5b8def', 0, 1, '', 1962182, 744772, 'toby fox', 'Last Goodbye', 'Extra', 'Fatfan Kolek', 'https://assets.ppy.sh/beatmaps/744772/covers/cover.jpg?1650676934', 5.1095, 9.2, 8.5, 4, 180, 111, NULL, NULL, NULL, NULL),
    ('Round of 32', 'NM', '#5b8def', 0, 2, '', 918723, 423527, 'dj TAKA', 'quaver', 'Haru''s Insane', 'Monstrata', 'https://assets.ppy.sh/beatmaps/423527/covers/cover.jpg?1650640646', 5.195, 9, 8, 3.6, 182, 204, NULL, NULL, NULL, NULL),
    ('Round of 32', 'NM', '#5b8def', 0, 3, '', 1962198, 931452, 'HAG', 'Colorful', 'Nevo''s Expert', 'Sotarks', 'https://assets.ppy.sh/beatmaps/931452/covers/cover.jpg?1650678433', 5.2407, 9.3, 9, 3.6, 192, 129, NULL, NULL, NULL, NULL),
    ('Round of 32', 'HD', '#e8b84a', 1, 0, 'HD', 996356, 326920, 'Seiryu', 'BLUE DRAGON', 'Fiery''s Insane', 'Blue Dragon', 'https://assets.ppy.sh/beatmaps/326920/covers/cover.jpg?1650644794', 5.2245, 9, 8, 4, 191, 173, NULL, NULL, NULL, NULL),
    ('Round of 32', 'HD', '#e8b84a', 1, 1, 'HD', 1969095, 942866, 'Getter Jaani', 'Rockefeller Street (Nightcore Mix)', 'ggf''s Expert', 'Sotarks', 'https://assets.ppy.sh/beatmaps/942866/covers/cover.jpg?1650678288', 5.2559, 9, 8.7, 3.8, 162.5, 100, NULL, NULL, NULL, NULL),
    ('Round of 32', 'HR', '#f3877e', 2, 0, 'HR', 637623, 219380, 'Konuko', 'Toumei Elegy', 'G''s Insane', 'Awaken', 'https://assets.ppy.sh/beatmaps/219380/covers/cover.jpg?1650632979', 5.1975, 10, 9.8, 5.2, 220, 200, NULL, NULL, NULL, NULL),
    ('Round of 32', 'HR', '#f3877e', 2, 1, 'HR', 128718, 40440, 'S3RL', 'Pika Girl', 'Insane', 'Takuya', 'https://assets.ppy.sh/beatmaps/40440/covers/cover.jpg?1645779887', 4.9499, 10, 9.8, 5.2, 175, 221, NULL, NULL, NULL, NULL),
    ('Round of 32', 'DT', '#b48cf2', 3, 0, 'DT', 351189, 140662, 'cYsmix feat. Emmy', 'Tear Rain', 'Insane', 'jonathanlfj', 'https://assets.ppy.sh/beatmaps/140662/covers/cover.jpg?1622070421', 5.4013, 9.67, 9.11, 4, 192, 160, NULL, NULL, NULL, NULL),
    ('Round of 32', 'DT', '#b48cf2', 3, 1, 'DT', 315552, 123593, 'Rostik', 'Liquid (Paul Rosenthal Remix)', 'Insane', 'Charles445', 'https://assets.ppy.sh/beatmaps/123593/covers/cover.jpg?1622068626', 5.5735, 9.67, 9.78, 4, 199.5, 107, NULL, NULL, NULL, NULL),
    ('Round of 32', 'DT', '#b48cf2', 3, 2, 'DT', 340104, 134151, 'Hanatan', 'Airman ga Taosenai (SOUND HOLIC Ver.)', 'Hard', 'Natsu', 'https://assets.ppy.sh/beatmaps/134151/covers/cover.jpg?1650621307', 5.3374, 9.67, 8.44, 4, 262.5, 151, NULL, NULL, NULL, NULL),
    ('Round of 32', 'FM', '#5fd38d', 4, 0, '', 992281, 462386, 'Foreground Eclipse', 'From Under Cover (Caught Up In A Love Song)', 'Lunatic', 'Seni', 'https://assets.ppy.sh/beatmaps/462386/covers/cover.jpg?1650643367', 5.1119, 9, 7.5, 4, 200, 203, 5.5052, 5.2, 10, 10),
    ('Round of 32', 'FM', '#5fd38d', 4, 1, '', 1531842, 721804, 'Omoi', 'Teo', 'Akitoshi vs. SnowNiNo''s Insane', 'Kroytz', 'https://assets.ppy.sh/beatmaps/721804/covers/cover.jpg?1650659527', 5.0582, 9, 7.5, 4, 185, 205, 5.2913, 5.2, 10, 10),
    ('Round of 32', 'TB', '#ff66ab', 5, 0, '', 719594, 292301, 'xi', 'Blue Zenith', 'RLC''s Insane', 'Asphyxia', 'https://assets.ppy.sh/beatmaps/292301/covers/cover.jpg?1650630372', 5.5873, 9, 8, 4, 200, 249, NULL, NULL, NULL, NULL),
    ('Round of 16', 'NM', '#5b8def', 0, 0, '', 557821, 241526, 'Soleily', 'Renatus', 'Insane', 'Multiple Creators', 'https://assets.ppy.sh/beatmaps/241526/covers/cover.jpg?1622080332', 5.316, 9, 8, 4, 182, 207, NULL, NULL, NULL, NULL),
    ('Round of 16', 'NM', '#5b8def', 0, 1, '', 374114, 151878, 'Chasers', 'Lost', 'Insane', 'ktgster', 'https://assets.ppy.sh/beatmaps/151878/covers/cover.jpg?1622071759', 5.244, 9, 8, 5, 170, 205, NULL, NULL, NULL, NULL),
    ('Round of 16', 'NM', '#5b8def', 0, 2, '', 709258, 213629, 'The Living Tombstone', 'Five Nights at Freddy''s', 'Extra', '-Faded-', 'https://assets.ppy.sh/beatmaps/213629/covers/cover.jpg?1650627414', 5.2435, 9.2, 8.2, 4, 103, 159, NULL, NULL, NULL, NULL),
    ('Round of 16', 'NM', '#5b8def', 0, 3, '', 2444149, 1171789, 'SPYAIR', 'Imagination (TV Size)', 'Extra', 'browiec', 'https://assets.ppy.sh/beatmaps/1171789/covers/cover.jpg?1622188996', 5.4189, 9.3, 9.2, 3.8, 204, 90, NULL, NULL, NULL, NULL),
    ('Round of 16', 'HD', '#e8b84a', 1, 0, 'HD', 1519137, 703957, 'Panda Eyes & Teminite', 'Immortal Flame (feat. Anna Yvette)', 'Mir''s Extra', 'Sotarks', 'https://assets.ppy.sh/beatmaps/703957/covers/cover.jpg?1650658558', 5.537, 9.3, 8.7, 3.5, 110, 214, NULL, NULL, NULL, NULL),
    ('Round of 16', 'HD', '#e8b84a', 1, 1, 'HD', 1027702, 480765, '3 Nen E Gumi Utatan (Nagisa & Kayano & Karuma & Isogai & Maehara)', 'Bye Bye YESTERDAY', 'Nhawak''s Extra', 'Gero', 'https://assets.ppy.sh/beatmaps/480765/covers/cover.jpg?1650646117', 5.5692, 9.2, 8, 4, 180, 92, NULL, NULL, NULL, NULL),
    ('Round of 16', 'HR', '#f3877e', 2, 0, 'HR', 1608726, 764602, 'Sex Whales & Fraxo', 'Dead To Me (Instrumental)', 'NiNo''s Insane', 'Sotarks', 'https://assets.ppy.sh/beatmaps/764602/covers/cover.jpg?1650662178', 5.3074, 10, 10, 5.2, 148, 93, NULL, NULL, NULL, NULL),
    ('Round of 16', 'HR', '#f3877e', 2, 1, 'HR', 136400, 43466, 'DJ Genericname', 'Dear You', 'Dear Rue', 'Rue', 'https://assets.ppy.sh/beatmaps/43466/covers/cover.jpg?1645780906', 5.1199, 10, 9.8, 5.2, 174.11, 111, NULL, NULL, NULL, NULL),
    ('Round of 16', 'DT', '#b48cf2', 3, 0, 'DT', 1593361, 757410, 'XXXTENTACION', 'SAD!', 'Depression', 'Chromoxx', 'https://assets.ppy.sh/beatmaps/757410/covers/cover.jpg?1645776895', 5.8557, 10, 9.78, 4, 112.5, 69, NULL, NULL, NULL, NULL),
    ('Round of 16', 'DT', '#b48cf2', 3, 1, 'DT', 1587909, 754219, 'TheFatRat ft. Laura Brehm', 'MAYDAY (Nightcore Mix)', 'Collab Insane', 'Peter', 'https://assets.ppy.sh/beatmaps/754219/covers/cover.jpg?1650661061', 5.8482, 10.33, 9.78, 4, 258.75, 67, NULL, NULL, NULL, NULL),
    ('Round of 16', 'DT', '#b48cf2', 3, 2, 'DT', 117427, 33651, 'Hatsune Miku', 'Rubik''s Cube', '5x5x5', 'rui', 'https://assets.ppy.sh/beatmaps/33651/covers/cover.jpg?1622048536', 5.4845, 9, 7.78, 4, 261, 113, NULL, NULL, NULL, NULL),
    ('Round of 16', 'FM', '#5fd38d', 4, 0, '', 780914, 354537, 'Feint', 'Vagrant (feat. Veela)', 'Still Alive', 'Aia', 'https://assets.ppy.sh/beatmaps/354537/covers/cover.jpg?1650633472', 5.2924, 9.2, 8.5, 4, 175, 112, 5.6411, 5.2, 10, 10),
    ('Round of 16', 'FM', '#5fd38d', 4, 1, '', 198171, 68500, 'LiSA', 'crossing field', 'Insane', 'Ryafuka', 'https://assets.ppy.sh/beatmaps/68500/covers/cover.jpg?1622060634', 5.2277, 9, 8, 4, 179, 237, 5.5808, 5.2, 10, 10),
    ('Round of 16', 'TB', '#ff66ab', 5, 0, '', 2535967, 1218852, 'YOASOBI', 'Yoru ni Kakeru', 'Collab Extra', 'CoLouRed GlaZeE', 'https://assets.ppy.sh/beatmaps/1218852/covers/cover.jpg?1645786453', 5.7745, 9.2, 8, 4, 130, 243, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'NM', '#5b8def', 0, 0, '', 1257298, 594429, 'Kenshi Yonezu', 'Peace Sign (TV edit.)', 'Plus Ultra', 'Finshie', 'https://assets.ppy.sh/beatmaps/594429/covers/cover.jpg?1645781351', 5.5602, 9.3, 8.5, 4, 200, 90, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'NM', '#5b8def', 0, 1, '', 164020, 53857, 'Saiya', 'Remote Control', 'Insane', 'Garven', 'https://assets.ppy.sh/beatmaps/53857/covers/cover.jpg?1650613297', 5.5319, 9, 9, 4, 165, 134, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'NM', '#5b8def', 0, 2, '', 836556, 366440, 'Reol', 'MONSTER', 'BYFAR''S EXTRA', 'handsome', 'https://assets.ppy.sh/beatmaps/366440/covers/cover.jpg?1622095148', 5.5177, 9, 8.5, 4, 164, 195, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'NM', '#5b8def', 0, 3, '', 857431, 357777, 'NOMA', 'Brain Power', 'NiNo''s EXHAUST', 'Jacob', 'https://assets.ppy.sh/beatmaps/357777/covers/cover.jpg?1650641012', 5.4671, 9, 8, 4, 170, 107, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'HD', '#e8b84a', 1, 0, 'HD', 476149, 153776, 'yuikonnu & ayaponzu*', 'Super Nuko World', 'Insane', 'AllStar12', 'https://assets.ppy.sh/beatmaps/153776/covers/cover.jpg?1650621428', 5.7529, 9, 7.5, 4, 190, 219, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'HD', '#e8b84a', 1, 1, 'HD', 1034482, 485056, '*namirin', 'Koi no Hime Hime Pettanko', 'Extra', '[C u r i]', 'https://assets.ppy.sh/beatmaps/485056/covers/cover.jpg?1650642198', 5.6865, 9.2, 8.5, 4, 178, 94, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'HR', '#f3877e', 2, 0, 'HR', 2237467, 1068768, 'yuikonnu', 'Yume Chizu', 'Expert', 'browiec', 'https://assets.ppy.sh/beatmaps/1068768/covers/cover.jpg?1645787898', 5.4533, 10, 10, 4.94, 176, 100, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'HR', '#f3877e', 2, 1, 'HR', 2129145, 1017271, 'Nashimoto Ui', 'AaAaAaAAaAaAAa', 'Reform''s ExPeRT', 'Sotarks', 'https://assets.ppy.sh/beatmaps/1017271/covers/cover.jpg?1650683889', 5.5796, 10, 10, 5.2, 192, 105, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'DT', '#b48cf2', 3, 0, 'DT', 803704, 361740, 'Agnete Kjolsrud', 'Get Jinxed', 'Platinum', 'Tarrasky', 'https://assets.ppy.sh/beatmaps/361740/covers/cover.jpg?1645788409', 6.2479, 10.33, 9.11, 4, 270, 87, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'DT', '#b48cf2', 3, 1, 'DT', 323875, 127712, 'Primastella', 'Koigokoro', 'Delis'' Insane', 'Luerxa', 'https://assets.ppy.sh/beatmaps/127712/covers/cover.jpg?1650619119', 5.7656, 9.67, 9.11, 4, 210, 105, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'DT', '#b48cf2', 3, 2, 'DT', 2969237, 1440767, 'DM DOKURO', 'SAVE (Encore)', 'browiec & Kuki''s Insane', 'browiec', 'https://assets.ppy.sh/beatmaps/1440767/covers/cover.jpg?1650709123', 5.857, 10.33, 9.78, 3.8, 270, 75, NULL, NULL, NULL, NULL),
    ('Quarterfinals', 'FM', '#5fd38d', 4, 0, '', 1843595, 881764, 'Will Stetson', 'Snow Halation (feat. BeasttrollMC)', 'Onlybiscuit''s Extra', 'Sotarks', 'https://assets.ppy.sh/beatmaps/881764/covers/cover.jpg?1650672557', 5.4408, 9, 9.2, 4, 173, 106, 5.8695, 5.2, 10, 10),
    ('Quarterfinals', 'FM', '#5fd38d', 4, 1, '', 2868389, 1388906, 'Raphlesia & BilliumMoto', 'My Love', 'Real & Panda''s Insane', 'Mao', 'https://assets.ppy.sh/beatmaps/1388906/covers/cover.jpg?1650707731', 5.3308, 9, 7.5, 3.8, 175, 223, 5.6377, 4.94, 10, 10),
    ('Quarterfinals', 'TB', '#ff66ab', 5, 0, '', 815857, 372510, 'THE ORAL CIGARETTES', 'Kyouran Hey Kids!!', 'God of Speed', 'monstrata', 'https://assets.ppy.sh/beatmaps/372510/covers/cover.jpg?1650634774', 5.8431, 9.3, 8, 4, 142, 246, NULL, NULL, NULL, NULL),
    ('Semifinals', 'NM', '#5b8def', 0, 0, '', 1828557, 874948, 'K/DA', 'POP/STARS (ft. Madison Beer, (G)I-DLE, Jaira Burns)', 'Sing''s Insane', 'Nathan', 'https://assets.ppy.sh/beatmaps/874948/covers/cover.jpg?1650671383', 5.5378, 9, 8, 4, 170, 187, NULL, NULL, NULL, NULL),
    ('Semifinals', 'NM', '#5b8def', 0, 1, '', 949011, 441155, 'Chino(CV.Minase Inori)', 'Shinsaku no Shiawase wa Kochira!', 'Dan Dan', 'Shioi', 'https://assets.ppy.sh/beatmaps/441155/covers/cover.jpg?1650638315', 5.6165, 9.2, 8.5, 4, 175, 124, NULL, NULL, NULL, NULL),
    ('Semifinals', 'NM', '#5b8def', 0, 2, '', 1576174, 745312, 'Ni-Sokkususu', 'Blade Dance', 'Kalibe''s Insane', 'Sotarks', 'https://assets.ppy.sh/beatmaps/745312/covers/cover.jpg?1650660483', 5.67, 9, 8, 4, 187, 145, NULL, NULL, NULL, NULL),
    ('Semifinals', 'NM', '#5b8def', 0, 3, '', 2397136, 1148073, 'ONE OK ROCK', 'One by One', 'Extra', 'Elinor', 'https://assets.ppy.sh/beatmaps/1148073/covers/cover.jpg?1645788262', 5.6456, 9.3, 9.1, 3.8, 197, 108, NULL, NULL, NULL, NULL),
    ('Semifinals', 'HD', '#e8b84a', 1, 0, 'HD', 1690359, 805224, 'Will Stetson', 'Super Driver', 'Rolniczy''s Extra', 'Sotarks', 'https://assets.ppy.sh/beatmaps/805224/covers/cover.jpg?1650665948', 5.6828, 9.3, 8.6, 3.6, 200, 96, NULL, NULL, NULL, NULL),
    ('Semifinals', 'HD', '#e8b84a', 1, 1, 'HD', 221950, 79116, 'Sagara Kokoro', 'Hoshizora no Ima', 'S.S', 'Star Stream', 'https://assets.ppy.sh/beatmaps/79116/covers/cover.jpg?1650615751', 5.7192, 9, 7, 4, 185, 100, NULL, NULL, NULL, NULL),
    ('Semifinals', 'HR', '#f3877e', 2, 0, 'HR', 1262433, 597111, 'RADWIMPS', 'Yume Tourou', 'Extra: Mitsuha', 'Monstrata', 'https://assets.ppy.sh/beatmaps/597111/covers/cover.jpg?1650649247', 5.5033, 10, 10, 5.2, 178, 126, NULL, NULL, NULL, NULL),
    ('Semifinals', 'HR', '#f3877e', 2, 1, 'HR', 1037478, 486535, 'cYsmix', 'Moonlight Sonata', 'Expert', 'Okorin', 'https://assets.ppy.sh/beatmaps/486535/covers/cover.jpg?1631509836', 5.6944, 10, 10, 5.2, 128, 190, NULL, NULL, NULL, NULL),
    ('Semifinals', 'DT', '#b48cf2', 3, 0, 'DT', 453358, 190390, 'Rameses B', 'Flaklypa', 'Insane', '-kevincela-', 'https://assets.ppy.sh/beatmaps/190390/covers/cover.jpg?1622075765', 6.2736, 10.33, 9.78, 4, 195, 121, NULL, NULL, NULL, NULL),
    ('Semifinals', 'DT', '#b48cf2', 3, 1, 'DT', 1627629, 769917, 'dark cat', 'BUBBLE TEA (feat. juu & cinders)', 'Kaifin''s Insane', 'Hobbes2', 'https://assets.ppy.sh/beatmaps/769917/covers/cover.jpg?1650663270', 6.1209, 9.67, 9.11, 3, 240, 97, NULL, NULL, NULL, NULL),
    ('Semifinals', 'DT', '#b48cf2', 3, 2, 'DT', 833638, 378559, 'NICO Touches the Walls', 'Niwaka Ame ni mo Makezu', 'Insane', 'monstrata', 'https://assets.ppy.sh/beatmaps/378559/covers/cover.jpg?1645787474', 5.9924, 10.13, 9.78, 4, 279, 78, NULL, NULL, NULL, NULL),
    ('Semifinals', 'FM', '#5fd38d', 4, 0, '', 2166311, 1035167, 'ONE OK ROCK', 'Start Again', 'Onlybiscuit''s Extra', 'A r M i N', 'https://assets.ppy.sh/beatmaps/1035167/covers/cover.jpg?1650685496', 5.4977, 9.2, 9, 3.8, 174, 122, 5.8947, 4.94, 10, 10),
    ('Semifinals', 'FM', '#5fd38d', 4, 1, '', 928868, 416153, 'Remo Prototype[CV: Hanamori Yumiri]', 'Sendan Life', 'Pho''s Insane', 'Lami', 'https://assets.ppy.sh/beatmaps/416153/covers/cover.jpg?1650639459', 5.4971, 9.3, 8, 4, 227, 232, 5.9835, 5.2, 10, 10),
    ('Semifinals', 'TB', '#ff66ab', 5, 0, '', 1088292, 478405, 'Omoi', 'Snow Drive(01.23)', 'Rumi''s Expert', 'Kroytz', 'https://assets.ppy.sh/beatmaps/478405/covers/cover.jpg?1650647231', 6.014, 9.2, 8, 4, 224, 240, NULL, NULL, NULL, NULL),
    ('Finals', 'NM', '#5b8def', 0, 0, '', 553131, 158023, 'UNDEAD CORPORATION', 'Everything will freeze', 'Lunatic', 'Ekoro', 'https://assets.ppy.sh/beatmaps/158023/covers/cover.jpg?1645787427', 5.8872, 9, 8, 4, 240, 194, NULL, NULL, NULL, NULL),
    ('Finals', 'NM', '#5b8def', 0, 1, '', 759056, 185250, 'ALiCE''S EMOTiON', 'Dark Flight Dreamer', 'Shawajoy''s Lunatic', 'Sakaue Nachi', 'https://assets.ppy.sh/beatmaps/185250/covers/cover.jpg?1650631939', 5.8103, 9.2, 8, 4, 189, 189, NULL, NULL, NULL, NULL),
    ('Finals', 'NM', '#5b8def', 0, 2, '', 1040942, 488238, 'cYsmix', 'Classic Pursuit', 'Expert', '-kevincela-', 'https://assets.ppy.sh/beatmaps/488238/covers/cover.jpg?1631509747', 5.8283, 9.3, 8, 4.3, 140, 139, NULL, NULL, NULL, NULL),
    ('Finals', 'NM', '#5b8def', 0, 3, '', 2064221, 985788, 'Loki', 'Wizard''s Tower', 'Daycore''s Extra', 'Taeyang', 'https://assets.ppy.sh/beatmaps/985788/covers/cover.jpg?1650681317', 5.7848, 9.2, 8.7, 4.5, 175, 160, NULL, NULL, NULL, NULL),
    ('Finals', 'HD', '#e8b84a', 1, 0, 'HD', 1501684, 671199, 'DECO*27', 'HIBANA feat. Hatsune Miku', 'neonat''s Extra', 'Pho', 'https://assets.ppy.sh/beatmaps/671199/covers/cover.jpg?1650657296', 5.9782, 9.2, 8, 4, 200, 203, NULL, NULL, NULL, NULL),
    ('Finals', 'HD', '#e8b84a', 1, 1, 'HD', 895077, 410998, 'Stonebank', 'All Night', 'Hi-Speed Extra', 'Monstrata', 'https://assets.ppy.sh/beatmaps/410998/covers/cover.jpg?1650636926', 5.6817, 9.6, 7, 3, 170, 113, NULL, NULL, NULL, NULL),
    ('Finals', 'HR', '#f3877e', 2, 0, 'HR', 69405, 19789, 'Hatsune Miku & Megpoid Gumi', 'MATRYOSHKA', 'Insane', 'gowww', 'https://assets.ppy.sh/beatmaps/19789/covers/cover.jpg?1645778675', 5.5548, 10, 10, 5.2, 205, 198, NULL, NULL, NULL, NULL),
    ('Finals', 'HR', '#f3877e', 2, 1, 'HR', 848345, 380545, 'Primary', 'Sweets Weekend', 'Sweet Neko', 'CaffeAmericano', 'https://assets.ppy.sh/beatmaps/380545/covers/cover.jpg?1622096656', 5.8056, 10, 10, 5.2, 180, 93, NULL, NULL, NULL, NULL),
    ('Finals', 'DT', '#b48cf2', 3, 0, 'DT', 1555610, 737128, 'ONE OK ROCK', 'Bombs away', 'Another', 'A r M i N', 'https://assets.ppy.sh/beatmaps/737128/covers/cover.jpg?1650659486', 6.4815, 10.33, 9.84, 4, 240, 69, NULL, NULL, NULL, NULL),
    ('Finals', 'DT', '#b48cf2', 3, 1, 'DT', 846261, 387784, 'Shawn Wasabi', 'Marble Soda', 'Narcissu''s Insane', 'Len', 'https://assets.ppy.sh/beatmaps/387784/covers/cover.jpg?1650635195', 6.2735, 10.33, 9.44, 4, 252, 107, NULL, NULL, NULL, NULL),
    ('Finals', 'DT', '#b48cf2', 3, 2, 'DT', 519080, 221954, 'Tsunamaru', 'Daidai Genome', 'Insane', 'Lust', 'https://assets.ppy.sh/beatmaps/221954/covers/cover.jpg?1650625685', 6.2782, 9.67, 9.78, 4, 202.5, 121, NULL, NULL, NULL, NULL),
    ('Finals', 'FM', '#5fd38d', 4, 0, '', 487589, 186318, 'Warak', 'REANIMATE', 'xsrs''s Extra', 'iyasine', 'https://assets.ppy.sh/beatmaps/186318/covers/cover.jpg?1650623687', 5.6651, 9.5, 8, 4, 174, 151, 6.1242, 5.2, 10, 10),
    ('Finals', 'FM', '#5fd38d', 4, 1, '', 2144226, 1025171, 'LiSA', 'Gurenge', 'Extra', 'Crissa', 'https://assets.ppy.sh/beatmaps/1025171/covers/cover.jpg?1650684543', 5.7738, 9.3, 9, 4.8, 135, 232, 6.5955, 6.24, 10, 10),
    ('Finals', 'TB', '#ff66ab', 5, 0, '', 1351114, 636839, 'KASAI HARCORES', 'Cycle Hit', 'Home Run', 'Worminators', 'https://assets.ppy.sh/beatmaps/636839/covers/cover.jpg?1641132312', 6.2375, 9.5, 8.7, 3.8, 175, 307, NULL, NULL, NULL, NULL),
    ('Grand Final', 'NM', '#5b8def', 0, 0, '', 366140, 147962, 'UPLIFT SPICE', 'Omega Rhythm', 'Extra', 'Jemmmmy', 'https://assets.ppy.sh/beatmaps/147962/covers/cover.jpg?1650620437', 5.9806, 9.2, 8.5, 4, 173, 178, NULL, NULL, NULL, NULL),
    ('Grand Final', 'NM', '#5b8def', 0, 1, '', 1569034, 732994, 'Team.NEKOKAN', 'Airman ga Taosenai', 'Ger M i N''s Extra', 'Sotarks', 'https://assets.ppy.sh/beatmaps/732994/covers/cover.jpg?1650660274', 5.9696, 9.4, 8, 3.5, 200, 221, NULL, NULL, NULL, NULL),
    ('Grand Final', 'NM', '#5b8def', 0, 2, '', 1650477, 785858, 'THE ORAL CIGARETTES', 'ReI', 'Nevo''s Expert', 'Sotarks', 'https://assets.ppy.sh/beatmaps/785858/covers/cover.jpg?1650664560', 5.9257, 9.3, 9, 3.8, 186, 115, NULL, NULL, NULL, NULL),
    ('Grand Final', 'NM', '#5b8def', 0, 3, '', 404429, 139525, 'Lite Show Magic (t+pazolite vs C-Show)', 'Crack Traxxxx', 'Gero''s Another', 'Fatfan Kolek', 'https://assets.ppy.sh/beatmaps/139525/covers/cover.jpg?1650630208', 5.9309, 9.2, 8, 4, 220, 116, NULL, NULL, NULL, NULL),
    ('Grand Final', 'HD', '#e8b84a', 1, 0, 'HD', 1400414, 661483, 'Reol', 'MONSTER', 'DARKNESS', 'Sotarks', 'https://assets.ppy.sh/beatmaps/661483/covers/cover.jpg?1622123968', 6.0545, 9, 9, 3.7, 164, 96, NULL, NULL, NULL, NULL),
    ('Grand Final', 'HD', '#e8b84a', 1, 1, 'HD', 888305, 409164, 'S3RL feat Mixie Moon', 'FriendZoned', 'Sorry, But We Are Just Friends', 'FCL', 'https://assets.ppy.sh/beatmaps/409164/covers/cover.jpg?1650636524', 6.1611, 9.2, 8.6, 4.3, 175, 168, NULL, NULL, NULL, NULL),
    ('Grand Final', 'HR', '#f3877e', 2, 0, 'HR', 93842, 28107, 'DJ Fresh', 'Gold Dust', 'Insane', 'galvenize', 'https://assets.ppy.sh/beatmaps/28107/covers/cover.jpg?1622045706', 5.8471, 10, 9.8, 5.2, 177, 182, NULL, NULL, NULL, NULL),
    ('Grand Final', 'HR', '#f3877e', 2, 1, 'HR', 382930, 143397, 'Natsume Chiaki', 'Hanairo Biyori', 'Extra', 'rinsukir', 'https://assets.ppy.sh/beatmaps/143397/covers/cover.jpg?1650625350', 5.8908, 10, 10, 5.2, 182, 192, NULL, NULL, NULL, NULL),
    ('Grand Final', 'DT', '#b48cf2', 3, 0, 'DT', 119803, 36858, 'Hatsune Miku', 'Senbonzakura (Short Ver.)', 'Rin', 'NatsumeRin', 'https://assets.ppy.sh/beatmaps/36858/covers/cover.jpg?1622049909', 6.7435, 9.67, 9.78, 4, 231, 101, NULL, NULL, NULL, NULL),
    ('Grand Final', 'DT', '#b48cf2', 3, 1, 'DT', 1038400, 486142, 'cYsmix', 'Fright March', 'Insane', 'Gero', 'https://assets.ppy.sh/beatmaps/486142/covers/cover.jpg?1631509649', 6.6778, 10, 9.11, 4, 240, 99, NULL, NULL, NULL, NULL),
    ('Grand Final', 'DT', '#b48cf2', 3, 2, 'DT', 897901, 413707, 'Milkychan', 'Stronger Than You -Chara Response-', 'Insane', 'Cosmolade', 'https://assets.ppy.sh/beatmaps/413707/covers/cover.jpg?1650644445', 6.7764, 10.33, 9.11, 4, 147, 110, NULL, NULL, NULL, NULL),
    ('Grand Final', 'FM', '#5fd38d', 4, 0, '', 953586, 443751, 'Chino(CV.Minase Inori)', 'Shinsaku no Shiawase wa Kochira!', 'Happy~!', 'Lami', 'https://assets.ppy.sh/beatmaps/443751/covers/cover.jpg?1650639287', 5.7427, 9.2, 8.5, 4, 175, 124, 6.1876, 5.2, 10, 10),
    ('Grand Final', 'FM', '#5fd38d', 4, 1, '', 1595191, 758344, 'Colors*Slash', 'Colors Power ni Omakasero!', 'Rainbows', 'Sotarks', 'https://assets.ppy.sh/beatmaps/758344/covers/cover.jpg?1650661435', 5.7688, 9.4, 9.2, 3.8, 200, 97, 6.2173, 4.94, 10, 10),
    ('Grand Final', 'TB', '#ff66ab', 5, 0, '', 550235, 237768, 'Our Stolen Theory', 'United (L.A.O.S Remix)', 'Infinity', 'Asphyxia', 'https://assets.ppy.sh/beatmaps/237768/covers/cover.jpg?1632036015', 6.2899, 9.2, 9, 4, 175, 326, NULL, NULL, NULL, NULL)
),
targets AS (
    SELECT DISTINCT ON (s.name) s.id, s.name
    FROM stages s
    WHERE s.name IN (SELECT stage FROM pool)
    ORDER BY s.name, s.position
),
missing AS (
    SELECT DISTINCT t.id AS stage_id, p.category, p.cat_pos, p.color
    FROM pool p
    JOIN targets t ON t.name = p.stage
    WHERE NOT EXISTS (
        SELECT 1 FROM stage_categories c
        WHERE c.stage_id = t.id AND lower(c.name) = lower(p.category)
    )
),
categories AS (
    INSERT INTO stage_categories (stage_id, name, position, color)
    SELECT m.stage_id, m.category,
           COALESCE((SELECT max(c.position) + 1 FROM stage_categories c
                     WHERE c.stage_id = m.stage_id), 0) + m.cat_pos,
           m.color
    FROM missing m
    RETURNING id, stage_id, name
),
slots AS (
    INSERT INTO category_slots (category_id, position)
    SELECT c.id, p.slot_pos
    FROM pool p
    JOIN targets t ON t.name = p.stage
    JOIN categories c ON c.stage_id = t.id AND c.name = p.category
    RETURNING id, category_id, position
)
INSERT INTO pool_maps (slot_id, mods, beatmap_id, beatmapset_id, artist, title, version, creator,
                       mode, cover_url, star_rating, ar, od, cs, bpm, total_length,
                       hr_star_rating, hr_cs, hr_ar, hr_od, added_by)
SELECT s.id, p.mods, p.beatmap_id, p.beatmapset_id, p.artist, p.title, p.version, p.creator,
       'osu', p.cover_url, p.star_rating, p.ar, p.od, p.cs, p.bpm, p.total_length,
       p.hr_star_rating, p.hr_cs, p.hr_ar, p.hr_od,
       (SELECT id FROM osu_users WHERE osu_id = -2)
FROM pool p
JOIN targets t ON t.name = p.stage
JOIN categories c ON c.stage_id = t.id AND c.name = p.category
JOIN slots s ON s.category_id = c.id AND s.position = p.slot_pos;

-- A 32-team double-elimination bracket, partway through: the Round of 32 and the
-- Round of 16 are over, and the Quarterfinals (winners round 3 and losers rounds 2 and
-- 3) are the one stage being played: three of winners round 3 and six of losers round 2
-- are done, the next one of each is live and the rest are still to come. Everything
-- after it is upcoming, the last rounds still unscheduled. Teams are seeded by their
-- players' combined rank in the standard order (1v32, 16v17, ...). Losers rounds
-- alternate between pairing up the survivors and taking the teams that drop from
-- winners round r + 1, uncrossed. The grand final is the last winners match, followed
-- by its bracket reset, which the same two teams play.
--
-- Each stage is a week, and no team plays more than two matches in one: winners round
-- 3 and losers rounds 2-3 make the Quarterfinals, winners round 4 and losers rounds 4-5
-- the Semifinals, the winners final and losers rounds 6-7 the Finals, and the losers
-- final, the grand final and its reset the Grand Final. A team's second match of the
-- week waits on another match of the same week, so the site gives it potential
-- matches (see below).
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
    pair record;
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
            ('w1', 'winners'::bracket_side, 16, 0,   0,   6,  interval '-6 days',  interval '1 hour',  16, false, 'Round of 32'),
            ('w2', 'winners', 8,  15,  3,   12, interval '-4 days',    interval '1 hour',  8,  false, 'Round of 16'),
            ('w3', 'winners', 4,  30,  9,   24, interval '-3 hours',   interval '1 hour',  3,  true,  'Quarterfinals'),
            ('w4', 'winners', 2,  45,  21,  48, interval '2 days',     interval '2 hours', 0,  false, 'Semifinals'),
            ('w5', 'winners', 1,  60,  45,  0,  interval '3 days 4 hours', interval '0', 0, false, 'Finals'),
            ('gf', 'winners', 1,  135, 84,  0,  NULL,                  NULL,               0,  false, 'Grand Final'),
            ('gr', 'winners', 1,  151, 84,  0,  NULL,                  NULL,               0,  false, 'Grand Final'),
            ('l1', 'losers',  8,  15,  102, 6,  interval '-3 days',    interval '1 hour',  8,  false, 'Round of 16'),
            ('l2', 'losers',  8,  30,  102, 6,  interval '-6 hours',   interval '1 hour',  6,  true,  'Quarterfinals'),
            ('l3', 'losers',  4,  45,  105, 12, interval '1 day',      interval '1 hour',  0,  false, 'Quarterfinals'),
            ('l4', 'losers',  4,  60,  105, 12, interval '1 day 6 hours', interval '1 hour', 0, false, 'Semifinals'),
            ('l5', 'losers',  2,  75,  111, 24, interval '2 days 6 hours', interval '1 hour', 0, false, 'Semifinals'),
            ('l6', 'losers',  2,  90,  111, 24, interval '3 days',     interval '1 hour',  0,  false, 'Finals'),
            ('l7', 'losers',  1,  105, 123, 0,  NULL,                  NULL,               0,  false, 'Finals'),
            ('l8', 'losers',  1,  120, 123, 0,  NULL,                  NULL,               0,  false, 'Grand Final')
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
            INSERT INTO matches (kind, starts_at, referee_id, concluded_at)
            VALUES (
                'bracket',
                starts,
                (SELECT id FROM osu_users
                 WHERE osu_id = CASE WHEN num % 2 = 0 THEN -3 ELSE -4 END),
                CASE WHEN idx < rnd.done THEN starts + interval '1 hour' END
            )
            RETURNING id INTO new_match;
            IF idx < rnd.done OR (idx = rnd.done AND rnd.live) THEN
                INSERT INTO match_rooms (mp_id, match_id, added_at)
                VALUES (CASE WHEN rnd.side = 'winners' THEN 112234000 ELSE 112235000 END + num,
                        new_match, starts);
            END IF;
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
        -- The reset: the grand final's loser in slot 1, its winner in slot 2.
        UNION ALL
        SELECT 'gf-0', 'loser', 'gr-0', 1
        UNION ALL
        SELECT 'gf-0', 'winner', 'gr-0', 2
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
      AND NOT EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
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

    -- L20 waits on L15 (being played) and L16 (still to come), both of its week, so the
    -- site gives it a potential match for each pairing of their teams (crate::potentials).
    -- They're made here so they can have times, staff, proposals and predictions; the
    -- backend keeps them as they are. Like the site's, they start on L20's time, referee
    -- and streamer, and then: the two better seeds' pairing agreed on two hours earlier,
    -- the next one's referee gave it up, DevHakem and DevYayinci took the third, and in
    -- the last one team 2 has offered an hour later, which team 1 hasn't read yet.
    FOR pair IN
        SELECT a.team AS team1_id, c.team AS team2_id, a.n * 10 + c.n AS key
        FROM bracket_matches s1
        CROSS JOIN bracket_matches s2
        CROSS JOIN LATERAL (VALUES (s1.team1_id, 1), (s1.team2_id, 2)) AS a (team, n)
        CROSS JOIN LATERAL (VALUES (s2.team1_id, 1), (s2.team2_id, 2)) AS c (team, n)
        WHERE s1.match_id = (ids ->> 'l2-6')::uuid AND s2.match_id = (ids ->> 'l2-7')::uuid
        ORDER BY a.n, c.n
    LOOP
        INSERT INTO matches (kind, starts_at, referee_id)
        SELECT 'potential',
               m.starts_at - CASE WHEN pair.key = 11 THEN interval '2 hours' ELSE interval '0' END,
               CASE pair.key
                   WHEN 12 THEN NULL
                   WHEN 21 THEN (SELECT id FROM osu_users WHERE osu_id = -4)
                   ELSE m.referee_id
               END
        FROM matches m
        WHERE m.id = (ids ->> 'l3-3')::uuid
        RETURNING id INTO new_match;
        INSERT INTO potential_matches (match_id, bracket_match_id, team1_id, team2_id, streamer_id)
        SELECT new_match, b.match_id, pair.team1_id, pair.team2_id,
               CASE WHEN pair.key = 21 THEN (SELECT id FROM osu_users WHERE osu_id = -5)
                    ELSE b.streamer_id END
        FROM bracket_matches b
        WHERE b.match_id = (ids ->> 'l3-3')::uuid;
        IF pair.key = 22 THEN
            INSERT INTO match_reschedules (match_id, team_id, proposed_by, starts_at, created_at)
            SELECT new_match, pair.team2_id, tm.user_id, m.starts_at + interval '1 hour',
                   now() - interval '40 minutes'
            FROM matches m
            CROSS JOIN LATERAL (
                SELECT user_id FROM team_members WHERE team_id = pair.team2_id
                ORDER BY joined_at, user_id LIMIT 1
            ) AS tm
            WHERE m.id = new_match;
            INSERT INTO notifications (user_id, kind, match_id, team_id, actor_id, created_at)
            SELECT tm.user_id, 'reschedule_proposed', new_match, pair.team1_id, r.proposed_by,
                   r.created_at
            FROM team_members tm
            JOIN match_reschedules r ON r.match_id = new_match
            WHERE tm.team_id = pair.team1_id;
        END IF;
    END LOOP;
END $$;

-- W1's rooms and their chat, so the referee page has a concluded match to show them
-- for. The first room broke after three maps and the match went on in a second one.
-- Team 1 plays blue (slots 1-2) and takes five maps to red's three, like the seeded 5-3. The maps
-- are picked from W1's stage pool, with made-up ones should that stage have none.
INSERT INTO match_rooms (mp_id, match_id, added_at)
SELECT 112236001, m.id, m.starts_at + interval '20 minutes'
FROM bracket_matches b
JOIN matches m ON m.id = b.match_id
WHERE b.side = 'winners' AND b.number = 1
  AND EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id);

WITH w1 AS (
    SELECT m.starts_at AS t0, b.team1_id, b.team2_id, b.stage_id, ref.username AS referee,
           ARRAY(SELECT r.mp_id FROM match_rooms r WHERE r.match_id = m.id
                 ORDER BY r.added_at) AS rooms
    FROM bracket_matches b
    JOIN matches m ON m.id = b.match_id
    LEFT JOIN osu_users ref ON ref.id = m.referee_id
    WHERE b.side = 'winners' AND b.number = 1
),
players AS (
    SELECT u.username, u.osu_id, tm.team_id = w1.team1_id AS team1,
           row_number() OVER (ORDER BY tm.team_id = w1.team1_id DESC, u.username) AS slot
    FROM w1
    JOIN team_members tm ON tm.team_id IN (w1.team1_id, w1.team2_id)
    JOIN osu_users u ON u.id = tm.user_id
),
-- The picks, in order: category and which of its slots (0-based).
picks (k, category, slot) AS (
    VALUES (1, 'NM', 0), (2, 'HD', 0), (3, 'HR', 0), (4, 'DT', 0),
           (5, 'NM', 1), (6, 'FM', 0), (7, 'DT', 1), (8, 'HR', 1)
),
-- Maps 1-3 in the first room, 4-8 in the second, which opens 20 minutes in.
maps AS (
    SELECT pk.k, pk.category, CASE WHEN pk.k <= 3 THEN 1 ELSE 2 END AS room,
           interval '3 minutes' + (pk.k - 1) * interval '6 minutes'
               + CASE WHEN pk.k > 3 THEN interval '3 minutes' ELSE interval '0' END AS at,
           COALESCE(pm.beatmap_id, 4000000 + pk.k) AS beatmap_id,
           COALESCE(pm.artist || ' - ' || pm.title || ' [' || pm.version || ']',
                    'Seed Artist - Map ' || pk.k || ' [Tournament]') AS name
    FROM w1
    CROSS JOIN picks pk
    LEFT JOIN LATERAL (
        SELECT pm.beatmap_id, pm.artist, pm.title, pm.version
        FROM stage_categories c
        JOIN category_slots s ON s.category_id = c.id
        JOIN pool_maps pm ON pm.slot_id = s.id
        WHERE c.stage_id = w1.stage_id AND c.name = pk.category
        ORDER BY s.position
        OFFSET pk.slot LIMIT 1
    ) pm ON true
),
lines (room, at, sender, text) AS (
    SELECT room, interval '20 minutes' * (room - 1) + interval '20 seconds' * p.slot, 'BanchoBot',
           p.username || ' joined in slot ' || p.slot || ' for team '
               || CASE WHEN p.team1 THEN 'blue' ELSE 'red' END || '.'
    FROM players p, generate_series(1, 2) room
    UNION ALL
    SELECT 1, interval '2 minutes', COALESCE(w1.referee, 'BanchoBot'), 'glhf! İlk pick kırmızı takımda.'
    FROM w1
    UNION ALL
    SELECT m.room, m.at - interval '10 seconds', COALESCE(w1.referee, 'BanchoBot'),
           '!mp map ' || m.beatmap_id || ' 0'
    FROM maps m, w1
    UNION ALL
    SELECT m.room, m.at, 'BanchoBot',
           'Changed beatmap to https://osu.ppy.sh/b/' || m.beatmap_id || ' ' || m.name
    FROM maps m
    UNION ALL
    SELECT m.room, m.at + interval '5 seconds', COALESCE(w1.referee, 'BanchoBot'),
           '!mp mods ' || CASE m.category WHEN 'HD' THEN 'HD NF' WHEN 'HR' THEN 'HR NF'
                                          WHEN 'DT' THEN 'DT NF' WHEN 'FM' THEN 'Freemod' ELSE 'NF' END
    FROM maps m, w1
    UNION ALL
    SELECT m.room, m.at + interval '6 seconds', 'BanchoBot',
           CASE m.category
               WHEN 'HD' THEN 'Enabled Hidden, NoFail, disabled FreeMod'
               WHEN 'HR' THEN 'Enabled HardRock, NoFail, disabled FreeMod'
               WHEN 'DT' THEN 'Enabled DoubleTime, NoFail, disabled FreeMod'
               WHEN 'FM' THEN 'Disabled all mods, enabled FreeMod'
               ELSE 'Enabled NoFail, disabled FreeMod'
           END
    FROM maps m
    UNION ALL
    SELECT m.room, m.at + interval '30 seconds', 'BanchoBot', 'The match has started!'
    FROM maps m
    -- The referee page asks for the room's settings as each map starts, which is how it
    -- reads the mods the players picked on the freemod map.
    UNION ALL
    SELECT m.room, m.at + interval '31 seconds', COALESCE(w1.referee, 'BanchoBot'), '!mp settings'
    FROM maps m, w1
    UNION ALL
    SELECT m.room, m.at + interval '32 seconds', 'BanchoBot',
           'Active mods: ' || CASE m.category WHEN 'HD' THEN 'Hidden, NoFail' WHEN 'HR' THEN 'HardRock, NoFail'
                                              WHEN 'DT' THEN 'DoubleTime, NoFail' WHEN 'FM' THEN 'Freemod'
                                              ELSE 'NoFail' END
    FROM maps m
    UNION ALL
    SELECT m.room, m.at + interval '33 seconds', 'BanchoBot', 'Players: ' || (SELECT count(*) FROM players)
    FROM maps m
    UNION ALL
    SELECT m.room, m.at + interval '33 seconds' + p.slot * interval '1 second', 'BanchoBot',
           'Slot ' || p.slot || '  Not Ready https://osu.ppy.sh/u/' || p.osu_id || ' ' || rpad(p.username, 16)
               || ' [Team ' || CASE WHEN p.team1 THEN 'Blue' ELSE 'Red' END
               || CASE WHEN m.category <> 'FM' OR p.slot = 3 THEN ''
                       WHEN p.slot = 1 THEN ' / Hidden'
                       WHEN p.slot = 2 THEN ' / HardRock'
                       ELSE ' / Hidden, HardRock'
                  END || ']'
    FROM maps m, players p
    UNION ALL
    SELECT m.room, m.at + interval '4 minutes' + p.slot * interval '1 second', 'BanchoBot',
           p.username || ' finished playing (Score: '
               || CASE WHEN p.team1 = (m.k NOT IN (2, 4, 6)) THEN 700000 ELSE 450000 END
                  + p.slot * 13579 + m.k * 2468
               || ', PASSED).'
    FROM maps m, players p
    UNION ALL
    SELECT m.room, m.at + interval '4 minutes 10 seconds', 'BanchoBot', 'The match has finished!'
    FROM maps m
    UNION ALL
    SELECT 1, interval '19 minutes', COALESCE(w1.referee, 'BanchoBot'),
           'Oda bozuldu, yenisini açıyorum. Davetleri kontrol edin.'
    FROM w1
    UNION ALL
    SELECT 2, interval '55 minutes' + p.slot * interval '2 seconds', p.username, 'gg'
    FROM players p
)
INSERT INTO room_chat_messages (mp_id, sent_at, sender, text)
SELECT w1.rooms[l.room], w1.t0 + l.at, l.sender, l.text
FROM w1, lines l
WHERE w1.rooms[l.room] IS NOT NULL
ORDER BY l.at;

-- W1's bans, from maps it didn't play, made after the first two picks as its stage's
-- order (p1-p2-b2-b1-p1-*) has them: red (team 2) took out DT3, then blue NM4.
INSERT INTO match_bans (match_id, pool_map_id, team_id, created_at)
SELECT b.match_id, pm.id, CASE WHEN v.team1 THEN b.team1_id ELSE b.team2_id END, m.starts_at + v.at
FROM bracket_matches b
JOIN matches m ON m.id = b.match_id
CROSS JOIN (VALUES (false, 'DT', 2, interval '19 minutes 20 seconds'),
                   (true, 'NM', 3, interval '19 minutes 40 seconds')) AS v (team1, category, slot, at)
JOIN stage_categories c ON c.stage_id = b.stage_id AND c.name = v.category
JOIN category_slots s ON s.category_id = c.id AND s.position = v.slot
JOIN pool_maps pm ON pm.slot_id = s.id
WHERE b.side = 'winners' AND b.number = 1
  AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL;

-- W1's picks: the maps its chat plays, in the same order, the teams taking turns
-- from blue (team 1, which picked first and so is 1 in the order).
INSERT INTO match_picks (match_id, pool_map_id, team_id, created_at)
SELECT b.match_id, pm.id, CASE WHEN v.k % 2 = 1 THEN b.team1_id ELSE b.team2_id END,
       m.starts_at + interval '2 minutes' + v.k * interval '6 minutes'
FROM bracket_matches b
JOIN matches m ON m.id = b.match_id
CROSS JOIN (VALUES (1, 'NM', 0), (2, 'HD', 0), (3, 'HR', 0), (4, 'DT', 0),
                   (5, 'NM', 1), (6, 'FM', 0), (7, 'DT', 1), (8, 'HR', 1)) AS v (k, category, slot)
JOIN stage_categories c ON c.stage_id = b.stage_id AND c.name = v.category
JOIN category_slots s ON s.category_id = c.id AND s.position = v.slot
JOIN pool_maps pm ON pm.slot_id = s.id
WHERE b.side = 'winners' AND b.number = 1
  AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL;

-- Two matches being played right now, each the first live one of its side. The winners
-- one is mid-match: blue (team 1) took NM1, red HD1, the two banned, and FM1 (blue's
-- pick) is being played, with everyone's mods read from !mp settings as it started.
-- The losers one is at the tiebreaker: its eight picks (W1's order) went to the teams
-- in turn, 4-4 under its stage's Bo9, and the room still has the last of them on, so
-- the referee page's room automation would put TB on next. Their scores and finished maps are saved as the
-- referee page saves them, so the pages show them live on the site.
WITH live AS (
    SELECT DISTINCT ON (b.side)
           m.id, b.side, m.starts_at AS t0, b.team1_id, b.team2_id, b.stage_id,
           ref.username AS referee,
           (SELECT r.mp_id FROM match_rooms r WHERE r.match_id = m.id
            ORDER BY r.added_at DESC LIMIT 1) AS mp_id
    FROM bracket_matches b
    JOIN matches m ON m.id = b.match_id
    LEFT JOIN osu_users ref ON ref.id = m.referee_id
    WHERE m.concluded_at IS NULL
      AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
      AND EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
      AND NOT EXISTS (
          SELECT 1 FROM team_members tm JOIN osu_users u ON u.id = tm.user_id
          WHERE tm.team_id IN (b.team1_id, b.team2_id) AND u.osu_id >= 0
      )
    ORDER BY b.side, b.number
),
players AS (
    SELECT live.id AS match_id, u.id AS user_id, u.username, u.osu_id,
           tm.team_id = live.team1_id AS team1,
           row_number() OVER (PARTITION BY live.id
                              ORDER BY tm.team_id = live.team1_id DESC, u.username) AS slot
    FROM live
    JOIN team_members tm ON tm.team_id IN (live.team1_id, live.team2_id)
    JOIN osu_users u ON u.id = tm.user_id
),
maps AS (
    SELECT live.id AS match_id, pk.k, pk.category, pk.finished, pk.k % 2 = 1 AS team1_wins,
           interval '3 minutes' + (pk.k - 1) * interval '6 minutes' AS at,
           pm.id AS map_id,
           COALESCE(pm.beatmap_id, 4100000 + pk.k) AS beatmap_id,
           COALESCE(pm.artist || ' - ' || pm.title || ' [' || pm.version || ']',
                    'Seed Artist - Map ' || pk.k || ' [Tournament]') AS name
    FROM live
    JOIN (VALUES ('winners', 1, 'NM', 0, true), ('winners', 2, 'HD', 0, true),
                 ('winners', 3, 'FM', 0, false),
                 ('losers', 1, 'NM', 0, true), ('losers', 2, 'HD', 0, true),
                 ('losers', 3, 'HR', 0, true), ('losers', 4, 'DT', 0, true),
                 ('losers', 5, 'NM', 1, true), ('losers', 6, 'FM', 0, true),
                 ('losers', 7, 'DT', 1, true), ('losers', 8, 'HR', 1, true))
        AS pk (side, k, category, slot, finished)
      ON pk.side = live.side::text
    LEFT JOIN LATERAL (
        SELECT pm.id, pm.beatmap_id, pm.artist, pm.title, pm.version
        FROM stage_categories c
        JOIN category_slots s ON s.category_id = c.id
        JOIN pool_maps pm ON pm.slot_id = s.id
        WHERE c.stage_id = live.stage_id AND c.name = pk.category
        ORDER BY s.position
        OFFSET pk.slot LIMIT 1
    ) pm ON true
),
-- Each player's score and mods on the finished maps, like W1's chat makes them up.
scores AS (
    SELECT m.match_id, m.k, p.user_id, p.username, p.slot, p.team1,
           CASE WHEN p.team1 = m.team1_wins THEN 700000 ELSE 450000 END
               + p.slot * 13579 + m.k * 2468 AS score,
           CASE m.category
               WHEN 'NM' THEN 'NF' WHEN 'HD' THEN 'HDNF' WHEN 'HR' THEN 'HRNF' WHEN 'DT' THEN 'DTNF'
               ELSE (ARRAY['HD', 'HR', '', 'HDHR'])[p.slot]
           END AS mods
    FROM maps m
    JOIN players p ON p.match_id = m.match_id
    WHERE m.finished
),
lines (match_id, at, sender, text) AS (
    SELECT p.match_id, interval '20 seconds' * p.slot, 'BanchoBot',
           p.username || ' joined in slot ' || p.slot || ' for team '
               || CASE WHEN p.team1 THEN 'blue' ELSE 'red' END || '.'
    FROM players p
    UNION ALL
    SELECT m.match_id, m.at - interval '10 seconds', COALESCE(live.referee, 'BanchoBot'),
           '!mp map ' || m.beatmap_id || ' 0'
    FROM maps m JOIN live ON live.id = m.match_id
    UNION ALL
    SELECT m.match_id, m.at, 'BanchoBot',
           'Changed beatmap to https://osu.ppy.sh/b/' || m.beatmap_id || ' ' || m.name
    FROM maps m
    UNION ALL
    SELECT m.match_id, m.at + interval '6 seconds', 'BanchoBot',
           CASE m.category
               WHEN 'HD' THEN 'Enabled Hidden, NoFail, disabled FreeMod'
               WHEN 'HR' THEN 'Enabled HardRock, NoFail, disabled FreeMod'
               WHEN 'DT' THEN 'Enabled DoubleTime, NoFail, disabled FreeMod'
               WHEN 'FM' THEN 'Disabled all mods, enabled FreeMod'
               ELSE 'Enabled NoFail, disabled FreeMod'
           END
    FROM maps m
    UNION ALL
    SELECT m.match_id, m.at + interval '30 seconds', 'BanchoBot', 'The match has started!'
    FROM maps m
    UNION ALL
    SELECT m.match_id, m.at + interval '31 seconds', COALESCE(live.referee, 'BanchoBot'), '!mp settings'
    FROM maps m JOIN live ON live.id = m.match_id
    UNION ALL
    SELECT m.match_id, m.at + interval '32 seconds', 'BanchoBot',
           'Active mods: ' || CASE m.category WHEN 'HD' THEN 'Hidden, NoFail' WHEN 'HR' THEN 'HardRock, NoFail'
                                              WHEN 'DT' THEN 'DoubleTime, NoFail' WHEN 'FM' THEN 'Freemod'
                                              ELSE 'NoFail' END
    FROM maps m
    UNION ALL
    SELECT m.match_id, m.at + interval '33 seconds', 'BanchoBot',
           'Players: ' || (SELECT count(*) FROM players p WHERE p.match_id = m.match_id)
    FROM maps m
    UNION ALL
    SELECT m.match_id, m.at + interval '33 seconds' + p.slot * interval '1 second', 'BanchoBot',
           'Slot ' || p.slot || '  Not Ready https://osu.ppy.sh/u/' || p.osu_id || ' ' || rpad(p.username, 16)
               || ' [Team ' || CASE WHEN p.team1 THEN 'Blue' ELSE 'Red' END
               || CASE WHEN m.category <> 'FM' OR p.slot = 3 THEN ''
                       WHEN p.slot = 1 THEN ' / Hidden'
                       WHEN p.slot = 2 THEN ' / HardRock'
                       ELSE ' / Hidden, HardRock'
                  END || ']'
    FROM maps m JOIN players p ON p.match_id = m.match_id
    UNION ALL
    SELECT m.match_id, m.at + interval '4 minutes' + sc.slot * interval '1 second', 'BanchoBot',
           sc.username || ' finished playing (Score: ' || sc.score || ', PASSED).'
    FROM maps m JOIN scores sc ON sc.match_id = m.match_id AND sc.k = m.k
    UNION ALL
    SELECT m.match_id, m.at + interval '4 minutes 10 seconds', 'BanchoBot', 'The match has finished!'
    FROM maps m
    WHERE m.finished
),
chat AS (
    INSERT INTO room_chat_messages (mp_id, sent_at, sender, text)
    SELECT live.mp_id, live.t0 + l.at, l.sender, l.text
    FROM live JOIN lines l ON l.match_id = live.id
    ORDER BY live.id, l.at
),
score AS (
    UPDATE bracket_matches b
    SET team1_score = w.team1, team2_score = w.team2
    FROM (
        SELECT match_id,
               count(*) FILTER (WHERE finished AND team1_wins) AS team1,
               count(*) FILTER (WHERE finished AND NOT team1_wins) AS team2
        FROM maps
        GROUP BY match_id
    ) AS w
    WHERE b.match_id = w.match_id
),
-- Both matches' drafts, as the Quarterfinals' order (p1-p2-b2-b1-p1-*) has them: the
-- teams picked their maps in turn from blue (team 1), each a minute before it was
-- played, and after the second map red (team 2) banned DT3 and then blue NM4.
bans AS (
    INSERT INTO match_bans (match_id, pool_map_id, team_id, created_at)
    SELECT live.id, pm.id, CASE WHEN v.team1 THEN live.team1_id ELSE live.team2_id END, live.t0 + v.at
    FROM live
    CROSS JOIN (VALUES (false, 'DT', 2, interval '13 minutes 20 seconds'),
                       (true, 'NM', 3, interval '13 minutes 40 seconds')) AS v (team1, category, slot, at)
    JOIN stage_categories c ON c.stage_id = live.stage_id AND c.name = v.category
    JOIN category_slots s ON s.category_id = c.id AND s.position = v.slot
    JOIN pool_maps pm ON pm.slot_id = s.id
),
picks AS (
    INSERT INTO match_picks (match_id, pool_map_id, team_id, created_at)
    SELECT live.id, m.map_id, CASE WHEN m.k % 2 = 1 THEN live.team1_id ELSE live.team2_id END,
           live.t0 + m.at - interval '1 minute'
    FROM live
    JOIN maps m ON m.match_id = live.id
    WHERE m.map_id IS NOT NULL
),
results AS (
    INSERT INTO match_map_results (match_id, position, pool_map_id, team1_score, team2_score)
    SELECT m.match_id, m.k, m.map_id,
           (SELECT sum(sc.score) FROM scores sc WHERE sc.match_id = m.match_id AND sc.k = m.k AND sc.team1),
           (SELECT sum(sc.score) FROM scores sc WHERE sc.match_id = m.match_id AND sc.k = m.k AND NOT sc.team1)
    FROM maps m
    WHERE m.finished AND m.map_id IS NOT NULL
)
INSERT INTO match_map_scores (match_id, position, user_id, score, mods, passed)
SELECT sc.match_id, sc.k, sc.user_id, sc.score, sc.mods, true
FROM scores sc
JOIN maps m ON m.match_id = sc.match_id AND m.k = sc.k
WHERE m.map_id IS NOT NULL;

-- The saved map results of every concluded seeded bracket match, as the referee page
-- would have saved them with the score: maps from its stage's pool in W1's pick order,
-- with the tiebreaker for a 5-4, the losing team taking the even maps up to its score.
-- Each player's score is made up like W1's chat (so W1's agree with it), and their
-- mods are the category's plus NoFail, or their own on FM and TB.
WITH done AS (
    SELECT b.match_id, b.stage_id, b.team1_id, b.team2_id,
           b.team1_score > b.team2_score AS team1_won,
           LEAST(b.team1_score, b.team2_score) AS lost_maps,
           b.team1_score + b.team2_score AS played
    FROM bracket_matches b
    JOIN matches m ON m.id = b.match_id
    WHERE m.concluded_at IS NOT NULL AND b.forfeit_slot IS NULL
      AND b.team1_score <> b.team2_score
      AND NOT EXISTS (SELECT 1 FROM match_map_results r WHERE r.match_id = b.match_id)
      AND NOT EXISTS (
          SELECT 1 FROM team_members tm JOIN osu_users u ON u.id = tm.user_id
          WHERE tm.team_id IN (b.team1_id, b.team2_id) AND u.osu_id >= 0
      )
),
picks (k, category, slot) AS (
    VALUES (1, 'NM', 0), (2, 'HD', 0), (3, 'HR', 0), (4, 'DT', 0),
           (5, 'NM', 1), (6, 'FM', 0), (7, 'DT', 1), (8, 'HR', 1), (9, 'TB', 0)
),
maps AS (
    SELECT d.match_id, pk.category, pm.id AS map_id,
           row_number() OVER (PARTITION BY d.match_id ORDER BY pk.k)::int AS position,
           -- The winner's maps are the odd ones and every one after the loser's last.
           (pk.k % 2 = 1 OR pk.k > 2 * d.lost_maps) = d.team1_won AS team1_wins
    FROM done d
    JOIN picks pk ON pk.k <= d.played
    JOIN stage_categories c ON c.stage_id = d.stage_id AND c.name = pk.category
    JOIN category_slots s ON s.category_id = c.id AND s.position = pk.slot
    JOIN pool_maps pm ON pm.slot_id = s.id
),
players AS (
    SELECT d.match_id, u.id AS user_id, tm.team_id = d.team1_id AS team1,
           row_number() OVER (PARTITION BY d.match_id
                              ORDER BY tm.team_id = d.team1_id DESC, u.username) AS slot
    FROM done d
    JOIN team_members tm ON tm.team_id IN (d.team1_id, d.team2_id)
    JOIN osu_users u ON u.id = tm.user_id
),
scores AS (
    SELECT m.match_id, m.position, p.user_id, p.team1,
           CASE WHEN p.team1 = m.team1_wins THEN 700000 ELSE 450000 END
               + p.slot * 13579 + m.position * 2468 AS score,
           CASE m.category
               WHEN 'NM' THEN 'NF' WHEN 'HD' THEN 'HDNF' WHEN 'HR' THEN 'HRNF' WHEN 'DT' THEN 'DTNF'
               WHEN 'FM' THEN (ARRAY['HD', 'HR', '', 'HDHR'])[p.slot]
               ELSE (ARRAY['HR', 'HDHR', 'HD', ''])[p.slot]
           END AS mods
    FROM maps m
    JOIN players p ON p.match_id = m.match_id
),
results AS (
    INSERT INTO match_map_results (match_id, position, pool_map_id, team1_score, team2_score)
    SELECT m.match_id, m.position, m.map_id,
           (SELECT sum(sc.score) FROM scores sc
            WHERE sc.match_id = m.match_id AND sc.position = m.position AND sc.team1),
           (SELECT sum(sc.score) FROM scores sc
            WHERE sc.match_id = m.match_id AND sc.position = m.position AND NOT sc.team1)
    FROM maps m
    RETURNING match_id
)
INSERT INTO match_map_scores (match_id, position, user_id, score, mods, passed)
SELECT match_id, position, user_id, score, mods, true
FROM scores;

-- Proposed times for the first two upcoming seeded matches with both teams and no room
-- yet: team 1 offers two later times, team 2 one, each by the team's first player.
INSERT INTO match_reschedules (match_id, team_id, proposed_by, starts_at, created_at)
SELECT up.match_id, t.team_id,
       (SELECT tm.user_id FROM team_members tm WHERE tm.team_id = t.team_id
        ORDER BY tm.joined_at, tm.user_id LIMIT 1),
       up.starts_at + t.shift, now() - interval '1 hour'
FROM (
    SELECT b.match_id, b.team1_id, b.team2_id, m.starts_at
    FROM bracket_matches b
    JOIN matches m ON m.id = b.match_id
    WHERE m.concluded_at IS NULL AND m.starts_at > now()
      AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
      AND NOT EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
      AND EXISTS (
          SELECT 1 FROM team_members tm JOIN osu_users u ON u.id = tm.user_id
          WHERE tm.team_id = b.team1_id AND u.osu_id < 0
      )
    ORDER BY m.starts_at
    LIMIT 2
) AS up
CROSS JOIN LATERAL (VALUES (up.team1_id, interval '1 day'),
                           (up.team1_id, interval '1 day 2 hours'),
                           (up.team2_id, interval '2 days')) AS t (team_id, shift);

-- The notices those proposals would have sent: one unread reschedule_proposed per
-- player of the team on the receiving end, per match.
INSERT INTO notifications (user_id, kind, match_id, team_id, actor_id, created_at)
SELECT DISTINCT ON (tm.user_id, r.match_id)
       tm.user_id, 'reschedule_proposed', r.match_id, tm.team_id, r.proposed_by, r.created_at
FROM match_reschedules r
JOIN bracket_matches b ON b.match_id = r.match_id
JOIN team_members tm
  ON tm.team_id = CASE WHEN r.team_id = b.team1_id THEN b.team2_id ELSE b.team1_id END
JOIN osu_users u ON u.id = tm.user_id
WHERE u.osu_id < 0
ORDER BY tm.user_id, r.match_id, r.created_at;

-- Rank Uçtu going over the limit when Kaymak climbed from #9,000 (18,894 with Bal)
-- to #2,400, told to its players and the hosts. 21,203 is team_rank::team_value.
INSERT INTO notifications (user_id, kind, team_id, actor_id, old_rank, new_rank, team_value,
                           created_at)
SELECT r.id, 'rank_over_limit', t.id, k.id, 9000, 2400, 21203.13, now() - interval '3 hours'
FROM teams t
JOIN osu_users k ON k.username = 'Kaymak' AND k.osu_id < 0
CROSS JOIN LATERAL (
    SELECT user_id AS id FROM team_members WHERE team_id = t.id
    UNION
    SELECT id FROM osu_users WHERE 'host' = ANY(roles)
) AS r
WHERE t.name = 'Rank Uçtu';

-- The concluded qualifier lobby's chat: its seven teams in one Head to Head room, every
-- map of the qualifier pool played in order, scores spread so the teams rank differently
-- from map to map, a few fails. Simitçiler came in late (see its referee notes), so the
-- first map was played without them and then again with everyone.
WITH lobby AS (
    SELECT m.starts_at AS t0, r.mp_id, COALESCE(ref.username, 'BanchoBot') AS referee
    FROM match_rooms r
    JOIN matches m ON m.id = r.match_id
    LEFT JOIN osu_users ref ON ref.id = m.referee_id
    WHERE r.mp_id = 112233441 AND m.kind = 'qualifier'
),
players AS (
    SELECT u.id AS user_id, u.username, u.osu_id, t.name = 'Simitçiler' AS late,
           row_number() OVER (ORDER BY e.joined_at, u.username) AS slot
    FROM lobby
    JOIN match_rooms r ON r.mp_id = lobby.mp_id
    JOIN qualifier_entries e ON e.match_id = r.match_id
    JOIN teams t ON t.id = e.team_id
    JOIN team_members tm ON tm.team_id = t.id
    JOIN osu_users u ON u.id = tm.user_id
),
pool AS (
    SELECT row_number() OVER (ORDER BY c.position, s.position) AS n, c.name AS category,
           pm.id AS map_id, pm.beatmap_id,
           pm.artist || ' - ' || pm.title || ' [' || pm.version || ']' AS name
    FROM stages st
    JOIN stage_categories c ON c.stage_id = st.id
    JOIN category_slots s ON s.category_id = c.id
    JOIN pool_maps pm ON pm.slot_id = s.id
    WHERE st.is_qualifier
),
-- Play k = 0 is the first map without Simitçiler; then every map in pool order.
plays AS (
    SELECT 0 AS k, p.* FROM pool p WHERE p.n = 1
    UNION ALL
    SELECT p.n AS k, p.* FROM pool p
),
timed AS (
    SELECT pl.*, interval '3 minutes' + pl.k * interval '7 minutes' AS at FROM plays pl
),
scores AS (
    SELECT t.k, p.username, p.slot,
           -- Slightly better players up front, then a per-map swing that reshuffles them.
           (CASE WHEN f.failed THEN 180000 ELSE 420000 END
               + (15 - p.slot) * 5000
               + abs(hashtext(p.username || ':' || t.k)) % 300000)::int AS score,
           f.failed
    FROM timed t
    JOIN players p ON NOT (t.k = 0 AND p.late)
    CROSS JOIN LATERAL (SELECT (p.slot * 3 + t.k * 5) % 23 = 0 AS failed) f
),
-- The lobby's saved maps, as its room's bot saved them from this chat (every attempt, the
-- first map's included), with the accuracy osu!'s record of the room gave each score.
results AS (
    INSERT INTO match_map_results (match_id, position, pool_map_id)
    SELECT r.match_id, t.k + 1, t.map_id
    FROM lobby JOIN match_rooms r ON r.mp_id = lobby.mp_id
    CROSS JOIN timed t
),
saved AS (
    INSERT INTO match_map_scores (match_id, position, user_id, score, mods, passed, accuracy)
    SELECT r.match_id, sc.k + 1, p.user_id, sc.score,
           CASE t.category WHEN 'HD' THEN 'HDNF' WHEN 'HR' THEN 'HRNF' WHEN 'DT' THEN 'DTNF' ELSE 'NF' END,
           NOT sc.failed,
           round((0.86 + sc.score / 1000000.0 * 0.12
                  + abs(hashtext(sc.username || '#' || sc.k)) % 150 / 10000.0)::numeric, 4)::float8
    FROM lobby JOIN match_rooms r ON r.mp_id = lobby.mp_id
    CROSS JOIN scores sc
    JOIN timed t ON t.k = sc.k
    JOIN players p ON p.username = sc.username
),
lines (at, sender, text) AS (
    SELECT p.slot * interval '8 seconds' + CASE WHEN p.late THEN interval '9 minutes' ELSE interval '0' END,
           'BanchoBot', p.username || ' joined in slot ' || p.slot || '.'
    FROM players p
    UNION ALL
    SELECT interval '2 minutes 30 seconds', lobby.referee,
           'Simitçiler henüz gelmedi, ilk map onlarsız oynanıyor; gelince tekrarlanacak.'
    FROM lobby
    UNION ALL
    SELECT t.at - interval '10 seconds', lobby.referee, '!mp map ' || t.beatmap_id || ' 0'
    FROM timed t, lobby
    UNION ALL
    SELECT t.at, 'BanchoBot', 'Changed beatmap to https://osu.ppy.sh/b/' || t.beatmap_id || ' ' || t.name
    FROM timed t
    UNION ALL
    SELECT t.at + interval '6 seconds', 'BanchoBot',
           CASE t.category
               WHEN 'HD' THEN 'Enabled Hidden, NoFail, disabled FreeMod'
               WHEN 'HR' THEN 'Enabled HardRock, NoFail, disabled FreeMod'
               WHEN 'DT' THEN 'Enabled DoubleTime, NoFail, disabled FreeMod'
               ELSE 'Enabled NoFail, disabled FreeMod'
           END
    FROM timed t
    UNION ALL
    SELECT t.at + interval '30 seconds', 'BanchoBot', 'The match has started!'
    FROM timed t
    UNION ALL
    SELECT t.at + interval '4 minutes' + sc.slot * interval '1 second', 'BanchoBot',
           sc.username || ' finished playing (Score: ' || sc.score || ', '
               || CASE WHEN sc.failed THEN 'FAILED' ELSE 'PASSED' END || ').'
    FROM timed t JOIN scores sc ON sc.k = t.k
    UNION ALL
    SELECT t.at + interval '4 minutes 20 seconds', 'BanchoBot', 'The match has finished!'
    FROM timed t
    UNION ALL
    SELECT interval '81 minutes' + p.slot * interval '2 seconds', p.username, 'gg'
    FROM players p
)
INSERT INTO room_chat_messages (mp_id, sent_at, sender, text)
SELECT lobby.mp_id, lobby.t0 + l.at, l.sender, l.text
FROM lobby, lines l
ORDER BY l.at;

-- The other seeded lobbies' saved results (they have no chat): every map of the
-- qualifier pool once, scored like the first lobby's, except that Rank Uçtu missed the
-- pool's last two maps, so the statistics count them as a 0 for its seeding.
WITH lobbies AS (
    SELECT m.id AS match_id
    FROM matches m
    JOIN match_rooms r ON r.match_id = m.id
    WHERE m.kind = 'qualifier' AND r.mp_id IN (112233442, 112233443, 112233444)
      AND NOT EXISTS (SELECT 1 FROM match_map_results q WHERE q.match_id = m.id)
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
       (420000 + abs(hashtext(u.username || ':' || p.n)) % 300000)::int,
       CASE p.category WHEN 'HD' THEN 'HDNF' WHEN 'HR' THEN 'HRNF' WHEN 'DT' THEN 'DTNF' ELSE 'NF' END,
       true
FROM lobbies l
CROSS JOIN pool p
JOIN qualifier_entries e ON e.match_id = l.match_id
JOIN teams t ON t.id = e.team_id
JOIN team_members tm ON tm.team_id = t.id
JOIN osu_users u ON u.id = tm.user_id
WHERE NOT (t.name = 'Rank Uçtu' AND p.n > (SELECT max(n) FROM pool) - 2);

-- The accuracy of every other seeded saved score, as the bots would have read it from
-- osu!'s record of the room after each map.
UPDATE match_map_scores s
SET accuracy = round((0.86 + LEAST(s.score, 1000000) / 1000000.0 * 0.12
                      + abs(hashtext(u.username || '#' || s.position)) % 150 / 10000.0)::numeric, 4)::float8
FROM osu_users u
WHERE u.id = s.user_id AND u.osu_id < 0 AND s.accuracy IS NULL;

-- Pick'em. The Semifinals' pick'em is opened by hand (the Quarterfinals are still being
-- played, so the rule alone would keep it waiting), so its ballot can be tried; the
-- stages before it are locked, with the split of their predictions public and the
-- played matches scored.
UPDATE stages SET pickem_mode = 'open' WHERE name = 'Semifinals' AND NOT is_qualifier;

-- 45 dummies predict every seeded match of the first four bracket stages that has both
-- teams (the Semifinals only where nothing has started, and about 60% of them there),
-- and every potential match there: those only count if the match turns out to be that
-- pairing (the site moves them to it then).
-- Each has a skill: how often they pick the winner of a match that was then played,
-- 45-84%. A played match's exact score is hit 30% of the time; otherwise the score
-- leans close (the loser one map short most often). Unplayed matches lean 62% to the
-- better seed, team 1. Predictions are made before the stage's first match.
WITH pickers AS (
    SELECT u.id, 45 + abs(hashtext(u.username)) % 40 AS skill
    FROM osu_users u
    WHERE u.osu_id BETWEEN -240 AND -201 OR u.osu_id IN (-1, -3, -5, -101, -103)
),
games AS (
    SELECT m.id, st.name AS stage, (st.best_of + 1) / 2 AS need, b.team1_id, b.team2_id,
           m.concluded_at IS NOT NULL AS done,
           CASE WHEN b.forfeit_slot = 1 OR (b.forfeit_slot IS NULL AND b.team2_score > b.team1_score)
                THEN b.team2_id ELSE b.team1_id END AS winner,
           CASE WHEN b.forfeit_slot = 1 OR (b.forfeit_slot IS NULL AND b.team2_score > b.team1_score)
                THEN b.team1_id ELSE b.team2_id END AS loser,
           CASE WHEN b.forfeit_slot IS NULL AND greatest(b.team1_score, b.team2_score) = (st.best_of + 1) / 2
                THEN least(b.team1_score, b.team2_score) END AS lost_by,
           (SELECT min(m2.starts_at) FROM bracket_matches b2 JOIN matches m2 ON m2.id = b2.match_id
            WHERE b2.stage_id = st.id) AS first_start
    FROM bracket_matches b
    JOIN matches m ON m.id = b.match_id
    JOIN stages st ON st.id = b.stage_id
    WHERE st.name IN ('Round of 32', 'Round of 16', 'Quarterfinals', 'Semifinals')
      AND st.best_of IS NOT NULL
      AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
      AND EXISTS (
          SELECT 1 FROM team_members tm JOIN osu_users u ON u.id = tm.user_id
          WHERE tm.team_id = b.team1_id AND u.osu_id < 0
      )
      AND (st.name <> 'Semifinals' OR (
          m.concluded_at IS NULL AND m.starts_at > now()
          AND NOT EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
      ))
    UNION ALL
    SELECT p.match_id, st.name, (st.best_of + 1) / 2, p.team1_id, p.team2_id, false,
           NULL, NULL, NULL,
           (SELECT min(m2.starts_at) FROM bracket_matches b2 JOIN matches m2 ON m2.id = b2.match_id
            WHERE b2.stage_id = st.id)
    FROM potential_matches p
    JOIN bracket_matches b ON b.match_id = p.bracket_match_id
    JOIN stages st ON st.id = b.stage_id
    WHERE st.name IN ('Round of 32', 'Round of 16', 'Quarterfinals', 'Semifinals')
      AND st.best_of IS NOT NULL
      AND EXISTS (
          SELECT 1 FROM team_members tm JOIN osu_users u ON u.id = tm.user_id
          WHERE tm.team_id = p.team1_id AND u.osu_id < 0
      )
),
rolls AS (
    SELECT p.id AS user_id, p.skill, g.*,
           abs(hashtext(p.id::text || g.id::text)) % 100 AS r1,
           abs(hashtext(g.id::text || p.id::text)) % 100 AS r2,
           abs(hashtext(p.id::text || 'score' || g.id::text)) % 20 AS r3
    FROM pickers p
    CROSS JOIN games g
)
INSERT INTO pickem_predictions (user_id, match_id, team_id, winner_score, loser_score,
                                created_at, updated_at)
SELECT user_id, id,
       CASE WHEN done THEN CASE WHEN r1 < skill THEN winner ELSE loser END
            ELSE CASE WHEN r1 < 62 THEN team1_id ELSE team2_id END END,
       need,
       CASE WHEN done AND lost_by IS NOT NULL AND r2 < 30 THEN lost_by
            ELSE greatest(0, need - 1 - CASE WHEN r3 < 4 THEN 0 WHEN r3 < 11 THEN 1
                                             WHEN r3 < 16 THEN 2 WHEN r3 < 19 THEN 3 ELSE 4 END)
       END,
       least(first_start, now()) - interval '2 hours' - r1 * interval '7 minutes',
       least(first_start, now()) - interval '2 hours' - r1 * interval '7 minutes'
FROM rolls
WHERE stage <> 'Semifinals' OR r2 % 10 < 6;
