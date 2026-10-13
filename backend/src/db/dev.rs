use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::role::Role;

const CLEAR_SQL: &str = include_str!("../../seed/clear.sql");
// Postgres runs a multi-statement query as one transaction, so a failed seed
// leaves the old data in place.
const RESEED_SQL: &str = concat!(
    include_str!("../../seed/clear.sql"),
    include_str!("../../seed/seed.sql")
);

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DevAccount {
    pub id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub avatar_url: String,
    pub global_rank: Option<i64>,
    pub role: Role,
    pub dummy: bool,
    pub in_team: bool,
    pub discord_linked: bool,
}

// Real accounts first, then dummies by rank.
pub async fn list_accounts(pool: &PgPool) -> sqlx::Result<Vec<DevAccount>> {
    sqlx::query_as::<_, DevAccount>(
        r#"
        SELECT u.id, u.osu_id, u.username, u.avatar_url, u.global_rank, u.role,
               u.osu_id < 0 AS dummy,
               EXISTS(SELECT 1 FROM team_members m WHERE m.user_id = u.id) AS in_team,
               EXISTS(SELECT 1 FROM discord_accounts d WHERE d.osu_user_id = u.id)
                   AS discord_linked
        FROM osu_users u
        ORDER BY u.osu_id < 0, u.role <> 'basic' DESC, u.global_rank ASC NULLS LAST, u.username
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn user_exists(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM osu_users WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn seed(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::raw_sql(RESEED_SQL).execute(pool).await?;
    Ok(())
}

pub async fn clear(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::raw_sql(CLEAR_SQL).execute(pool).await?;
    Ok(())
}
