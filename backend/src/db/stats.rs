//! What the statistics are counted from: the saved maps of every match (a qualifier
//! lobby's as well as a bracket match's, in match_map_results and match_map_scores), the
//! accuracy osu!'s record gives each score (the chat has none) and a stage's matches,
//! bans and picks.

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::bracket::{replace_saved_maps, MapPlayerScore, SavedMap, Side};

// A finished pool map of a qualifier lobby, with its players' scores.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QualifierMapRow {
    // Null once the map was deleted from the site.
    pub map_id: Option<Uuid>,
    #[sqlx(skip)]
    pub players: Vec<MapPlayerScore>,
}

#[derive(sqlx::FromRow)]
struct PositionedScore {
    position: i32,
    #[sqlx(flatten)]
    score: MapPlayerScore,
}

// A lobby's saved results, in the order the maps were played.
pub async fn list_qualifier_results(
    pool: &PgPool,
    match_id: Uuid,
) -> sqlx::Result<Vec<QualifierMapRow>> {
    let mut rows = sqlx::query_as::<_, QualifierMapRow>(
        "SELECT pool_map_id AS map_id FROM match_map_results WHERE match_id = $1 ORDER BY position",
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

// Replaces a lobby's saved results with `rows`, in their order (without team totals:
// a lobby's teams don't play each other).
pub async fn replace_qualifier_results(
    conn: &mut PgConnection,
    match_id: Uuid,
    rows: &[QualifierMapRow],
) -> sqlx::Result<()> {
    let rows: Vec<SavedMap> = rows
        .iter()
        .map(|r| SavedMap {
            map_id: r.map_id,
            totals: None,
            players: &r.players,
        })
        .collect();
    replace_saved_maps(conn, match_id, &rows).await
}

// Concluded lobbies whose results were never saved (concluded before the site saved
// them, or seeded with only their chat): the bots don't run them any more, so they're
// read from their saved chat instead.
pub async fn lobbies_without_results(pool: &PgPool) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id
        FROM matches m
        WHERE m.kind = 'qualifier' AND m.concluded_at IS NOT NULL
          AND EXISTS (
              SELECT 1 FROM match_rooms r
              JOIN room_chat_messages c ON c.mp_id = r.mp_id
              WHERE r.match_id = m.id
          )
          AND NOT EXISTS (SELECT 1 FROM match_map_results q WHERE q.match_id = m.id)
        "#,
    )
    .fetch_all(pool)
    .await
}

// The match a room belongs to.
pub async fn match_of_room(pool: &PgPool, mp_id: i64) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT match_id FROM match_rooms WHERE mp_id = $1")
        .bind(mp_id)
        .fetch_optional(pool)
        .await
}

