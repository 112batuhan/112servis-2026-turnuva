use sqlx::{PgExecutor, PgPool};
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

// FOR SHARE makes a concurrent freeze wait for a transaction that read the flag,
// so no team can form after the freeze commits.
pub async fn teams_frozen(ex: impl PgExecutor<'_>) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT teams_frozen FROM app_settings FOR SHARE")
        .fetch_one(ex)
        .await
}

pub async fn set_teams_frozen(pool: &PgPool, frozen: bool) -> sqlx::Result<()> {
    sqlx::query("UPDATE app_settings SET teams_frozen = $1")
        .bind(frozen)
        .execute(pool)
        .await?;
    Ok(())
}

// Whether the auto referee opens match rooms by itself (bancho/manager.rs).
pub async fn auto_rooms(pool: &PgPool) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT auto_rooms FROM app_settings")
        .fetch_one(pool)
        .await
}

pub async fn set_auto_rooms(pool: &PgPool, enabled: bool) -> sqlx::Result<()> {
    sqlx::query("UPDATE app_settings SET auto_rooms = $1")
        .bind(enabled)
        .execute(pool)
        .await?;
    Ok(())
}
