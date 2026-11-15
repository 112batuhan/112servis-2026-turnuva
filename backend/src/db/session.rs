use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::role::Role;

// The user behind a live session, as the AuthUser extractor needs it. Roles, bans and
// Discord status come straight from their tables, so changes apply on the next request.
#[derive(Debug, sqlx::FromRow)]
pub struct SessionUser {
    pub id: Uuid,
    pub roles: Vec<Role>,
    pub discord_verified: bool,
    pub banned: bool,
}

// Stores a new session for `user_id`, keyed by the SHA-256 of its cookie token.
pub async fn create(
    pool: &PgPool,
    token_hash: &[u8],
    user_id: Uuid,
    expires_at: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(token_hash)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(())
}

// Looks up the user for an unexpired session, or None if it's unknown or expired.
pub async fn find_user(pool: &PgPool, token_hash: &[u8]) -> sqlx::Result<Option<SessionUser>> {
    sqlx::query_as::<_, SessionUser>(
        r#"
        SELECT u.id, u.roles,
               EXISTS(SELECT 1 FROM discord_accounts d WHERE d.osu_user_id = u.id)
                   AS discord_verified,
               EXISTS(SELECT 1 FROM user_bans b WHERE b.user_id = u.id) AS banned
        FROM sessions s
        JOIN osu_users u ON u.id = s.user_id
        WHERE s.token_hash = $1 AND s.expires_at > now()
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

// Ends one session (logout). A no-op if it's already gone.
pub async fn delete(pool: &PgPool, token_hash: &[u8]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

// Removes every expired session, returning how many were deleted.
pub async fn delete_expired(pool: &PgPool) -> sqlx::Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
