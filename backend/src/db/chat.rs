use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub sent_at: OffsetDateTime,
    pub sender: String,
    pub text: String,
    pub action: bool,
}

pub async fn insert_message(
    pool: &PgPool,
    mp_id: i64,
    sender: &str,
    text: &str,
    action: bool,
    logged_by: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO room_chat_messages (mp_id, sender, text, action, logged_by)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(mp_id)
    .bind(sender)
    .bind(text)
    .bind(action)
    .bind(logged_by)
    .execute(pool)
    .await?;
    Ok(())
}

// A room's whole chat, oldest first.
pub async fn list_messages(pool: &PgPool, mp_id: i64) -> sqlx::Result<Vec<ChatMessage>> {
    sqlx::query_as::<_, ChatMessage>(
        r#"
        SELECT id, sent_at, sender, text, action
        FROM room_chat_messages
        WHERE mp_id = $1
        ORDER BY sent_at, id
        "#,
    )
    .bind(mp_id)
    .fetch_all(pool)
    .await
}