// Rooms of matches with a saved score still without its accuracy, added in the last two
// weeks, whose last read was long enough ago: two minutes after a read that found
// something, then twice as long after each one that found nothing.
pub async fn rooms_missing_accuracy(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        r#"
        SELECT r.mp_id
        FROM match_rooms r
        WHERE r.added_at > now() - interval '14 days'
          AND (r.scores_read_at IS NULL
               OR r.scores_read_at < now() - make_interval(secs => 120 * power(2, LEAST(r.scores_misses, 9))))
          AND EXISTS (
              SELECT 1 FROM match_map_scores s
              WHERE s.match_id = r.match_id AND s.accuracy IS NULL
          )
        ORDER BY r.scores_read_at NULLS FIRST, r.added_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub struct RoomScore {
    pub beatmap_id: i64,
    pub osu_id: i64,
    pub score: i64,
    pub accuracy: f64,
}

// Gives the saved scores of the room's match the accuracy osu!'s record of the room has
// for them, and notes the read; says how many scores it changed. A saved score takes the
// record's score by the same player on the same beatmap with the same score, or, when
// the player has only the one on that beatmap on both sides, that one.
pub async fn apply_room_accuracy(
    pool: &PgPool,
    mp_id: i64,
    scores: &[RoomScore],
) -> sqlx::Result<u64> {
    let mut tx = pool.begin().await?;
    let beatmaps: Vec<i64> = scores.iter().map(|s| s.beatmap_id).collect();
    let users: Vec<i64> = scores.iter().map(|s| s.osu_id).collect();
    let values: Vec<i64> = scores.iter().map(|s| s.score).collect();
    let accuracy: Vec<f64> = scores.iter().map(|s| s.accuracy).collect();
    let changed = sqlx::query(
        r#"
        WITH fetched AS (
            SELECT * FROM UNNEST($2::bigint[], $3::bigint[], $4::bigint[], $5::float8[])
                AS f(beatmap_id, osu_id, score, accuracy)
        ),
        saved AS (
            SELECT s.match_id, s.position, s.user_id, s.score, pm.beatmap_id, u.osu_id
            FROM match_map_scores s
            JOIN match_map_results res USING (match_id, position)
            JOIN pool_maps pm ON pm.id = res.pool_map_id
            JOIN osu_users u ON u.id = s.user_id
            WHERE s.match_id = (SELECT match_id FROM match_rooms WHERE mp_id = $1)
        ),
        pairs AS (
            SELECT DISTINCT ON (sv.match_id, sv.position, sv.user_id)
                   sv.match_id, sv.position, sv.user_id, f.accuracy
            FROM saved sv
            JOIN fetched f ON f.beatmap_id = sv.beatmap_id AND f.osu_id = sv.osu_id
            WHERE f.score = sv.score
               OR ((SELECT count(*) FROM saved o
                    WHERE o.beatmap_id = sv.beatmap_id AND o.osu_id = sv.osu_id) = 1
                   AND (SELECT count(*) FROM fetched o
                        WHERE o.beatmap_id = f.beatmap_id AND o.osu_id = f.osu_id) = 1)
            ORDER BY sv.match_id, sv.position, sv.user_id, (f.score = sv.score) DESC
        )
        UPDATE match_map_scores s
        SET accuracy = p.accuracy
        FROM pairs p
        WHERE s.match_id = p.match_id AND s.position = p.position AND s.user_id = p.user_id
          AND s.accuracy IS DISTINCT FROM p.accuracy
        "#,
    )
    .bind(mp_id)
    .bind(&beatmaps)
    .bind(&users)
    .bind(&values)
    .bind(&accuracy)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query(
        r#"
        UPDATE match_rooms
        SET scores_read_at = now(),
            scores_misses = CASE WHEN $2 THEN 0 ELSE scores_misses + 1 END
        WHERE mp_id = $1
        "#,
    )
    .bind(mp_id)
    .bind(changed > 0)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(changed)
}

// ---------- a stage's statistics ----------

// How far each published stage has got: its matches with a saved map, and those being
// played now (a room, not concluded). The qualifier stage's matches are the lobbies.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StageActivity {
    pub stage_id: Uuid,
    pub played: i64,
    pub live: i64,
}

pub async fn stage_activity(pool: &PgPool) -> sqlx::Result<Vec<StageActivity>> {
    sqlx::query_as::<_, StageActivity>(
        r#"
        SELECT s.id AS stage_id,
               CASE WHEN s.is_qualifier THEN (
                   SELECT count(*) FROM matches m
                   WHERE m.kind = 'qualifier'
                     AND EXISTS (SELECT 1 FROM match_map_results q WHERE q.match_id = m.id)
               ) ELSE (
                   SELECT count(*) FROM bracket_matches b
                   WHERE b.stage_id = s.id
                     AND EXISTS (SELECT 1 FROM match_map_results r WHERE r.match_id = b.match_id)
               ) END AS played,
               CASE WHEN s.is_qualifier THEN (
                   SELECT count(*) FROM matches m
                   WHERE m.kind = 'qualifier' AND m.concluded_at IS NULL
                     AND EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
               ) ELSE (
                   SELECT count(*) FROM bracket_matches b
                   JOIN matches m ON m.id = b.match_id
                   WHERE b.stage_id = s.id AND m.concluded_at IS NULL
                     AND EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
               ) END AS live
        FROM stages s
        WHERE s.published
        ORDER BY s.position, s.created_at
        "#,
    )
    .fetch_all(pool)
    .await
}

// A saved player score on a pool map of the stage, with the player's team and the
// accuracy osu! recorded for it (null until it's been read).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StatScore {
    pub match_id: Uuid,
    pub position: i32,
    pub map_id: Uuid,
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub score: i64,
    pub mods: Option<String>,
    pub passed: bool,
    pub accuracy: Option<f64>,
}

