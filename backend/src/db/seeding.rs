//! The hosts' seeding order (bracket_seeds) and what making the bracket from it, or
//! taking that bracket down, reads and writes.

use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::bracket::{Outcome, Side};

// The saved order, best seed first. Empty: the qualifier's order stands.
pub async fn saved_order(ex: impl PgExecutor<'_>) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar("SELECT team_id FROM bracket_seeds ORDER BY position, team_id")
        .fetch_all(ex)
        .await
}

pub async fn save_order(conn: &mut PgConnection, team_ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM bracket_seeds")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO bracket_seeds (team_id, position)
        SELECT v.team_id, v.ord - 1
        FROM UNNEST($1::uuid[]) WITH ORDINALITY AS v (team_id, ord)
        WHERE EXISTS (SELECT 1 FROM teams t WHERE t.id = v.team_id)
        "#,
    )
    .bind(team_ids)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn clear_order(ex: impl PgExecutor<'_>) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM bracket_seeds").execute(ex).await?;
    Ok(())
}

// The bracket as the seeding page needs it: how many matches it has, and how many of
// them have been played (concluded, or with a room: started).
#[derive(Debug, Clone, Copy, Default, sqlx::FromRow)]
pub struct BracketState {
    pub matches: i64,
    pub played: i64,
}

pub async fn bracket_state(ex: impl PgExecutor<'_>) -> sqlx::Result<BracketState> {
    sqlx::query_as::<_, BracketState>(
        r#"
        SELECT count(*) AS matches,
               count(*) FILTER (
                   WHERE m.concluded_at IS NOT NULL
                      OR EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
               ) AS played
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        "#,
    )
    .fetch_one(ex)
    .await
}

// Qualifier lobbies with teams in them that aren't concluded: their teams' missing maps
// don't count yet, so the seeding isn't final.
pub async fn lobbies_not_over(ex: impl PgExecutor<'_>) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM matches m
        WHERE m.kind = 'qualifier' AND m.concluded_at IS NULL
          AND EXISTS (SELECT 1 FROM qualifier_entries e WHERE e.match_id = m.id)
        "#,
    )
    .fetch_one(ex)
    .await
}

// Every bracket match under a row lock, so nothing starts or concludes while the
// bracket is taken down.
pub async fn lock_bracket(conn: &mut PgConnection) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id
        FROM matches m
        JOIN bracket_matches b ON b.match_id = m.id
        ORDER BY m.id
        FOR UPDATE OF m
        "#,
    )
    .fetch_all(conn)
    .await
}

pub async fn delete_bracket(conn: &mut PgConnection) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "DELETE FROM matches m WHERE m.kind = 'bracket' AND EXISTS (SELECT 1 FROM bracket_matches b WHERE b.match_id = m.id)",
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

pub struct NewMatch {
    pub side: Side,
    pub number: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub stage_id: Option<Uuid>,
}

// A bracket match with no time yet (the hosts schedule each week).
pub async fn insert_match(conn: &mut PgConnection, m: &NewMatch) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        r#"
        WITH new AS (INSERT INTO matches (kind) VALUES ('bracket') RETURNING id)
        INSERT INTO bracket_matches (match_id, side, number, pos_x, pos_y, team1_id, team2_id, stage_id)
        SELECT new.id, $1, $2, $3, $4, $5, $6, $7 FROM new
        RETURNING match_id
        "#,
    )
    .bind(m.side)
    .bind(m.number)
    .bind(m.pos_x)
    .bind(m.pos_y)
    .bind(m.team1_id)
    .bind(m.team2_id)
    .bind(m.stage_id)
    .fetch_one(conn)
    .await
}

pub async fn insert_link(
    conn: &mut PgConnection,
    source_id: Uuid,
    outcome: Outcome,
    target_id: Uuid,
    slot: i16,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO bracket_links (source_id, outcome, target_id, slot) VALUES ($1, $2, $3, $4)",
    )
    .bind(source_id)
    .bind(outcome)
    .bind(target_id)
    .bind(slot)
    .execute(conn)
    .await?;
    Ok(())
}

// How many of `ids` are teams.
pub async fn existing_teams(ex: impl PgExecutor<'_>, ids: &[Uuid]) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM teams WHERE id = ANY($1)")
        .bind(ids)
        .fetch_one(ex)
        .await
}

// How many of `ids` are bracket stages (not the qualifier).
pub async fn existing_bracket_stages(ex: impl PgExecutor<'_>, ids: &[Uuid]) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM stages WHERE id = ANY($1) AND NOT is_qualifier")
        .bind(ids)
        .fetch_one(ex)
        .await
}

// The teams in any bracket match: the ones a bracket made from the seeding took in.
pub async fn bracket_teams(ex: impl PgExecutor<'_>) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT team1_id FROM bracket_matches WHERE team1_id IS NOT NULL
        UNION
        SELECT team2_id FROM bracket_matches WHERE team2_id IS NOT NULL
        "#,
    )
    .fetch_all(ex)
    .await
}
