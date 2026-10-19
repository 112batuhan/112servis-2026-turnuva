use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{bracket::Side, team::Player};

// An unconcluded bracket match a team plays in, once its opponent is known, or a
// potential match it may play (`potential`), coded like its bracket match.
#[derive(Debug, sqlx::FromRow)]
pub struct TeamMatchRow {
    pub id: Uuid,
    pub potential: bool,
    pub side: Side,
    pub number: i32,
    pub starts_at: Option<OffsetDateTime>,
    pub stage_name: Option<String>,
    pub opponent_id: Uuid,
    pub opponent_name: String,
    pub has_room: bool,
}

// A proposed time together with the player who proposed it.
#[derive(Debug, sqlx::FromRow)]
pub struct ProposalRow {
    pub proposal_id: Uuid,
    pub match_id: Uuid,
    pub team_id: Uuid,
    pub starts_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    #[sqlx(flatten)]
    pub proposed_by: Player,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ProposalLock {
    pub match_id: Uuid,
    pub team_id: Uuid,
    pub starts_at: OffsetDateTime,
}

// The team's unconcluded matches with both teams set and its potential matches,
// soonest first.
pub async fn list_team_matches(pool: &PgPool, team_id: Uuid) -> sqlx::Result<Vec<TeamMatchRow>> {
    sqlx::query_as::<_, TeamMatchRow>(
        r#"
        SELECT * FROM (
            SELECT m.id, false AS potential, b.side, b.number, m.starts_at,
                   st.name AS stage_name, o.id AS opponent_id, o.name AS opponent_name,
                   EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id) AS has_room
            FROM bracket_matches b
            JOIN matches m ON m.id = b.match_id
            JOIN teams o ON o.id = CASE WHEN b.team1_id = $1 THEN b.team2_id ELSE b.team1_id END
            LEFT JOIN stages st ON st.id = b.stage_id
            WHERE $1 IN (b.team1_id, b.team2_id) AND m.concluded_at IS NULL
            UNION ALL
            SELECT m.id, true, b.side, b.number, m.starts_at, st.name, o.id, o.name, false
            FROM potential_matches p
            JOIN matches m ON m.id = p.match_id
            JOIN bracket_matches b ON b.match_id = p.bracket_match_id
            JOIN teams o ON o.id = CASE WHEN p.team1_id = $1 THEN p.team2_id ELSE p.team1_id END
            LEFT JOIN stages st ON st.id = b.stage_id
            WHERE $1 IN (p.team1_id, p.team2_id) AND p.removed_at IS NULL
        ) AS t
        ORDER BY starts_at NULLS LAST, side, number, potential, opponent_name
        "#,
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
}

// Every proposal made by either side for the team's unconcluded matches, earliest
// proposed time first.
pub async fn list_team_proposals(pool: &PgPool, team_id: Uuid) -> sqlx::Result<Vec<ProposalRow>> {
    sqlx::query_as::<_, ProposalRow>(
        r#"
        SELECT p.id AS proposal_id, p.match_id, p.team_id, p.starts_at, p.created_at,
               u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank
        FROM match_reschedules p
        JOIN (
            SELECT match_id, team1_id, team2_id FROM bracket_matches
            UNION ALL
            SELECT match_id, team1_id, team2_id FROM potential_matches WHERE removed_at IS NULL
        ) AS b ON b.match_id = p.match_id
        JOIN matches m ON m.id = p.match_id
        JOIN osu_users u ON u.id = p.proposed_by
        WHERE $1 IN (b.team1_id, b.team2_id)
          AND p.team_id IN (b.team1_id, b.team2_id)
          AND m.concluded_at IS NULL
        ORDER BY p.starts_at, p.created_at
        "#,
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
}

pub async fn find_proposal(
    ex: impl PgExecutor<'_>,
    id: Uuid,
) -> sqlx::Result<Option<ProposalLock>> {
    sqlx::query_as::<_, ProposalLock>(
        "SELECT match_id, team_id, starts_at FROM match_reschedules WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(ex)
    .await
}

pub async fn count_proposals(
    conn: &mut PgConnection,
    match_id: Uuid,
    team_id: Uuid,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT count(*) FROM match_reschedules WHERE match_id = $1 AND team_id = $2",
    )
    .bind(match_id)
    .bind(team_id)
    .fetch_one(conn)
    .await
}

pub async fn proposal_exists(
    conn: &mut PgConnection,
    match_id: Uuid,
    team_id: Uuid,
    starts_at: OffsetDateTime,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM match_reschedules
            WHERE match_id = $1 AND team_id = $2 AND starts_at = $3
        )
        "#,
    )
    .bind(match_id)
    .bind(team_id)
    .bind(starts_at)
    .fetch_one(conn)
    .await
}

// Returns None when the team already proposed that time.
pub async fn create_proposal(
    conn: &mut PgConnection,
    match_id: Uuid,
    team_id: Uuid,
    proposed_by: Uuid,
    starts_at: OffsetDateTime,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        r#"
        INSERT INTO match_reschedules (match_id, team_id, proposed_by, starts_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (match_id, team_id, starts_at) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(match_id)
    .bind(team_id)
    .bind(proposed_by)
    .bind(starts_at)
    .fetch_optional(conn)
    .await
}

// Deletes a proposal for a match `team_id` plays in: its own (cancel) or the
// opponent's (decline).
pub async fn delete_proposal(pool: &PgPool, id: Uuid, team_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM match_reschedules p
        USING (
            SELECT match_id, team1_id, team2_id FROM bracket_matches
            UNION ALL
            SELECT match_id, team1_id, team2_id FROM potential_matches WHERE removed_at IS NULL
        ) AS b
        WHERE p.id = $1 AND b.match_id = p.match_id AND $2 IN (b.team1_id, b.team2_id)
        "#,
    )
    .bind(id)
    .bind(team_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// Sets a match's time, without touching its proposals.
pub async fn set_time(
    conn: &mut PgConnection,
    match_id: Uuid,
    starts_at: Option<OffsetDateTime>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE matches SET starts_at = $2 WHERE id = $1")
        .bind(match_id)
        .bind(starts_at)
        .execute(conn)
        .await?;
    Ok(())
}

// Moves the match to the accepted time and drops every proposal for it.
pub async fn reschedule(
    conn: &mut PgConnection,
    match_id: Uuid,
    starts_at: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE matches SET starts_at = $2 WHERE id = $1")
        .bind(match_id)
        .bind(starts_at)
        .execute(&mut *conn)
        .await?;
    delete_for_match(conn, match_id).await
}

pub async fn delete_for_match(conn: &mut PgConnection, match_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM match_reschedules WHERE match_id = $1")
        .bind(match_id)
        .execute(conn)
        .await?;
    Ok(())
}

// Drops the proposals of teams that no longer play in the match, after a host
// changes its teams.
pub async fn delete_stale(conn: &mut PgConnection, match_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        DELETE FROM match_reschedules p
        USING bracket_matches b
        WHERE p.match_id = $1 AND b.match_id = p.match_id
          AND p.team_id IS DISTINCT FROM b.team1_id
          AND p.team_id IS DISTINCT FROM b.team2_id
        "#,
    )
    .bind(match_id)
    .execute(conn)
    .await?;
    Ok(())
}
