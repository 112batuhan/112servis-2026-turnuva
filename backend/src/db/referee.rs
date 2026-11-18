use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

// What the referee page keeps about a match (match_room_settings), with the defaults
// for a match that has no row yet.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RoomSettings {
    pub score_mode: i16,
    pub no_fail: bool,
    pub auto_place: bool,
    pub auto_kick: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub osu_synced_at: Option<OffsetDateTime>,
    pub next_pick_team_id: Option<Uuid>,
    // The auto referee runs the match (bancho/autoref.rs); false: manual control.
    pub auto_ref: bool,
}

impl Default for RoomSettings {
    fn default() -> Self {
        RoomSettings {
            score_mode: 3,
            no_fail: true,
            auto_place: true,
            auto_kick: true,
            osu_synced_at: None,
            next_pick_team_id: None,
            auto_ref: true,
        }
    }
}

pub async fn find_settings(pool: &PgPool, match_id: Uuid) -> sqlx::Result<RoomSettings> {
    Ok(sqlx::query_as::<_, RoomSettings>(
        r#"
        SELECT score_mode, no_fail, auto_place, auto_kick, osu_synced_at, next_pick_team_id,
               auto_ref
        FROM match_room_settings
        WHERE match_id = $1
        "#,
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default())
}

pub async fn set_options(
    pool: &PgPool,
    match_id: Uuid,
    score_mode: i16,
    no_fail: bool,
    auto_place: bool,
    auto_kick: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO match_room_settings (match_id, score_mode, no_fail, auto_place, auto_kick)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (match_id) DO UPDATE SET
            score_mode = EXCLUDED.score_mode,
            no_fail = EXCLUDED.no_fail,
            auto_place = EXCLUDED.auto_place,
            auto_kick = EXCLUDED.auto_kick
        "#,
    )
    .bind(match_id)
    .bind(score_mode)
    .bind(no_fail)
    .bind(auto_place)
    .bind(auto_kick)
    .execute(pool)
    .await?;
    Ok(())
}

// Records that the referee synced the results from osu!'s record just now, which every
// referee page then reads in place of the chat's maps. Returns the new stamp.
pub async fn mark_osu_synced(pool: &PgPool, match_id: Uuid) -> sqlx::Result<OffsetDateTime> {
    sqlx::query_scalar(
        r#"
        INSERT INTO match_room_settings (match_id, osu_synced_at)
        VALUES ($1, now())
        ON CONFLICT (match_id) DO UPDATE SET osu_synced_at = EXCLUDED.osu_synced_at
        RETURNING osu_synced_at
        "#,
    )
    .bind(match_id)
    .fetch_one(pool)
    .await
}

pub async fn set_next_pick(
    pool: &PgPool,
    match_id: Uuid,
    team_id: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO match_room_settings (match_id, next_pick_team_id)
        VALUES ($1, $2)
        ON CONFLICT (match_id) DO UPDATE SET next_pick_team_id = EXCLUDED.next_pick_team_id
        "#,
    )
    .bind(match_id)
    .bind(team_id)
    .execute(pool)
    .await?;
    Ok(())
}

// A ban or pick was marked or taken back: the rules say whose pick is next again.
pub async fn clear_next_pick(pool: &PgPool, match_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE match_room_settings SET next_pick_team_id = NULL WHERE match_id = $1")
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_ignored(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT map_key FROM match_ignored_maps WHERE match_id = $1 ORDER BY map_key",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

pub async fn ignore_map(pool: &PgPool, match_id: Uuid, key: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO match_ignored_maps (match_id, map_key) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(match_id)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count_map(pool: &PgPool, match_id: Uuid, key: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM match_ignored_maps WHERE match_id = $1 AND map_key = $2")
        .bind(match_id)
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}
