use serde::Serialize;
use sqlx::{Executor, PgConnection, PgPool};
use uuid::Uuid;

use crate::role::Role;

const CLEAR_SQL: &str = include_str!("../../seed/clear.sql");
// Seeding runs in one transaction, so a failed seed leaves the old data in place.
const RESEED_SQL: &str = concat!(
    include_str!("../../seed/clear.sql"),
    include_str!("../../seed/model.sql"),
    include_str!("../../seed/seed.sql")
);
// The tournament after its qualifier instead, with no bracket yet. seed.sql drops the
// score model at its end, so it's made again.
const QUALIFIERS_SQL: &str = concat!(
    include_str!("../../seed/model.sql"),
    include_str!("../../seed/qualifiers.sql")
);
// Only a few teams instead: no lobby, score, stage or bracket (teams.sql).
const TEAMS_SQL: &str = concat!(
    include_str!("../../seed/clear.sql"),
    include_str!("../../seed/teams.sql")
);
const LIVE_SQL: &str = include_str!("../../seed/live.sql");

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
    pub roles: Vec<Role>,
    pub dummy: bool,
    pub in_team: bool,
    pub discord_linked: bool,
}

// Real accounts first, then dummies by rank.
pub async fn list_accounts(pool: &PgPool) -> sqlx::Result<Vec<DevAccount>> {
    sqlx::query_as::<_, DevAccount>(
        r#"
        SELECT u.id, u.osu_id, u.username, u.avatar_url, u.global_rank, u.roles,
               u.osu_id < 0 AS dummy,
               EXISTS(SELECT 1 FROM team_members m WHERE m.user_id = u.id) AS in_team,
               EXISTS(SELECT 1 FROM discord_accounts d WHERE d.osu_user_id = u.id)
                   AS discord_linked
        FROM osu_users u
        ORDER BY u.osu_id < 0, cardinality(u.roles) > 0 DESC, u.global_rank ASC NULLS LAST, u.username
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
    let mut tx = pool.begin().await?;
    reseed(&mut tx).await?;
    tx.commit().await
}

// A few teams and the dev staff, nothing played (teams.sql).
pub async fn seed_teams(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    run_seed(&mut tx, TEAMS_SQL).await?;
    tx.commit().await
}

// The regular seed turned into the tournament just after its qualifier (qualifiers.sql).
pub async fn seed_qualifiers(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    reseed(&mut tx).await?;
    tx.execute(QUALIFIERS_SQL).await?;
    tx.commit().await
}

pub enum LiveSeed {
    Done,
    // A dummy account, which the reseed would delete.
    Dummy,
    // The account is in a team that isn't seeded, so it can't join the test team.
    InTeam,
}

// The regular seed plus live.sql's match for `user_id`, a real account.
pub async fn seed_live(pool: &PgPool, user_id: Uuid) -> sqlx::Result<LiveSeed> {
    let mut tx = pool.begin().await?;
    let dummy: bool = sqlx::query_scalar("SELECT osu_id < 0 FROM osu_users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    if dummy {
        return Ok(LiveSeed::Dummy);
    }
    reseed(&mut tx).await?;
    let in_team: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM team_members WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
    if in_team {
        return Ok(LiveSeed::InTeam);
    }
    // live.sql reads it; local to the transaction.
    sqlx::query("SELECT set_config('dev.live_user', $1, true)")
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;
    tx.execute(LIVE_SQL).await?;
    tx.commit().await?;
    Ok(LiveSeed::Done)
}

async fn reseed(conn: &mut PgConnection) -> sqlx::Result<()> {
    run_seed(conn, RESEED_SQL).await
}

// Runs a seed, then gives the seeded teams named in BANNERS their banners.
async fn run_seed(conn: &mut PgConnection, sql: &'static str) -> sqlx::Result<()> {
    // A plain &str runs as a simple query, which may hold several statements.
    conn.execute(sql).await?;
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
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn clear(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::raw_sql(CLEAR_SQL).execute(pool).await?;
    Ok(())
}
