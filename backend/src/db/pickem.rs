use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::team::Player;

// How a stage's pick'em opens and locks: by the rule, or as a host set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "pickem_mode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    // Opens once the stage before it is played, locks when its first match starts.
    Auto,
    // Open now, whatever the stage before it; still locks when its first match starts.
    Open,
    // Locked now.
    Closed,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StageRow {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub is_qualifier: bool,
    pub best_of: Option<i32>,
    pub pickem_mode: Mode,
}

// A bracket match as the pick'em reads it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MatchRow {
    pub id: Uuid,
    pub stage_id: Option<Uuid>,
    pub starts_at: Option<OffsetDateTime>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub forfeit_slot: Option<i16>,
    pub concluded_at: Option<OffsetDateTime>,
    pub has_room: bool,
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct PredictionRow {
    pub user_id: Uuid,
    pub match_id: Uuid,
    pub team_id: Uuid,
    pub winner_score: i32,
    pub loser_score: i32,
}

pub async fn list_stages(pool: &PgPool) -> sqlx::Result<Vec<StageRow>> {
    sqlx::query_as::<_, StageRow>(
        "SELECT id, name, position, is_qualifier, best_of, pickem_mode \
         FROM stages ORDER BY position, created_at",
    )
    .fetch_all(pool)
    .await
}

pub async fn list_matches(pool: &PgPool) -> sqlx::Result<Vec<MatchRow>> {
    sqlx::query_as::<_, MatchRow>(
        r#"
        SELECT m.id, b.stage_id, m.starts_at, b.team1_id, b.team2_id,
               b.team1_score, b.team2_score, b.forfeit_slot, m.concluded_at,
               EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id) AS has_room
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        "#,
    )
    .fetch_all(pool)
    .await
}

// Every potential match that can still happen as the pick'em reads it: in its bracket
// match's stage, with no result. Predictions on one only count if the match turns out
// to be that pairing, when they move to it (db::potential::sync); a set-aside one keeps
// its predictions, unseen, for if it comes back.
pub async fn list_potentials(pool: &PgPool) -> sqlx::Result<Vec<MatchRow>> {
    sqlx::query_as::<_, MatchRow>(
        r#"
        SELECT p.match_id AS id, b.stage_id, m.starts_at, p.team1_id, p.team2_id,
               NULL::int AS team1_score, NULL::int AS team2_score,
               NULL::smallint AS forfeit_slot, m.concluded_at, false AS has_room
        FROM potential_matches p
        JOIN matches m ON m.id = p.match_id
        JOIN bracket_matches b ON b.match_id = p.bracket_match_id
        WHERE p.removed_at IS NULL
        "#,
    )
    .fetch_all(pool)
    .await
}

// When each qualifier lobby was concluded (null while it isn't): the qualifier stage's
// matches, which the first bracket stage's pick'em waits for.
pub async fn list_lobby_conclusions(pool: &PgPool) -> sqlx::Result<Vec<Option<OffsetDateTime>>> {
    sqlx::query_scalar("SELECT concluded_at FROM matches WHERE kind = 'qualifier'")
        .fetch_all(pool)
        .await
}

pub async fn list_predictions(pool: &PgPool) -> sqlx::Result<Vec<PredictionRow>> {
    sqlx::query_as::<_, PredictionRow>(
        "SELECT user_id, match_id, team_id, winner_score, loser_score FROM pickem_predictions",
    )
    .fetch_all(pool)
    .await
}

pub async fn list_match_predictions(
    pool: &PgPool,
    match_id: Uuid,
) -> sqlx::Result<Vec<PredictionRow>> {
    sqlx::query_as::<_, PredictionRow>(
        "SELECT user_id, match_id, team_id, winner_score, loser_score \
         FROM pickem_predictions WHERE match_id = $1",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

// Everyone who has predicted anything (other than set-aside potential matches), for the
// standings.
pub async fn list_pickers(pool: &PgPool) -> sqlx::Result<Vec<Player>> {
    sqlx::query_as::<_, Player>(
        r#"
        SELECT u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank
        FROM osu_users u
        WHERE EXISTS (
            SELECT 1 FROM pickem_predictions p
            WHERE p.user_id = u.id
              AND NOT EXISTS (SELECT 1 FROM potential_matches x
                              WHERE x.match_id = p.match_id AND x.removed_at IS NOT NULL)
        )
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn save_prediction(pool: &PgPool, p: &PredictionRow) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO pickem_predictions (user_id, match_id, team_id, winner_score, loser_score)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id, match_id) DO UPDATE
        SET team_id = EXCLUDED.team_id, winner_score = EXCLUDED.winner_score,
            loser_score = EXCLUDED.loser_score, updated_at = now()
        "#,
    )
    .bind(p.user_id)
    .bind(p.match_id)
    .bind(p.team_id)
    .bind(p.winner_score)
    .bind(p.loser_score)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_prediction(pool: &PgPool, user_id: Uuid, match_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM pickem_predictions WHERE user_id = $1 AND match_id = $2")
        .bind(user_id)
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(())
}

// False when there's no such stage.
pub async fn set_mode(pool: &PgPool, stage_id: Uuid, mode: Mode) -> sqlx::Result<bool> {
    let result =
        sqlx::query("UPDATE stages SET pickem_mode = $2, updated_at = now() WHERE id = $1")
            .bind(stage_id)
            .bind(mode)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}
