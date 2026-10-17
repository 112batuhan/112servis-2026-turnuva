use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub sent_at: OffsetDateTime,
    pub sender: String,
    pub text: String,
    pub action: bool,
}

// Saves a line the room's bot saw (or sent) at `sent_at` and returns it as stored, so
// it can be sent on to the match pages.
pub async fn insert_message(
    pool: &PgPool,
    mp_id: i64,
    sender: &str,
    text: &str,
    action: bool,
    bot_id: Uuid,
    sent_at: OffsetDateTime,
) -> sqlx::Result<ChatMessage> {
    sqlx::query_as::<_, ChatMessage>(
        r#"
        INSERT INTO room_chat_messages (mp_id, sender, text, action, bot_id, sent_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, sent_at, sender, text, action
        "#,
    )
    .bind(mp_id)
    .bind(sender)
    .bind(text)
    .bind(action)
    .bind(bot_id)
    .bind(sent_at)
    .fetch_one(pool)
    .await
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
