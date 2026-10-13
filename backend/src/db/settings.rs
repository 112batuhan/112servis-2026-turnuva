use sqlx::PgPool;
use time::OffsetDateTime;

pub async fn ranks_updated_at(pool: &PgPool) -> sqlx::Result<Option<OffsetDateTime>> {
    sqlx::query_scalar("SELECT ranks_updated_at FROM app_settings")
        .fetch_one(pool)
        .await
}

pub async fn touch_ranks_updated_at(pool: &PgPool) -> sqlx::Result<OffsetDateTime> {
    sqlx::query_scalar(
        "UPDATE app_settings SET ranks_updated_at = now() RETURNING ranks_updated_at",
    )
    .fetch_one(pool)
    .await
}
