-- The seed's score model (seed.sql, qualifiers.sql): temporary functions, gone with
-- the session, made before each of them.

-- How the seed makes up scores, so every saved score, chat line and statistic agrees.
-- Each map has its own centre (380k-820k) and spread (8-30%); each player a skill that
-- holds across maps; each play its own luck, now and then a choke. Scores bend softly
-- under 1M rather than piling up at a cap. seed.sql (and qualifiers.sql after it) drop
-- them again at their end, so run this first each time.
CREATE OR REPLACE FUNCTION pg_temp.seed_uniform(key text) RETURNS float8
LANGUAGE sql IMMUTABLE AS $$ SELECT (abs(hashtext(key)) % 100000) / 100000.0 $$;

-- Roughly normal (mean 0, deviation 1): four uniforms summed.
CREATE OR REPLACE FUNCTION pg_temp.seed_normal(key text) RETURNS float8
LANGUAGE sql IMMUTABLE AS $$
    SELECT (pg_temp.seed_uniform(key || '/a') + pg_temp.seed_uniform(key || '/b')
            + pg_temp.seed_uniform(key || '/c') + pg_temp.seed_uniform(key || '/d') - 2) * sqrt(3)
$$;

-- A player's score on a map (`map_key`) in one play of it (`play_key`).
CREATE OR REPLACE FUNCTION pg_temp.seed_score(player text, map_key text, play_key text) RETURNS int
LANGUAGE sql IMMUTABLE AS $$
    SELECT GREATEST(40000, CASE WHEN raw > 800000
                                THEN 800000 + 199000 * tanh((raw - 800000) / 199000.0)
                                ELSE raw END)::int
    FROM (
        SELECT (380000 + pg_temp.seed_uniform(map_key || ':centre') * 440000)
               * (1 + (0.08 + pg_temp.seed_uniform(map_key || ':spread') * 0.22)
                      * (0.8 * pg_temp.seed_normal(player || ':skill')
                         + 0.6 * pg_temp.seed_normal(player || ':' || map_key || ':' || play_key)))
               * CASE WHEN pg_temp.seed_uniform(player || ':' || map_key || ':' || play_key || ':choke') < 0.05
                      THEN 0.45 + pg_temp.seed_uniform(player || ':' || map_key || ':' || play_key || ':how') * 0.25
                      ELSE 1 END AS raw
    ) x
$$;

-- The same in a bracket match, where the map's result is already decided: when the
-- losing side's made-up total would beat the winner's, its scores are scaled to finish
-- 2-17% short of it.
CREATE OR REPLACE FUNCTION pg_temp.seed_versus_score(player text, team1 bool, team1_names text[],
                                                     team2_names text[], team1_wins bool,
                                                     map_key text, play_key text) RETURNS int
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE WHEN team1 <> team1_wins AND own >= opp
                THEN (pg_temp.seed_score(player, map_key, play_key) * opp / own
                      * (0.83 + pg_temp.seed_uniform(map_key || ':' || play_key || ':margin') * 0.15))::int
                ELSE pg_temp.seed_score(player, map_key, play_key) END
    FROM (
        SELECT (SELECT sum(pg_temp.seed_score(u, map_key, play_key))
                FROM unnest(CASE WHEN team1 THEN team1_names ELSE team2_names END) u)::float8 AS own,
               (SELECT sum(pg_temp.seed_score(u, map_key, play_key))
                FROM unnest(CASE WHEN team1 THEN team2_names ELSE team1_names END) u)::float8 AS opp
    ) t
$$;
