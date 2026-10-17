use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

// A bot account as hosts see it on /admin/bancho: never with its password.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BotRow {
    pub id: Uuid,
    pub username: String,
    pub active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

// What the bot connection logs in with.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BotLogin {
    pub id: Uuid,
    pub username: String,
    pub irc_password: String,
    pub active: bool,
}

pub async fn list_bots(pool: &PgPool) -> sqlx::Result<Vec<BotRow>> {
    sqlx::query_as::<_, BotRow>(
        "SELECT id, username, active, created_at FROM bancho_bots ORDER BY created_at, id",
    )
    .fetch_all(pool)
    .await
}

pub async fn list_logins(pool: &PgPool) -> sqlx::Result<Vec<BotLogin>> {
    sqlx::query_as::<_, BotLogin>(
        "SELECT id, username, irc_password, active FROM bancho_bots ORDER BY created_at, id",
    )
    .fetch_all(pool)
    .await
}

// Fails on bancho_bots_username_key when the account is already a bot.
pub async fn create_bot(pool: &PgPool, username: &str, irc_password: &str) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO bancho_bots (username, irc_password) VALUES ($1, $2) RETURNING id",
    )
    .bind(username)
    .bind(irc_password)
    .fetch_one(pool)
    .await
}

pub async fn set_password(pool: &PgPool, id: Uuid, irc_password: &str) -> sqlx::Result<bool> {
    let result = sqlx::query("UPDATE bancho_bots SET irc_password = $2 WHERE id = $1")
        .bind(id)
        .bind(irc_password)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_active(pool: &PgPool, id: Uuid, active: bool) -> sqlx::Result<bool> {
    let result = sqlx::query("UPDATE bancho_bots SET active = $2 WHERE id = $1")
        .bind(id)
        .bind(active)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// Its rooms stay on their matches without a bot.
pub async fn delete_bot(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM bancho_bots WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// A room on a match, with the bot that runs it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoomOwner {
    pub mp_id: i64,
    pub match_id: Uuid,
    pub bot_id: Option<Uuid>,
    pub bot_made: bool,
    pub added_at: OffsetDateTime,
    pub closed_at: Option<OffsetDateTime>,
    // The match's.
    pub concluded_at: Option<OffsetDateTime>,
}

// The rooms the bots look after: every room of an unconcluded match, and every room
// still open on Bancho as far as the site knows, whatever its match.
pub async fn list_tracked_rooms(pool: &PgPool) -> sqlx::Result<Vec<RoomOwner>> {
    sqlx::query_as::<_, RoomOwner>(
        r#"
        SELECT r.mp_id, r.match_id, r.bot_id, r.bot_made, r.added_at, r.closed_at,
               m.concluded_at
        FROM match_rooms r
        JOIN matches m ON m.id = r.match_id
        WHERE m.concluded_at IS NULL OR (r.closed_at IS NULL AND r.bot_id IS NOT NULL)
        ORDER BY r.added_at, r.mp_id
        "#,
    )
    .fetch_all(pool)
    .await
}

// Adds a room a bot made, or one made elsewhere and handed to `bot_id`. Fails on
// match_rooms_pkey when the room already belongs to a match.
pub async fn add_room(
    pool: &PgPool,
    match_id: Uuid,
    mp_id: i64,
    bot_id: Option<Uuid>,
    bot_made: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO match_rooms (mp_id, match_id, bot_id, bot_made) VALUES ($1, $2, $3, $4)",
    )
    .bind(mp_id)
    .bind(match_id)
    .bind(bot_id)
    .bind(bot_made)
    .execute(pool)
    .await?;
    Ok(())
}

// The room was closed on Bancho (or turned out to be gone), or a bot that didn't make it
// left it: it no longer counts against its bot.
pub async fn mark_closed(pool: &PgPool, mp_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE match_rooms SET closed_at = now() WHERE mp_id = $1 AND closed_at IS NULL")
        .bind(mp_id)
        .execute(pool)
        .await?;
    Ok(())
}

// The unconcluded matches that have a room: the ones the bots run.
pub async fn list_running_matches(pool: &PgPool) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id
        FROM matches m
        WHERE m.concluded_at IS NULL
          AND EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = m.id)
        "#,
    )
    .fetch_all(pool)
    .await
}
