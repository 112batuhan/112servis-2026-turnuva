use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::user::OsuUser;
use crate::role::Role;

// A registered user for the admin list: the osu! profile with any linked Discord
// account folded in. The discord_* fields are null when there's no linked account.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUser {
    pub id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: Option<String>,
    pub global_rank: Option<i64>,
    pub country_rank: Option<i64>,
    pub roles: Vec<Role>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub discord_id: Option<String>,
    pub discord_username: Option<String>,
    pub discord_avatar: Option<String>,
    // Set when the hosts banned them: why (empty when no reason was given), when, by whom.
    pub ban_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub banned_at: Option<OffsetDateTime>,
    pub banned_by_username: Option<String>,
}

// Every registered user, newest first. A single LEFT JOIN folds in the Discord
// account so users who never linked Discord still appear (with null discord_*).
pub async fn list_users(pool: &PgPool) -> sqlx::Result<Vec<AdminUser>> {
    sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT o.id, o.osu_id, o.username, o.avatar_url, o.country_code,
               o.global_rank, o.country_rank, o.roles, o.created_at,
               d.discord_id AS discord_id,
               d.username   AS discord_username,
               d.avatar     AS discord_avatar,
               b.reason     AS ban_reason,
               b.created_at AS banned_at,
               bb.username  AS banned_by_username
        FROM osu_users o
        LEFT JOIN discord_accounts d ON d.osu_user_id = o.id
        LEFT JOIN user_bans b ON b.user_id = o.id
        LEFT JOIN osu_users bb ON bb.id = b.banned_by
        ORDER BY o.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

// Replaces a user's roles (already normalized). Returns the updated row.
pub async fn set_roles(pool: &PgPool, user_id: Uuid, roles: &[Role]) -> sqlx::Result<OsuUser> {
    sqlx::query_as::<_, OsuUser>(
        r#"
        UPDATE osu_users
        SET roles = $2, updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(roles)
    .fetch_one(pool)
    .await
}
