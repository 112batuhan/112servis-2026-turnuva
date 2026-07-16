use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{discord_api::user::DiscordProfile, osu_api::user::OsuProfile, role::Role};

// A row in the osu_users table.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OsuUser {
    pub id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: Option<String>,
    pub global_rank: Option<i64>,
    pub country_rank: Option<i64>,
    pub role: Role,
    // Internal revocation counter; never sent to the client.
    #[serde(skip_serializing)]
    pub token_version: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// A row in the discord_accounts table, linked to an osu_users row.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DiscordAccount {
    pub id: Uuid,
    pub osu_user_id: Uuid,
    pub discord_id: String,
    pub username: String,
    pub avatar: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// What find_user sends back to the frontend: the osu! profile plus an optional linked Discord account.
#[derive(Debug, Serialize)]
pub struct UserProfile {
    #[serde(flatten)]
    pub osu: OsuUser,
    pub discord: Option<DiscordAccount>,
}

// Creates a user on first osu! login, or refreshes their profile on later logins.
pub async fn upsert_osu_user(pool: &PgPool, profile: &OsuProfile) -> sqlx::Result<OsuUser> {
    sqlx::query_as::<_, OsuUser>(
        r#"
        INSERT INTO osu_users (osu_id, username, avatar_url, country_code, global_rank, country_rank)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (osu_id) DO UPDATE
        SET username = EXCLUDED.username,
            avatar_url = EXCLUDED.avatar_url,
            country_code = EXCLUDED.country_code,
            global_rank = EXCLUDED.global_rank,
            country_rank = EXCLUDED.country_rank,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(profile.id)
    .bind(&profile.username)
    .bind(&profile.avatar_url)
    .bind(&profile.country_code)
    .bind(profile.statistics.global_rank)
    .bind(profile.statistics.country_rank)
    .fetch_one(pool)
    .await
}

// Attaches a Discord profile to an already-existing osu! user, replacing any previous link.
pub async fn link_discord(
    pool: &PgPool,
    osu_user_id: Uuid,
    profile: &DiscordProfile,
) -> sqlx::Result<DiscordAccount> {
    sqlx::query_as::<_, DiscordAccount>(
        r#"
        INSERT INTO discord_accounts (osu_user_id, discord_id, username, avatar)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (osu_user_id) DO UPDATE
        SET discord_id = EXCLUDED.discord_id,
            username = EXCLUDED.username,
            avatar = EXCLUDED.avatar,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(osu_user_id)
    .bind(&profile.id)
    .bind(&profile.username)
    .bind(&profile.avatar)
    .fetch_one(pool)
    .await
}

// Looks up a user by internal id, joining in their linked Discord account if any.
pub async fn find_user(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Option<UserProfile>> {
    let Some(osu) = sqlx::query_as::<_, OsuUser>("SELECT * FROM osu_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };

    let discord = sqlx::query_as::<_, DiscordAccount>(
        "SELECT * FROM discord_accounts WHERE osu_user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(Some(UserProfile { osu, discord }))
}

// The user's current token_version, or None if the user no longer exists. Used
// by /api/me to detect a JWT that predates a role change and re-issue the cookie.
pub async fn token_version(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar::<_, i32>("SELECT token_version FROM osu_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

// Whether the given osu! user has a linked Discord account (i.e. is "verified").
pub async fn is_discord_linked(pool: &PgPool, osu_user_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM discord_accounts WHERE osu_user_id = $1)",
    )
    .bind(osu_user_id)
    .fetch_one(pool)
    .await
}
