use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::osu_api::user::OsuUserCompact;

// osu! ids of the given users whose rank was last pulled before `cutoff`. Dummy
// accounts (negative osu_id) are never refreshed.
pub async fn stale_osu_ids(
    pool: &PgPool,
    user_ids: &[Uuid],
    cutoff: OffsetDateTime,
) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        r#"
        SELECT osu_id FROM osu_users
        WHERE id = ANY($1) AND osu_id > 0
          AND (rank_updated_at IS NULL OR rank_updated_at < $2)
        "#,
    )
    .bind(user_ids)
    .bind(cutoff)
    .fetch_all(pool)
    .await
}

pub async fn team_member_osu_ids(pool: &PgPool) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        r#"
        SELECT u.osu_id FROM team_members m
        JOIN osu_users u ON u.id = m.user_id
        WHERE u.osu_id > 0
        "#,
    )
    .fetch_all(pool)
    .await
}

// A user's global rank before and after an update.
#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct RankChange {
    pub user_id: Uuid,
    pub old_rank: Option<i64>,
    pub new_rank: Option<i64>,
}

// Writes the profiles from osu! in one statement and returns every updated user's
// rank before and after (the self-join reads the rows as they were). The bulk
// endpoint has no country rank, so that column is left alone.
pub async fn update_from_osu(
    pool: &PgPool,
    users: &[OsuUserCompact],
) -> sqlx::Result<Vec<RankChange>> {
    let ids: Vec<i64> = users.iter().map(|u| u.id).collect();
    let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
    let avatars: Vec<&str> = users.iter().map(|u| u.avatar_url.as_str()).collect();
    let countries: Vec<Option<&str>> = users.iter().map(|u| u.country_code.as_deref()).collect();
    let ranks: Vec<Option<i64>> = users.iter().map(OsuUserCompact::global_rank).collect();
    sqlx::query_as::<_, RankChange>(
        r#"
        UPDATE osu_users u
        SET username = v.username,
            avatar_url = v.avatar_url,
            country_code = v.country_code,
            global_rank = v.global_rank,
            rank_updated_at = now(),
            updated_at = now()
        FROM UNNEST($1::bigint[], $2::text[], $3::text[], $4::text[], $5::bigint[])
            AS v (osu_id, username, avatar_url, country_code, global_rank),
            osu_users o
        WHERE u.osu_id = v.osu_id AND o.id = u.id
        RETURNING u.id AS user_id, o.global_rank AS old_rank, u.global_rank AS new_rank
        "#,
    )
    .bind(ids)
    .bind(usernames)
    .bind(avatars)
    .bind(countries)
    .bind(ranks)
    .fetch_all(pool)
    .await
}

// A user's id and stored rank, looked up before a login overwrites it.
pub async fn find_by_osu_id(
    pool: &PgPool,
    osu_id: i64,
) -> sqlx::Result<Option<(Uuid, Option<i64>)>> {
    sqlx::query_as("SELECT id, global_rank FROM osu_users WHERE osu_id = $1")
        .bind(osu_id)
        .fetch_optional(pool)
        .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct TeamRankRow {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub global_rank: Option<i64>,
}

// Both members of every team any of the users is in, grouped by team.
pub async fn team_ranks_of(pool: &PgPool, user_ids: &[Uuid]) -> sqlx::Result<Vec<TeamRankRow>> {
    sqlx::query_as::<_, TeamRankRow>(
        r#"
        SELECT m.team_id, m.user_id, u.global_rank
        FROM team_members m
        JOIN osu_users u ON u.id = m.user_id
        WHERE m.team_id IN (SELECT team_id FROM team_members WHERE user_id = ANY($1))
        ORDER BY m.team_id, m.joined_at, m.user_id
        "#,
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await
}
