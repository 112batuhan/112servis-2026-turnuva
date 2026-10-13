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

// Writes the profiles from osu! in one statement. The bulk endpoint has no
// country rank, so that column is left alone.
pub async fn update_from_osu(pool: &PgPool, users: &[OsuUserCompact]) -> sqlx::Result<u64> {
    let ids: Vec<i64> = users.iter().map(|u| u.id).collect();
    let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
    let avatars: Vec<&str> = users.iter().map(|u| u.avatar_url.as_str()).collect();
    let countries: Vec<Option<&str>> = users.iter().map(|u| u.country_code.as_deref()).collect();
    let ranks: Vec<Option<i64>> = users.iter().map(OsuUserCompact::global_rank).collect();
    let result = sqlx::query(
        r#"
        UPDATE osu_users u
        SET username = v.username,
            avatar_url = v.avatar_url,
            country_code = v.country_code,
            global_rank = v.global_rank,
            rank_updated_at = now(),
            updated_at = now()
        FROM UNNEST($1::bigint[], $2::text[], $3::text[], $4::text[], $5::bigint[])
            AS v (osu_id, username, avatar_url, country_code, global_rank)
        WHERE u.osu_id = v.osu_id
        "#,
    )
    .bind(ids)
    .bind(usernames)
    .bind(avatars)
    .bind(countries)
    .bind(ranks)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
