//! The tournament's stages: its rounds, each with a map pool (db/mappool.rs) and, for a
//! bracket stage, how its matches are played (best of, ban and pick order).

use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Stage {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub published: bool,
    // Whether this is the qualifier lobbies' map pool; at most one stage is.
    pub is_qualifier: bool,
    // How many maps its bracket matches are played to (odd); null until the hosts set it.
    pub best_of: Option<i32>,
    // The order its bracket matches ban and pick in (crate::draft); null: the default.
    pub draft_order: Option<String>,
    // Where its beatmap pack is downloaded from (hosted elsewhere); null: none yet.
    pub pack_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

pub async fn list_stages(pool: &PgPool) -> sqlx::Result<Vec<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages ORDER BY position, created_at")
        .fetch_all(pool)
        .await
}

pub async fn create_stage(pool: &PgPool, name: &str) -> sqlx::Result<Stage> {
    sqlx::query_as::<_, Stage>(
        r#"
        INSERT INTO stages (name, position)
        VALUES ($1, (SELECT COALESCE(MAX(position), -1) + 1 FROM stages))
        RETURNING *
        "#,
    )
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn get_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM stages WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// None when there's no such stage.
pub async fn rename_stage(pool: &PgPool, id: Uuid, name: &str) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>(
        "UPDATE stages SET name = $2, updated_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(name)
    .fetch_optional(pool)
    .await
}

// Publishes or unpublishes a stage's pool; published stages are visible on the public
// pages. None when there's no such stage.
pub async fn set_stage_published(
    pool: &PgPool,
    id: Uuid,
    published: bool,
) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>(
        "UPDATE stages SET published = $2, updated_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(published)
    .fetch_optional(pool)
    .await
}

// Sets each stage's position to its index in `ids` (one statement via WITH ORDINALITY).
pub async fn reorder_stages(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE stages s SET position = v.ord - 1, updated_at = now() \
         FROM unnest($1::uuid[]) WITH ORDINALITY AS v(id, ord) WHERE s.id = v.id",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(())
}

// Makes `id` the qualifier stage in place of the current one, or leaves none with
// None. False when there's no such stage. Two statements, since the unique index is
// checked row by row and would trip over the old and new one both being set.
pub async fn set_qualifier_stage(pool: &PgPool, id: Option<Uuid>) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE stages SET is_qualifier = false WHERE is_qualifier")
        .execute(&mut *tx)
        .await?;
    if let Some(id) = id {
        let result = sqlx::query("UPDATE stages SET is_qualifier = true WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
    }
    tx.commit().await?;
    Ok(true)
}

// Sets the order a stage's bracket matches ban and pick in (None: the default). False
// when there's no such stage.
pub async fn set_stage_draft_order(
    pool: &PgPool,
    id: Uuid,
    order: Option<&str>,
) -> sqlx::Result<bool> {
    let found = sqlx::query("UPDATE stages SET draft_order = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(order)
        .execute(pool)
        .await?;
    Ok(found.rows_affected() > 0)
}

// Sets the link to a stage's beatmap pack (None: no pack). False when there's no such
// stage.
pub async fn set_stage_pack_url(pool: &PgPool, id: Uuid, url: Option<&str>) -> sqlx::Result<bool> {
    let found = sqlx::query("UPDATE stages SET pack_url = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(url)
        .execute(pool)
        .await?;
    Ok(found.rows_affected() > 0)
}

// Sets how many maps a stage's bracket matches are played to (None: not set). Returns
// the ids of its bracket matches, whose Discord cards show it, or None when there's no
// such stage.
pub async fn set_stage_best_of(
    conn: &mut PgConnection,
    id: Uuid,
    best_of: Option<i32>,
) -> sqlx::Result<Option<Vec<Uuid>>> {
    let found = sqlx::query("UPDATE stages SET best_of = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(best_of)
        .execute(&mut *conn)
        .await?;
    if found.rows_affected() == 0 {
        return Ok(None);
    }
    let matches = sqlx::query_scalar("SELECT match_id FROM bracket_matches WHERE stage_id = $1")
        .bind(id)
        .fetch_all(&mut *conn)
        .await?;
    Ok(Some(matches))
}

// A published stage as the public stage tabs show it: how far it has got (counted as
// db::stats::stage_activity counts it: its matches with a saved map, and those with a room
// that aren't concluded; the qualifier's are the lobbies) and whether the tournament is in
// it. That's the rule of the landing page's track (frontend/src/lib/stageTrack.ts): the
// qualifier while a lobby is open and no bracket match is over, else the stage of the
// earliest unfinished bracket match (by time, the unscheduled last, then W before L and
// by number).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicStage {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub stage: Stage,
    pub played: i64,
    pub live: i64,
    pub current: bool,
}

pub async fn list_published_stages(pool: &PgPool) -> sqlx::Result<Vec<PublicStage>> {
    sqlx::query_as::<_, PublicStage>(
        r#"
        WITH progress AS (
            SELECT EXISTS (
                       SELECT 1 FROM matches
                       WHERE kind = 'qualifier' AND concluded_at IS NULL
                   ) AS qualifiers_open,
                   EXISTS (
                       SELECT 1 FROM bracket_matches b JOIN matches m ON m.id = b.match_id
                       WHERE m.concluded_at IS NOT NULL
                   ) AS bracket_started,
                   (
                       SELECT b.stage_id FROM bracket_matches b JOIN matches m ON m.id = b.match_id
                       WHERE m.concluded_at IS NULL
                       ORDER BY m.starts_at ASC NULLS LAST, b.side, b.number
                       LIMIT 1
                   ) AS next_stage_id
        )
        SELECT s.*,
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
               ) END AS live,
               CASE WHEN p.qualifiers_open AND NOT p.bracket_started THEN s.is_qualifier
                    ELSE COALESCE(s.id = p.next_stage_id, false)
               END AS current
        FROM stages s
        CROSS JOIN progress p
        WHERE s.published
        ORDER BY s.position, s.created_at
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_published_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE id = $1 AND published")
        .bind(id)
        .fetch_optional(pool)
        .await
}

// The qualifier stage's id, when it's published; the qualifier pages link to its pool.
pub async fn published_qualifier_stage_id(pool: &PgPool) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM stages WHERE is_qualifier AND published")
        .fetch_optional(pool)
        .await
}
