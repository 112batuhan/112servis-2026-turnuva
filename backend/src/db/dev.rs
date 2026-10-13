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

// Banners for some of the seeded teams, by team name. They're bytes, so they're
// added here rather than in seed.sql.
const BANNERS: [(&str, &[u8]); 5] = [
    (
        "Sokak Kedileri",
        include_bytes!("../../seed/banners/sokak-kedileri.jpg"),
    ),
    (
        "Tatlı Krizi",
        include_bytes!("../../seed/banners/tatli-krizi.jpg"),
    ),
    (
        "Rank Uçtu",
        include_bytes!("../../seed/banners/rank-uctu.jpg"),
    ),
    (
        "Çay Ocağı",
        include_bytes!("../../seed/banners/cay-ocagi.jpg"),
    ),
    (
        "Fırın Ekibi",
        include_bytes!("../../seed/banners/firin-ekibi.jpg"),
    ),
];

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
    let (names, images): (Vec<&str>, Vec<&[u8]>) = BANNERS.into_iter().unzip();
    sqlx::query(
        r#"
        INSERT INTO team_banners (team_id, content_type, data)
        SELECT t.id, 'image/jpeg', v.data
        FROM UNNEST($1::text[], $2::bytea[]) AS v (name, data)
        JOIN teams t ON t.name = v.name
        ON CONFLICT (team_id) DO UPDATE
        SET content_type = EXCLUDED.content_type, data = EXCLUDED.data, updated_at = now()
        "#,
    )
    .bind(names)
    .bind(images)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::raw_sql(CLEAR_SQL).execute(pool).await?;
    Ok(())
}