pub async fn bracket_scores(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<StatScore>> {
    sqlx::query_as::<_, StatScore>(
        r#"
        SELECT s.match_id, s.position, res.pool_map_id AS map_id, s.user_id, tm.team_id,
               s.score, s.mods, s.passed, s.accuracy
        FROM match_map_scores s
        JOIN match_map_results res USING (match_id, position)
        JOIN bracket_matches b ON b.match_id = s.match_id
        JOIN pool_maps pm ON pm.id = res.pool_map_id
        JOIN category_slots cs ON cs.id = pm.slot_id
        JOIN stage_categories c ON c.id = cs.category_id AND c.stage_id = $1
        JOIN team_members tm ON tm.user_id = s.user_id AND tm.team_id IN (b.team1_id, b.team2_id)
        WHERE b.stage_id = $1
        ORDER BY s.match_id, s.position, s.score DESC
        "#,
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

// Every lobby's saved scores on the qualifier pool (`stage_id`).
pub async fn qualifier_scores(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<StatScore>> {
    sqlx::query_as::<_, StatScore>(
        r#"
        SELECT s.match_id, s.position, res.pool_map_id AS map_id, s.user_id, tm.team_id,
               s.score, s.mods, s.passed, s.accuracy
        FROM match_map_scores s
        JOIN match_map_results res USING (match_id, position)
        JOIN matches m ON m.id = s.match_id AND m.kind = 'qualifier'
        JOIN pool_maps pm ON pm.id = res.pool_map_id
        JOIN category_slots cs ON cs.id = pm.slot_id
        JOIN stage_categories c ON c.id = cs.category_id AND c.stage_id = $1
        JOIN team_members tm ON tm.user_id = s.user_id
        ORDER BY s.match_id, s.position, s.score DESC
        "#,
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

// A match of the stage: a bracket match of it, or a qualifier lobby, in play order.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StatMatch {
    pub id: Uuid,
    pub side: Option<Side>,
    pub number: Option<i32>,
    pub lobby_index: Option<i32>,
    pub concluded_at: Option<OffsetDateTime>,
    pub has_room: bool,
    // Qualifier lobbies: the teams signed up.
    pub teams: Vec<Uuid>,
}

pub async fn bracket_matches(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<StatMatch>> {
    sqlx::query_as::<_, StatMatch>(
        r#"
        SELECT m.id, b.side, b.number, NULL::int AS lobby_index, m.concluded_at,
               EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id) AS has_room,
               ARRAY[]::uuid[] AS teams
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        WHERE b.stage_id = $1
        ORDER BY m.starts_at NULLS LAST, b.side, b.number
        "#,
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

pub async fn qualifier_lobbies(pool: &PgPool) -> sqlx::Result<Vec<StatMatch>> {
    sqlx::query_as::<_, StatMatch>(
        r#"
        SELECT m.id, NULL::bracket_side AS side, NULL::int AS number, m.lobby_index,
               m.concluded_at,
               EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id) AS has_room,
               ARRAY(SELECT e.team_id FROM qualifier_entries e WHERE e.match_id = m.id
                     ORDER BY e.joined_at) AS teams
        FROM matches m
        WHERE m.kind = 'qualifier'
        ORDER BY m.starts_at, m.lobby_index
        "#,
    )
    .fetch_all(pool)
    .await
}

// A ban or a pick in one of the stage's bracket matches.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DraftMark {
    pub match_id: Uuid,
    pub map_id: Uuid,
    pub team_id: Uuid,
    // The other team of the match; None while its slot is empty.
    pub opponent_id: Option<Uuid>,
    pub ban: bool,
}

pub async fn drafts(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<DraftMark>> {
    sqlx::query_as::<_, DraftMark>(
        r#"
        SELECT x.match_id, x.pool_map_id AS map_id, x.team_id,
               CASE WHEN x.team_id = b.team1_id THEN b.team2_id ELSE b.team1_id END AS opponent_id,
               true AS ban
        FROM match_bans x JOIN bracket_matches b ON b.match_id = x.match_id
        WHERE b.stage_id = $1
        UNION ALL
        SELECT x.match_id, x.pool_map_id AS map_id, x.team_id,
               CASE WHEN x.team_id = b.team1_id THEN b.team2_id ELSE b.team1_id END AS opponent_id,
               false AS ban
        FROM match_picks x JOIN bracket_matches b ON b.match_id = x.match_id
        WHERE b.stage_id = $1
        "#,
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}
