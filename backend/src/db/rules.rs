use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

// The rules page: one row of osu! BBCode, plus who last saved it and when.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Rules {
    pub bbcode: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub updated_by: Option<String>,
}

pub async fn get(pool: &PgPool) -> sqlx::Result<Rules> {
    sqlx::query_as::<_, Rules>(
        r#"
        SELECT r.bbcode, r.updated_at, o.username AS updated_by
        FROM rules r
        LEFT JOIN osu_users o ON o.id = r.updated_by
        "#,
    )
    .fetch_one(pool)
    .await
}

// Saves only if nobody has saved since `base` (the version the editor loaded); None
// means someone has.
pub async fn save(
    pool: &PgPool,
    bbcode: &str,
    base: OffsetDateTime,
    user_id: Uuid,
) -> sqlx::Result<Option<Rules>> {
    sqlx::query_as::<_, Rules>(
        r#"
        WITH saved AS (
            UPDATE rules SET bbcode = $1, updated_at = now(), updated_by = $3
            WHERE updated_at = $2
            RETURNING bbcode, updated_at, updated_by
        )
        SELECT s.bbcode, s.updated_at, o.username AS updated_by
        FROM saved s
        LEFT JOIN osu_users o ON o.id = s.updated_by
        "#,
    )
    .bind(bbcode)
    .bind(base)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
