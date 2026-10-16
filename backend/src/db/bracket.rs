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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
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
    pub stage_published: Option<bool>,
    pub stage_position: Option<i32>,
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
               st.id AS stage_id, st.name AS stage_name, st.published AS stage_published,
               st.position AS stage_position
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        LEFT JOIN osu_users u ON u.id = m.referee_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        LEFT JOIN osu_users s ON s.id = b.streamer_id
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

pub async fn delete_match(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM matches WHERE id = $1 AND kind = 'bracket'")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn find_match(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<BracketLock>> {
    sqlx::query_as::<_, BracketLock>(
        r#"
        SELECT m.starts_at, m.referee_id, b.team1_id, b.team2_id, b.team1_score, b.team2_score,
               b.forfeit_slot, b.streamer_id,
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
               b.forfeit_slot, b.streamer_id,
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

// Takes an unstreamed match. False when someone else got there first.
pub async fn claim_stream(pool: &PgPool, id: Uuid, streamer_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE bracket_matches SET streamer_id = $2 WHERE match_id = $1 AND streamer_id IS NULL",
    )
    .bind(id)
    .bind(streamer_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// Drops the streamer only if it's still `streamer_id`.
pub async fn release_stream(pool: &PgPool, id: Uuid, streamer_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE bracket_matches SET streamer_id = NULL WHERE match_id = $1 AND streamer_id = $2",
    )
    .bind(id)
    .bind(streamer_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_streamer(pool: &PgPool, id: Uuid, streamer_id: Option<Uuid>) -> sqlx::Result<()> {
    sqlx::query("UPDATE bracket_matches SET streamer_id = $2 WHERE match_id = $1")
        .bind(id)
        .bind(streamer_id)
        .execute(pool)
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BanRow {
    pub map_id: Uuid,
    pub team_id: Uuid,
}

// A match's bans, in the order they were made.
pub async fn list_bans(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<BanRow>> {
    sqlx::query_as::<_, BanRow>(
        r#"
        SELECT pool_map_id AS map_id, team_id
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PickRow {
    pub map_id: Uuid,
    pub team_id: Uuid,
}

// A match's picks, in the order they were made.
pub async fn list_picks(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<PickRow>> {
    sqlx::query_as::<_, PickRow>(
        r#"
        SELECT pool_map_id AS map_id, team_id
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
#[derive(Debug, Serialize, sqlx::FromRow)]
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
