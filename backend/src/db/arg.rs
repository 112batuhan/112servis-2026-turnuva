use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

// The player's place among those who entered the message (1 = first), if they have.
pub async fn place_of(pool: &PgPool, user_id: Uuid, message: i16) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar(
        r#"
        SELECT (SELECT count(*) FROM arg_solves o WHERE o.message = s.message AND o.id <= s.id)
        FROM arg_solves s
        WHERE s.user_id = $1 AND s.message = $2
        "#,
    )
    .bind(user_id)
    .bind(message)
    .fetch_optional(pool)
    .await
}

// How many guesses the player made in the last 10 minutes.
pub async fn recent_attempts(db: impl PgExecutor<'_>, user_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT count(*) FROM arg_attempts WHERE user_id = $1 AND created_at > now() - interval '10 minutes'",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
}

// Held until the transaction ends: a player's guesses are counted and recorded one at a
// time.
pub async fn lock_player(conn: &mut PgConnection, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('arg:' || $1::text))")
        .bind(user_id)
        .execute(conn)
        .await?;
    Ok(())
}

// `message` is the one the guess matched, None for a wrong guess.
pub async fn record_attempt(
    conn: &mut PgConnection,
    user_id: Uuid,
    message: Option<i16>,
    guess: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO arg_attempts (user_id, message, guess, correct) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(message)
    .bind(guess)
    .bind(message.is_some())
    .execute(conn)
    .await?;
    Ok(())
}

// Records that the player entered the message (a no-op if they already had).
pub async fn solve(conn: &mut PgConnection, user_id: Uuid, message: i16) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO arg_solves (user_id, message) VALUES ($1, $2) ON CONFLICT (user_id, message) DO NOTHING",
    )
    .bind(user_id)
    .bind(message)
    .execute(conn)
    .await?;
    Ok(())
}

// Deletes every guess and every message found, for the results page's reset.
pub async fn reset(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM arg_attempts")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM arg_solves")
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

// Every message a player entered, for the results page: by message, then in the order
// they came in.
#[derive(Debug, sqlx::FromRow)]
pub struct SolveRow {
    pub message: i16,
    pub place: i64,
    pub osu_id: i64,
    pub username: String,
    pub discord_username: Option<String>,
    pub staff: bool,
    pub solved_at: OffsetDateTime,
}

pub async fn all_solves(pool: &PgPool) -> sqlx::Result<Vec<SolveRow>> {
    sqlx::query_as::<_, SolveRow>(
        r#"
        SELECT s.message,
               row_number() OVER (PARTITION BY s.message ORDER BY s.id) AS place,
               o.osu_id, o.username, d.username AS discord_username,
               cardinality(o.roles) > 0 AS staff,
               s.solved_at
        FROM arg_solves s
        JOIN osu_users o ON o.id = s.user_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = o.id
        ORDER BY s.message, s.id
        "#,
    )
    .fetch_all(pool)
    .await
}

// Each player who has guessed: whoever entered the winning message first, then the most
// messages found, then whoever found their last one first.
#[derive(Debug, sqlx::FromRow)]
pub struct PlayerRow {
    pub osu_id: i64,
    pub username: String,
    pub discord_username: Option<String>,
    pub staff: bool,
    pub attempts: i64,
    pub first_at: OffsetDateTime,
    pub last_at: OffsetDateTime,
}

pub async fn players(pool: &PgPool) -> sqlx::Result<Vec<PlayerRow>> {
    sqlx::query_as::<_, PlayerRow>(
        r#"
        SELECT o.osu_id, o.username, d.username AS discord_username,
               cardinality(o.roles) > 0 AS staff,
               a.attempts, a.first_at, a.last_at
        FROM (
            SELECT user_id, count(*) AS attempts, min(created_at) AS first_at, max(created_at) AS last_at
            FROM arg_attempts
            GROUP BY user_id
        ) a
        JOIN osu_users o ON o.id = a.user_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = o.id
        ORDER BY (SELECT s.id FROM arg_solves s WHERE s.user_id = o.id AND s.message = 3) NULLS LAST,
                 (SELECT count(*) FROM arg_solves s WHERE s.user_id = o.id) DESC,
                 (SELECT max(s.id) FROM arg_solves s WHERE s.user_id = o.id) NULLS LAST,
                 a.first_at
        "#,
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct WrongGuess {
    pub username: String,
    pub guess: String,
    pub created_at: OffsetDateTime,
}

// The latest wrong guesses, newest first.
pub async fn wrong_guesses(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<WrongGuess>> {
    sqlx::query_as::<_, WrongGuess>(
        r#"
        SELECT o.username, a.guess, a.created_at
        FROM arg_attempts a
        JOIN osu_users o ON o.id = a.user_id
        WHERE NOT a.correct
        ORDER BY a.id DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}
