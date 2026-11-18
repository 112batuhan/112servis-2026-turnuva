//! The auto referee's saved state and log (bancho/autoref/), its switch, and the
//! matches it should open a room for now.

use serde::Serialize;
use sqlx::{types::Json, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "autoref_log_level", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LogEntry {
    pub id: i64,
    pub level: LogLevel,
    pub message: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

pub async fn find_state(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Option<serde_json::Value>> {
    let row: Option<Json<serde_json::Value>> =
        sqlx::query_scalar("SELECT state FROM match_autoref WHERE match_id = $1")
            .bind(match_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|j| j.0))
}

pub async fn save_state(
    pool: &PgPool,
    match_id: Uuid,
    state: &serde_json::Value,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO match_autoref (match_id, state, updated_at)
        VALUES ($1, $2, now())
        ON CONFLICT (match_id) DO UPDATE SET state = EXCLUDED.state, updated_at = now()
        "#,
    )
    .bind(match_id)
    .bind(Json(state))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_log(
    pool: &PgPool,
    match_id: Uuid,
    level: LogLevel,
    message: &str,
) -> sqlx::Result<LogEntry> {
    sqlx::query_as::<_, LogEntry>(
        r#"
        INSERT INTO autoref_logs (match_id, level, message)
        VALUES ($1, $2, $3)
        RETURNING id, level, message, created_at
        "#,
    )
    .bind(match_id)
    .bind(level)
    .bind(message)
    .fetch_one(pool)
    .await
}

// The match's latest `limit` lines, oldest first.
pub async fn list_logs(pool: &PgPool, match_id: Uuid, limit: i64) -> sqlx::Result<Vec<LogEntry>> {
    sqlx::query_as::<_, LogEntry>(
        r#"
        SELECT id, level, message, created_at FROM (
            SELECT id, level, message, created_at FROM autoref_logs
            WHERE match_id = $1
            ORDER BY id DESC
            LIMIT $2
        ) latest
        ORDER BY id
        "#,
    )
    .bind(match_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

// Switches the match between the auto referee and manual control.
pub async fn set_enabled(pool: &PgPool, match_id: Uuid, enabled: bool) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO match_room_settings (match_id, auto_ref)
        VALUES ($1, $2)
        ON CONFLICT (match_id) DO UPDATE SET auto_ref = EXCLUDED.auto_ref
        "#,
    )
    .bind(match_id)
    .bind(enabled)
    .execute(pool)
    .await?;
    Ok(())
}

// Matches in auto mode that start within `within_secs` (and started no more than
// `past_secs` ago) and have no room yet: a bracket match with both teams, or a lobby
// with a team in it. Soonest first. None while the hosts have rooms opening by
// themselves switched off (app_settings.auto_rooms).
pub async fn due_rooms(pool: &PgPool, within_secs: i64, past_secs: i64) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        LEFT JOIN match_room_settings s ON s.match_id = m.id
        WHERE (SELECT auto_rooms FROM app_settings)
          AND m.concluded_at IS NULL
          AND m.starts_at <= now() + make_interval(secs => $1)
          AND m.starts_at > now() - make_interval(secs => $2)
          AND COALESCE(s.auto_ref, true)
          AND NOT EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
          AND CASE m.kind
                  WHEN 'bracket' THEN b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
                                      AND b.forfeit_slot IS NULL
                  WHEN 'qualifier' THEN EXISTS (SELECT 1 FROM qualifier_entries e
                                                WHERE e.match_id = m.id)
                  ELSE false
              END
        ORDER BY m.starts_at
        "#,
    )
    .bind(within_secs as f64)
    .bind(past_secs as f64)
    .fetch_all(pool)
    .await
}
