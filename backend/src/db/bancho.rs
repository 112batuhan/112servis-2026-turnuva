use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::secret::{BotSecret, Sealed};

// A bot account as hosts see it on /admin/bancho: never with its password.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BotRow {
    pub id: Uuid,
    pub username: String,
    pub active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

// What the bot connection logs in with, its password decrypted.
#[derive(Debug, Clone)]
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

#[derive(sqlx::FromRow)]
struct LoginRow {
    id: Uuid,
    username: String,
    // Only on rows saved before passwords were encrypted, until the backend encrypts them.
    irc_password: Option<String>,
    password_nonce: Option<Vec<u8>>,
    password_ciphertext: Option<Vec<u8>>,
    active: bool,
}

// Every bot whose password can be read: encrypted ones need the key they were sealed
// with. The rest are left out (and so stay offline) with an error in the log.
pub async fn list_logins(pool: &PgPool, secret: Option<&BotSecret>) -> sqlx::Result<Vec<BotLogin>> {
    let rows = sqlx::query_as::<_, LoginRow>(
        r#"
        SELECT id, username, irc_password, password_nonce, password_ciphertext, active
        FROM bancho_bots
        ORDER BY created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let password = match (&row.password_nonce, &row.password_ciphertext, secret) {
                (Some(nonce), Some(ciphertext), Some(secret)) => secret.open(nonce, ciphertext),
                _ => None,
            }
            .or(row.irc_password);
            if password.is_none() {
                tracing::error!(
                    bot = %row.username,
                    "can't read the bot's IRC password (BOT_SECRET_KEY unset or changed); enter it again on /admin/bancho"
                );
            }
            Some(BotLogin {
                id: row.id,
                username: row.username,
                irc_password: password?,
                active: row.active,
            })
        })
        .collect())
}

// Fails on bancho_bots_username_key when the account is already a bot.
pub async fn create_bot(pool: &PgPool, username: &str, password: &Sealed) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        r#"
        INSERT INTO bancho_bots (username, password_nonce, password_ciphertext)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(username)
    .bind(&password.nonce)
    .bind(&password.ciphertext)
    .fetch_one(pool)
    .await
}

pub async fn set_password(pool: &PgPool, id: Uuid, password: &Sealed) -> sqlx::Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE bancho_bots
        SET password_nonce = $2, password_ciphertext = $3, irc_password = NULL
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&password.nonce)
    .bind(&password.ciphertext)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// Encrypts the passwords saved before they were, and empties the plaintext. Returns how
// many it encrypted.
pub async fn encrypt_plaintext(pool: &PgPool, secret: &BotSecret) -> sqlx::Result<usize> {
    let mut tx = pool.begin().await?;
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, irc_password FROM bancho_bots WHERE irc_password IS NOT NULL FOR UPDATE",
    )
    .fetch_all(&mut *tx)
    .await?;
    for (id, password) in &rows {
        let sealed = secret.seal(password);
        sqlx::query(
            r#"
            UPDATE bancho_bots
            SET password_nonce = $2, password_ciphertext = $3, irc_password = NULL
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(&sealed.nonce)
        .bind(&sealed.ciphertext)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(rows.len())
}

// How many bots still have their password in plaintext.
pub async fn count_plaintext(pool: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM bancho_bots WHERE irc_password IS NOT NULL")
        .fetch_one(pool)
        .await
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
    ex: impl PgExecutor<'_>,
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
    .execute(ex)
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
