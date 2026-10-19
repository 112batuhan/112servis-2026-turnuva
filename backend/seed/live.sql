-- Live test, run after seed.sql: the signed-in real account (`dev.live_user`, set by
-- db/dev.rs) teamed with a dummy, against a dummy team, in a bracket match that starts
-- now and has no room yet, so a referee can make a real osu! room for it and the real
-- player can join it from the game. DevReferee referees it and it's played in the
-- Quarterfinals' pool, the stage being played. The dummies make clear.sql remove all
-- of it, freeing the real account.
DO $$
DECLARE
    me uuid := current_setting('dev.live_user')::uuid;
    home uuid;
    away uuid;
    new_match uuid;
BEGIN
    INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank, roles) VALUES
        (-301, 'Kanka', 'https://a.ppy.sh/', 'TR', 90000, 780, '{}'),
        (-302, 'Rakip', 'https://a.ppy.sh/', 'TR', 30000, 280, '{}'),
        (-303, 'Hasim', 'https://a.ppy.sh/', 'TR', 60000, 530, '{}');
    INSERT INTO discord_accounts (osu_user_id, discord_id, username)
    SELECT id, 'dummy' || osu_id, lower(username) FROM osu_users WHERE osu_id IN (-301, -302, -303);

    INSERT INTO teams (name) VALUES ('Canlı Test') RETURNING id INTO home;
    INSERT INTO team_members (user_id, team_id)
    SELECT id, home FROM osu_users WHERE id = me OR osu_id = -301;
    INSERT INTO teams (name) VALUES ('Rakip Takım') RETURNING id INTO away;
    INSERT INTO team_members (user_id, team_id)
    SELECT id, away FROM osu_users WHERE osu_id IN (-302, -303);

    INSERT INTO matches (kind, starts_at, referee_id)
    VALUES ('bracket', date_trunc('minute', now()), (SELECT id FROM osu_users WHERE osu_id = -3))
    RETURNING id INTO new_match;
    -- The real account's team plays blue (team 1, slots 1-2). Placed right of the bracket.
    INSERT INTO bracket_matches (match_id, side, number, pos_x, pos_y, team1_id, team2_id, stage_id)
    VALUES (
        new_match,
        'winners',
        (SELECT COALESCE(max(number), 0) + 1 FROM bracket_matches WHERE side = 'winners'),
        (SELECT COALESCE(max(pos_x), 0) + 15 FROM bracket_matches),
        (SELECT COALESCE(min(pos_y), 0) FROM bracket_matches),
        home,
        away,
        (SELECT id FROM stages WHERE name = 'Quarterfinals' ORDER BY position LIMIT 1)
    );
END $$;
