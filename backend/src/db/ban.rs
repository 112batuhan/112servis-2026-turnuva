use serde::Serialize;
use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

// A user's ban, as they and the hosts see it.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Ban {
    // Empty when the host gave none.
    pub reason: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

pub async fn find(ex: impl PgExecutor<'_>, user_id: Uuid) -> sqlx::Result<Option<Ban>> {
    sqlx::query_as::<_, Ban>("SELECT reason, created_at FROM user_bans WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(ex)
        .await
}

// A ban as written: the ban, and whether it's new rather than a changed reason.
#[derive(Debug, sqlx::FromRow)]
pub struct BanWrite {
    #[sqlx(flatten)]
    pub ban: Ban,
    pub new: bool,
}

// Bans a user, or changes the reason of their ban, and drops every pending invite
// involving them (they could no longer accept or be accepted). None when there's no such
// user.
pub async fn ban(
    conn: &mut PgConnection,
    user_id: Uuid,
    reason: &str,
    banned_by: Uuid,
) -> sqlx::Result<Option<BanWrite>> {
    // A row the insert made has no xmax; one the conflict updated has.
    let ban = sqlx::query_as::<_, BanWrite>(
        r#"
        INSERT INTO user_bans (user_id, reason, banned_by)
        SELECT id, $2, $3 FROM osu_users WHERE id = $1
        ON CONFLICT (user_id) DO UPDATE SET reason = EXCLUDED.reason
        RETURNING reason, created_at, (xmax = 0) AS new
        "#,
    )
    .bind(user_id)
    .bind(reason)
    .bind(banned_by)
    .fetch_optional(&mut *conn)
    .await?;
    if ban.is_some() {
        sqlx::query("DELETE FROM team_invites WHERE inviter_id = $1 OR invitee_id = $1")
            .bind(user_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(ban)
}

// Lifts a ban; false when the user wasn't banned.
pub async fn unban(ex: impl PgExecutor<'_>, user_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM user_bans WHERE user_id = $1")
        .bind(user_id)
        .execute(ex)
        .await?;
    Ok(result.rows_affected() > 0)
}

// A ban as the hosts' Discord channel reports it: the reason and who banned them.
#[derive(Debug, sqlx::FromRow)]
pub struct BanDetails {
    pub reason: String,
    pub banned_by_username: Option<String>,
}

pub async fn details(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Option<BanDetails>> {
    sqlx::query_as::<_, BanDetails>(
        r#"
        SELECT b.reason, h.username AS banned_by_username
        FROM user_bans b
        LEFT JOIN osu_users h ON h.id = b.banned_by
        WHERE b.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
