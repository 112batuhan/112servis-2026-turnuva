use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{matches::RefereeColumns, team::Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "bracket_side", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Winners,
    Losers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "bracket_outcome", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Winner,
    Loser,
}

// A bracket match with its referee and streamer folded in. The streamer_* fields are
// null when nobody has signed up to stream it.
#[derive(Debug, sqlx::FromRow)]
pub struct BracketRow {
    pub id: Uuid,
    pub side: Side,
    pub number: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub starts_at: Option<OffsetDateTime>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    // The slot of the team that forfeited, when the match was decided that way.
    pub forfeit_slot: Option<i16>,
    // Its rooms, oldest first.
    pub mp_ids: Vec<i64>,
    pub referee_notes: String,
    pub concluded_at: Option<OffsetDateTime>,
    #[sqlx(flatten)]
    pub referee: RefereeColumns,
    pub streamer_id: Option<Uuid>,
    pub streamer_osu_id: Option<i64>,
    pub streamer_username: Option<String>,
    pub streamer_avatar_url: Option<String>,
    pub stage_id: Option<Uuid>,
    pub stage_name: Option<String>,
    pub stage_abbreviation: Option<String>,
    pub stage_published: Option<bool>,
    pub stage_position: Option<i32>,
    pub stage_best_of: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct LinkRow {
    pub source_id: Uuid,
    pub outcome: Outcome,
    pub target_id: Uuid,
    pub slot: i16,
}

// A bracket match as its edits and its conclusion read it, under a row lock.
#[derive(Debug, sqlx::FromRow)]
pub struct BracketLock {
    pub starts_at: Option<OffsetDateTime>,
    pub referee_id: Option<Uuid>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub forfeit_slot: Option<i16>,
    pub streamer_id: Option<Uuid>,
    pub has_room: bool,
    pub concluded_at: Option<OffsetDateTime>,
}

impl BracketLock {
    pub fn concluded(&self) -> bool {
        self.concluded_at.is_some()
    }
}

// Where a match's winner or loser goes, with the target's state, locked so the
// target can't be concluded while a team is moved into it.
#[derive(Debug, sqlx::FromRow)]
pub struct TargetLock {
    pub outcome: Outcome,
    pub target_id: Uuid,
    pub slot: i16,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub concluded_at: Option<OffsetDateTime>,
}

// Every bracket match, in grid order.
pub async fn list_matches(pool: &PgPool) -> sqlx::Result<Vec<BracketRow>> {
    sqlx::query_as::<_, BracketRow>(
        r#"
        SELECT m.id, b.side, b.number, b.pos_x, b.pos_y, m.starts_at,
               b.team1_id, b.team2_id, b.team1_score, b.team2_score, b.forfeit_slot,
               ARRAY(SELECT r.mp_id FROM match_rooms r WHERE r.match_id = m.id
                     ORDER BY r.added_at, r.mp_id) AS mp_ids,
               m.referee_notes, m.concluded_at,
               u.id AS referee_id, u.osu_id AS referee_osu_id,
               u.username AS referee_username, u.avatar_url AS referee_avatar_url,
               d.discord_id AS referee_discord_id, d.username AS referee_discord_username,
               s.id AS streamer_id, s.osu_id AS streamer_osu_id,
               s.username AS streamer_username, s.avatar_url AS streamer_avatar_url,
               st.id AS stage_id, st.name AS stage_name, st.abbreviation AS stage_abbreviation,
               st.published AS stage_published, st.position AS stage_position,
               st.best_of AS stage_best_of
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        LEFT JOIN osu_users u ON u.id = m.referee_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        LEFT JOIN osu_users s ON s.id = m.streamer_id
        LEFT JOIN stages st ON st.id = b.stage_id
        ORDER BY b.pos_x, b.pos_y
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_links(ex: impl PgExecutor<'_>) -> sqlx::Result<Vec<LinkRow>> {
    sqlx::query_as::<_, LinkRow>("SELECT source_id, outcome, target_id, slot FROM bracket_links")
        .fetch_all(ex)
        .await
}

// Creates a match numbered after the last one on its side.
pub async fn create_match(pool: &PgPool, side: Side, pos_x: i32, pos_y: i32) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        r#"
        WITH m AS (INSERT INTO matches (kind) VALUES ('bracket') RETURNING id)
        INSERT INTO bracket_matches (match_id, side, number, pos_x, pos_y)
        SELECT m.id, $1,
               COALESCE((SELECT max(number) FROM bracket_matches WHERE side = $1), 0) + 1,
               $2, $3
        FROM m
        RETURNING match_id
        "#,
    )
    .bind(side)
    .bind(pos_x)
    .bind(pos_y)
    .fetch_one(pool)
    .await
}

pub async fn delete_match(ex: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM matches WHERE id = $1 AND kind = 'bracket'")
        .bind(id)
        .execute(ex)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn find_match(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<BracketLock>> {
    sqlx::query_as::<_, BracketLock>(
        r#"
        SELECT m.starts_at, m.referee_id, b.team1_id, b.team2_id, b.team1_score, b.team2_score,
               b.forfeit_slot, m.streamer_id,
               EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id) AS has_room,
               m.concluded_at
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        WHERE b.match_id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

// find_match under a row lock, for edits and conclusions.
pub async fn lock_match(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<Option<BracketLock>> {
    sqlx::query_as::<_, BracketLock>(
        r#"
        SELECT m.starts_at, m.referee_id, b.team1_id, b.team2_id, b.team1_score, b.team2_score,
               b.forfeit_slot, m.streamer_id,
               EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id) AS has_room,
               m.concluded_at
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        WHERE b.match_id = $1
        FOR UPDATE OF b, m
        "#,
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn lock_targets(
    conn: &mut PgConnection,
    source_id: Uuid,
) -> sqlx::Result<Vec<TargetLock>> {
    sqlx::query_as::<_, TargetLock>(
        r#"
        SELECT l.outcome, l.target_id, l.slot, b.team1_id, b.team2_id, m.concluded_at
        FROM bracket_links l
        JOIN bracket_matches b ON b.match_id = l.target_id
        JOIN matches m ON m.id = l.target_id
        WHERE l.source_id = $1
        ORDER BY l.target_id
        FOR UPDATE OF b, m
        "#,
    )
    .bind(source_id)
    .fetch_all(conn)
    .await
}

// The fields a host edits together in the match editor.
pub struct MatchUpdate {
    pub side: Side,
    pub number: i32,
    pub starts_at: Option<OffsetDateTime>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub stage_id: Option<Uuid>,
}

pub async fn update_match(conn: &mut PgConnection, id: Uuid, u: &MatchUpdate) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE bracket_matches
        SET side = $2, number = $3, team1_id = $4, team2_id = $5, stage_id = $6
        WHERE match_id = $1
        "#,
    )
    .bind(id)
    .bind(u.side)
    .bind(u.number)
    .bind(u.team1_id)
    .bind(u.team2_id)
    .bind(u.stage_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE matches SET starts_at = $2 WHERE id = $1")
        .bind(id)
        .bind(u.starts_at)
        .execute(conn)
        .await?;
    Ok(())
}

// Replaces where a match's winner and loser go.
pub async fn set_links(
    conn: &mut PgConnection,
    source_id: Uuid,
    links: &[LinkRow],
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM bracket_links WHERE source_id = $1")
        .bind(source_id)
        .execute(&mut *conn)
        .await?;
    for link in links {
        sqlx::query(
            "INSERT INTO bracket_links (source_id, outcome, target_id, slot) VALUES ($1, $2, $3, $4)",
        )
        .bind(source_id)
        .bind(link.outcome)
        .bind(link.target_id)
        .bind(link.slot)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

// Moves a match on the editor's board. False when there's no such match.
pub async fn move_match(pool: &PgPool, id: Uuid, pos_x: i32, pos_y: i32) -> sqlx::Result<bool> {
    let result =
        sqlx::query("UPDATE bracket_matches SET pos_x = $2, pos_y = $3 WHERE match_id = $1")
            .bind(id)
            .bind(pos_x)
            .bind(pos_y)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

// Sets the score of a match that isn't concluded. False when it is.
pub async fn set_score(
    ex: impl PgExecutor<'_>,
    id: Uuid,
    team1_score: Option<i32>,
    team2_score: Option<i32>,
) -> sqlx::Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE bracket_matches b SET team1_score = $2, team2_score = $3
        FROM matches m
        WHERE b.match_id = $1 AND m.id = b.match_id AND m.concluded_at IS NULL
        "#,
    )
    .bind(id)
    .bind(team1_score)
    .bind(team2_score)
    .execute(ex)
    .await?;
    Ok(result.rows_affected() > 0)
}

// Records which slot forfeited (or clears it), inside a conclusion or a reopening.
pub async fn set_forfeit(
    conn: &mut PgConnection,
    id: Uuid,
    forfeit_slot: Option<i16>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE bracket_matches SET forfeit_slot = $2 WHERE match_id = $1")
        .bind(id)
        .bind(forfeit_slot)
        .execute(conn)
        .await?;
    Ok(())
}

// Puts a team into slot 1 or 2 of a match.
pub async fn set_slot(
    conn: &mut PgConnection,
    id: Uuid,
    slot: i16,
    team_id: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE bracket_matches
        SET team1_id = CASE WHEN $2 = 1 THEN $3 ELSE team1_id END,
            team2_id = CASE WHEN $2 = 2 THEN $3 ELSE team2_id END
        WHERE match_id = $1
        "#,
    )
    .bind(id)
    .bind(slot)
    .bind(team_id)
    .execute(conn)
    .await?;
    Ok(())
}

// Everyone who can stream: the users holding the streamer role.
pub async fn list_streamers(pool: &PgPool) -> sqlx::Result<Vec<Player>> {
    sqlx::query_as::<_, Player>(
        r#"
        SELECT id, osu_id, username, avatar_url, country_code, global_rank
        FROM osu_users
        WHERE 'streamer' = ANY(roles)
        ORDER BY lower(username)
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn can_stream(pool: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM osu_users WHERE id = $1 AND 'streamer' = ANY(roles))",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BanRow {
    pub map_id: Uuid,
    pub team_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

// A match's bans, in the order they were made.
pub async fn list_bans(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<BanRow>> {
    sqlx::query_as::<_, BanRow>(
        r#"
        SELECT pool_map_id AS map_id, team_id, created_at
        FROM match_bans
        WHERE match_id = $1
        ORDER BY created_at, pool_map_id
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

// Whether a map is placed in the pool of the stage the match is played in.
pub async fn map_in_pool(pool: &PgPool, match_id: Uuid, map_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM bracket_matches b
            JOIN stage_categories c ON c.stage_id = b.stage_id
            JOIN category_slots s ON s.category_id = c.id
            JOIN pool_maps pm ON pm.slot_id = s.id
            WHERE b.match_id = $1 AND pm.id = $2
        )
        "#,
    )
    .bind(match_id)
    .bind(map_id)
    .fetch_one(pool)
    .await
}

// Bans a map for a team, or hands an existing ban to the other team. False when the
// map has been picked.
pub async fn set_ban(
    pool: &PgPool,
    match_id: Uuid,
    map_id: Uuid,
    team_id: Uuid,
) -> sqlx::Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO match_bans (match_id, pool_map_id, team_id)
        SELECT $1, $2, $3
        WHERE NOT EXISTS (
            SELECT 1 FROM match_picks WHERE match_id = $1 AND pool_map_id = $2
        )
        ON CONFLICT (match_id, pool_map_id) DO UPDATE SET team_id = EXCLUDED.team_id
        "#,
    )
    .bind(match_id)
    .bind(map_id)
    .bind(team_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_ban(pool: &PgPool, match_id: Uuid, map_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM match_bans WHERE match_id = $1 AND pool_map_id = $2")
        .bind(match_id)
        .bind(map_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PickRow {
    pub map_id: Uuid,
    pub team_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

// A match's picks, in the order they were made.
pub async fn list_picks(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<PickRow>> {
    sqlx::query_as::<_, PickRow>(
        r#"
        SELECT pool_map_id AS map_id, team_id, created_at
        FROM match_picks
        WHERE match_id = $1
        ORDER BY created_at, pool_map_id
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

// Records a team's pick, or hands an existing pick to the other team. False when the
// map is banned.
pub async fn set_pick(
    pool: &PgPool,
    match_id: Uuid,
    map_id: Uuid,
    team_id: Uuid,
) -> sqlx::Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO match_picks (match_id, pool_map_id, team_id)
        SELECT $1, $2, $3
        WHERE NOT EXISTS (
            SELECT 1 FROM match_bans WHERE match_id = $1 AND pool_map_id = $2
        )
        ON CONFLICT (match_id, pool_map_id) DO UPDATE SET team_id = EXCLUDED.team_id
        "#,
    )
    .bind(match_id)
    .bind(map_id)
    .bind(team_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_pick(pool: &PgPool, match_id: Uuid, map_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM match_picks WHERE match_id = $1 AND pool_map_id = $2")
        .bind(match_id)
        .bind(map_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// A map win the referee awarded by hand.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ManualResultRow {
    pub id: Uuid,
    // Null for a point given without a map, or once the map left the pool.
    pub map_id: Option<Uuid>,
    pub team_id: Uuid,
    pub note: String,
}

// A match's hand-awarded map wins, in the order they were given.
pub async fn list_manual_results(
    pool: &PgPool,
    match_id: Uuid,
) -> sqlx::Result<Vec<ManualResultRow>> {
    sqlx::query_as::<_, ManualResultRow>(
        r#"
        SELECT id, pool_map_id AS map_id, team_id, note
        FROM match_manual_results
        WHERE match_id = $1
        ORDER BY created_at, id
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

pub async fn add_manual_result(
    pool: &PgPool,
    match_id: Uuid,
    map_id: Option<Uuid>,
    team_id: Uuid,
    note: &str,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        r#"
        INSERT INTO match_manual_results (match_id, pool_map_id, team_id, note)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(match_id)
    .bind(map_id)
    .bind(team_id)
    .bind(note)
    .fetch_one(pool)
    .await
}

pub async fn delete_manual_result(pool: &PgPool, match_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM match_manual_results WHERE match_id = $1 AND id = $2")
        .bind(match_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// A finished pool map that counts towards a match's score, with each team's summed
// score on it and each player's, as the referee page last saved them.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MapResultRow {
    // Always set by the referee page; null once the map was deleted from the site.
    pub map_id: Option<Uuid>,
    pub team1_score: i64,
    pub team2_score: i64,
    #[sqlx(skip)]
    #[serde(default)]
    pub players: Vec<MapPlayerScore>,
}

// A player's score on a map result, with the mods they played it with.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MapPlayerScore {
    pub user_id: Uuid,
    pub score: i64,
    // Acronyms, the room's included ("HDNF"); null when they weren't read.
    pub mods: Option<String>,
    pub passed: bool,
}

#[derive(sqlx::FromRow)]
struct PositionedScore {
    position: i32,
    #[sqlx(flatten)]
    score: MapPlayerScore,
}

// A match's map results, in the order the maps were played, each with its players'
// scores, best first.
pub async fn list_map_results(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<MapResultRow>> {
    let mut rows = sqlx::query_as::<_, MapResultRow>(
        r#"
        SELECT pool_map_id AS map_id, team1_score, team2_score
        FROM match_map_results
        WHERE match_id = $1
        ORDER BY position
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await?;
    let scores = sqlx::query_as::<_, PositionedScore>(
        r#"
        SELECT position, user_id, score, mods, passed
        FROM match_map_scores
        WHERE match_id = $1
        ORDER BY position, score DESC, user_id
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await?;
    // Positions run from 1, in the order replace_map_results wrote them.
    for s in scores {
        if let Some(row) = usize::try_from(s.position - 1)
            .ok()
            .and_then(|i| rows.get_mut(i))
        {
            row.players.push(s.score);
        }
    }
    Ok(rows)
}

// Replaces a bracket match's map results and their players' scores with `rows`, in
// their order.
pub async fn replace_map_results(
    conn: &mut PgConnection,
    match_id: Uuid,
    rows: &[MapResultRow],
) -> sqlx::Result<()> {
    let rows: Vec<SavedMap> = rows
        .iter()
        .map(|r| SavedMap {
            map_id: r.map_id,
            totals: Some((r.team1_score, r.team2_score)),
            players: &r.players,
        })
        .collect();
    replace_saved_maps(conn, match_id, &rows).await
}

// A finished pool map as a match saves it: a bracket match's with each team's total, a
// qualifier lobby's without.
pub struct SavedMap<'a> {
    pub map_id: Option<Uuid>,
    pub totals: Option<(i64, i64)>,
    pub players: &'a [MapPlayerScore],
}

#[derive(sqlx::FromRow)]
struct KnownAccuracy {
    user_id: Uuid,
    map_id: Option<Uuid>,
    score: i64,
    accuracy: f64,
}

// Replaces any match's saved maps and their players' scores with `rows`, in their order.
// The bots save them again from the chat as each map ends, and the chat has no accuracy:
// a score that stays the same (player, map and score) keeps the accuracy osu!'s record
// gave it (crate::room_scores).
pub async fn replace_saved_maps(
    conn: &mut PgConnection,
    match_id: Uuid,
    rows: &[SavedMap<'_>],
) -> sqlx::Result<()> {
    let known = sqlx::query_as::<_, KnownAccuracy>(
        r#"
        SELECT s.user_id, r.pool_map_id AS map_id, s.score, s.accuracy
        FROM match_map_scores s
        JOIN match_map_results r USING (match_id, position)
        WHERE s.match_id = $1 AND s.accuracy IS NOT NULL
        "#,
    )
    .bind(match_id)
    .fetch_all(&mut *conn)
    .await?;
    let accuracy_of = |user_id: Uuid, map_id: Option<Uuid>, score: i64| {
        known
            .iter()
            .find(|k| k.user_id == user_id && k.map_id == map_id && k.score == score)
            .map(|k| k.accuracy)
    };

    // The players' scores go with their rows (ON DELETE CASCADE).
    sqlx::query("DELETE FROM match_map_results WHERE match_id = $1")
        .bind(match_id)
        .execute(&mut *conn)
        .await?;
    let maps: Vec<Option<Uuid>> = rows.iter().map(|r| r.map_id).collect();
    let team1: Vec<Option<i64>> = rows.iter().map(|r| r.totals.map(|t| t.0)).collect();
    let team2: Vec<Option<i64>> = rows.iter().map(|r| r.totals.map(|t| t.1)).collect();
    sqlx::query(
        r#"
        INSERT INTO match_map_results (match_id, position, pool_map_id, team1_score, team2_score)
        SELECT $1, r.position, r.map_id, r.team1_score, r.team2_score
        FROM UNNEST($2::uuid[], $3::bigint[], $4::bigint[])
             WITH ORDINALITY AS r(map_id, team1_score, team2_score, position)
        "#,
    )
    .bind(match_id)
    .bind(&maps)
    .bind(&team1)
    .bind(&team2)
    .execute(&mut *conn)
    .await?;

    let mut positions = Vec::new();
    let mut users = Vec::new();
    let mut scores = Vec::new();
    let mut mods = Vec::new();
    let mut passed = Vec::new();
    let mut accuracy = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for p in row.players {
            positions.push(i as i32 + 1);
            users.push(p.user_id);
            scores.push(p.score);
            mods.push(p.mods.clone());
            passed.push(p.passed);
            accuracy.push(accuracy_of(p.user_id, row.map_id, p.score));
        }
    }
    sqlx::query(
        r#"
        INSERT INTO match_map_scores (match_id, position, user_id, score, mods, passed, accuracy)
        SELECT $1, s.position, s.user_id, s.score, s.mods, s.passed, s.accuracy
        FROM UNNEST($2::int[], $3::uuid[], $4::bigint[], $5::text[], $6::bool[], $7::float8[])
             AS s(position, user_id, score, mods, passed, accuracy)
        "#,
    )
    .bind(match_id)
    .bind(&positions)
    .bind(&users)
    .bind(&scores)
    .bind(&mods)
    .bind(&passed)
    .bind(&accuracy)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

// Whether every one of `user_ids` plays for one of the match's two teams.
pub async fn players_in_match(
    ex: impl PgExecutor<'_>,
    match_id: Uuid,
    user_ids: &[Uuid],
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT NOT EXISTS (
            SELECT 1 FROM UNNEST($2::uuid[]) AS wanted(id)
            WHERE NOT EXISTS (
                SELECT 1
                FROM bracket_matches b
                JOIN team_members tm ON tm.team_id IN (b.team1_id, b.team2_id)
                WHERE b.match_id = $1 AND tm.user_id = wanted.id
            )
        )
        "#,
    )
    .bind(match_id)
    .bind(user_ids)
    .fetch_one(ex)
    .await
}

// Whether every one of `map_ids` is placed in the pool of the match's stage.
pub async fn maps_in_pool(
    ex: impl PgExecutor<'_>,
    match_id: Uuid,
    map_ids: &[Uuid],
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT NOT EXISTS (
            SELECT 1 FROM UNNEST($2::uuid[]) AS wanted(id)
            WHERE NOT EXISTS (
                SELECT 1
                FROM bracket_matches b
                JOIN stage_categories c ON c.stage_id = b.stage_id
                JOIN category_slots s ON s.category_id = c.id
                JOIN pool_maps pm ON pm.slot_id = s.id
                WHERE b.match_id = $1 AND pm.id = wanted.id
            )
        )
        "#,
    )
    .bind(match_id)
    .bind(map_ids)
    .fetch_one(ex)
    .await
}
