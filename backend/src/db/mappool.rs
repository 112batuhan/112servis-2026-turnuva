use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

// ---------- rows ----------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Stage {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub published: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// A category is just a named group now — mods live on the maps, not the category.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub stage_id: Uuid,
    pub name: String,
    pub position: i32,
}

// A map in the pool: a beatmap locked to a mod combination with the stats it had when
// added. `category_id` NULL means it's in the (global) generic pool; otherwise it's in
// that category. Metadata (title, artist, ...) is joined from the cached beatmap.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PoolMap {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub position: i32,
    pub mods: String,
    pub beatmap_id: i64,
    pub beatmapset_id: i64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub creator: Option<String>,
    pub star_rating: f64,
    pub bpm: f64,
    pub total_length: i32,
    pub cs: f64,
    pub ar: f64,
    pub od: f64,
    pub hp: f64,
    pub mode: String,
    pub cover_url: Option<String>,
}

// The mod-adjusted stats locked onto a map when it's added to the pool.
pub struct ModStats {
    pub star_rating: f64,
    pub ar: f64,
    pub od: f64,
    pub cs: f64,
    pub hp: f64,
    pub bpm: f64,
    pub total_length: i32,
}

// A beatmap's cached nomod stats, used to compute mod-adjusted CS/HP/BPM/length.
#[derive(Debug, sqlx::FromRow)]
pub struct Beatmap {
    pub id: i64,
    pub star_rating: f64,
    pub bpm: f64,
    pub total_length: i32,
    pub cs: f64,
    pub ar: f64,
    pub od: f64,
    pub hp: f64,
}

// Everything the editor needs for one stage: its categories, plus every relevant map
// (the global generic pool + this stage's categorised maps).
#[derive(Debug, Serialize)]
pub struct StageDetail {
    #[serde(flatten)]
    pub stage: Stage,
    pub categories: Vec<Category>,
    pub maps: Vec<PoolMap>,
}

// Read-only view of a published stage for the public page: categories + their maps,
// no generic pool.
#[derive(Debug, Serialize)]
pub struct PublicStageDetail {
    #[serde(flatten)]
    pub stage: Stage,
    pub categories: Vec<Category>,
    pub maps: Vec<PoolMap>,
}

// Beatmap fields to cache, mapped from an osu! API response by the handler.
pub struct NewBeatmap {
    pub id: i64,
    pub beatmapset_id: i64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub creator: Option<String>,
    pub star_rating: f64,
    pub bpm: f64,
    pub total_length: i32,
    pub cs: f64,
    pub ar: f64,
    pub od: f64,
    pub hp: f64,
    pub mode: String,
    pub cover_url: Option<String>,
}

// ---------- stages ----------

pub async fn list_stages(pool: &PgPool) -> sqlx::Result<Vec<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages ORDER BY position, created_at")
        .fetch_all(pool)
        .await
}

pub async fn create_stage(pool: &PgPool, name: &str) -> sqlx::Result<Stage> {
    sqlx::query_as::<_, Stage>(
        r#"
        INSERT INTO stages (name, position)
        VALUES ($1, (SELECT COALESCE(MAX(position), -1) + 1 FROM stages))
        RETURNING *
        "#,
    )
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn get_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM stages WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// Publishes or unpublishes a stage. Published stages are visible on the public page.
pub async fn set_published(pool: &PgPool, id: Uuid, published: bool) -> sqlx::Result<Stage> {
    sqlx::query_as::<_, Stage>(
        "UPDATE stages SET published = $2, updated_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(published)
    .fetch_one(pool)
    .await
}

pub async fn list_published_stages(pool: &PgPool) -> sqlx::Result<Vec<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE published ORDER BY position, created_at")
        .fetch_all(pool)
        .await
}

pub async fn get_published_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE id = $1 AND published")
        .bind(id)
        .fetch_optional(pool)
        .await
}

// ---------- categories ----------

pub async fn list_categories(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<Category>> {
    sqlx::query_as::<_, Category>(
        "SELECT id, stage_id, name, position FROM stage_categories WHERE stage_id = $1 ORDER BY position, created_at",
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

pub async fn create_category(pool: &PgPool, stage_id: Uuid, name: &str) -> sqlx::Result<Category> {
    sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO stage_categories (stage_id, name, position)
        VALUES ($1, $2, (SELECT COALESCE(MAX(position), -1) + 1 FROM stage_categories WHERE stage_id = $1))
        RETURNING id, stage_id, name, position
        "#,
    )
    .bind(stage_id)
    .bind(name)
    .fetch_one(pool)
    .await
}

// Deletes a category. Its maps fall back to the generic pool via the ON DELETE SET
// NULL foreign key, so they aren't lost.
pub async fn delete_category(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM stage_categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- beatmaps ----------

pub async fn upsert_beatmap(pool: &PgPool, b: &NewBeatmap) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO beatmaps (id, beatmapset_id, artist, title, version, creator,
                              star_rating, bpm, total_length, cs, ar, od, hp, mode, cover_url, fetched_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, now())
        ON CONFLICT (id) DO UPDATE
        SET beatmapset_id = EXCLUDED.beatmapset_id, artist = EXCLUDED.artist, title = EXCLUDED.title,
            version = EXCLUDED.version, creator = EXCLUDED.creator, star_rating = EXCLUDED.star_rating,
            bpm = EXCLUDED.bpm, total_length = EXCLUDED.total_length, cs = EXCLUDED.cs, ar = EXCLUDED.ar,
            od = EXCLUDED.od, hp = EXCLUDED.hp, mode = EXCLUDED.mode, cover_url = EXCLUDED.cover_url,
            fetched_at = now()
        "#,
    )
    .bind(b.id)
    .bind(b.beatmapset_id)
    .bind(&b.artist)
    .bind(&b.title)
    .bind(&b.version)
    .bind(&b.creator)
    .bind(b.star_rating)
    .bind(b.bpm)
    .bind(b.total_length)
    .bind(b.cs)
    .bind(b.ar)
    .bind(b.od)
    .bind(b.hp)
    .bind(&b.mode)
    .bind(&b.cover_url)
    .execute(pool)
    .await?;
    Ok(())
}

// The cached nomod stats for a beatmap (for computing mod-adjusted stats).
pub async fn get_beatmap(pool: &PgPool, id: i64) -> sqlx::Result<Option<Beatmap>> {
    sqlx::query_as::<_, Beatmap>(
        "SELECT id, star_rating, bpm, total_length, cs, ar, od, hp FROM beatmaps WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

// ---------- pool maps ----------

const POOL_MAP_COLS: &str = "pm.id, pm.category_id, pm.position, pm.mods, \
     b.id AS beatmap_id, b.beatmapset_id, b.artist, b.title, b.version, b.creator, b.mode, b.cover_url, \
     pm.star_rating, pm.bpm, pm.total_length, pm.cs, pm.ar, pm.od, pm.hp";

// Adds a map (beatmap + mods with locked stats) to the generic pool. Returns its id.
pub async fn add_pool_map(
    pool: &PgPool,
    beatmap_id: i64,
    mods: &str,
    stats: &ModStats,
    added_by: Uuid,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO pool_maps
            (beatmap_id, mods, added_by, position,
             star_rating, ar, od, cs, hp, bpm, total_length)
        VALUES ($1, $2, $3,
                (SELECT COALESCE(MAX(position), -1) + 1 FROM pool_maps WHERE category_id IS NULL),
                $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
    )
    .bind(beatmap_id)
    .bind(mods)
    .bind(added_by)
    .bind(stats.star_rating)
    .bind(stats.ar)
    .bind(stats.od)
    .bind(stats.cs)
    .bind(stats.hp)
    .bind(stats.bpm)
    .bind(stats.total_length)
    .fetch_one(pool)
    .await
}

pub async fn get_pool_map(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<PoolMap>> {
    let sql =
        format!("SELECT {POOL_MAP_COLS} FROM pool_maps pm JOIN beatmaps b ON b.id = pm.beatmap_id WHERE pm.id = $1");
    sqlx::query_as::<_, PoolMap>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await
}

// Maps the editor shows for a stage: the global generic pool (category_id NULL) plus
// this stage's categorised maps.
pub async fn list_stage_maps(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<PoolMap>> {
    let sql = format!(
        "SELECT {POOL_MAP_COLS} FROM pool_maps pm JOIN beatmaps b ON b.id = pm.beatmap_id \
         WHERE pm.category_id IS NULL \
            OR pm.category_id IN (SELECT id FROM stage_categories WHERE stage_id = $1) \
         ORDER BY pm.category_id NULLS FIRST, pm.position, pm.created_at"
    );
    sqlx::query_as::<_, PoolMap>(&sql)
        .bind(stage_id)
        .fetch_all(pool)
        .await
}

// Maps the public page shows for a (published) stage: only its categorised maps.
pub async fn list_public_maps(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<PoolMap>> {
    let sql = format!(
        "SELECT {POOL_MAP_COLS} FROM pool_maps pm JOIN beatmaps b ON b.id = pm.beatmap_id \
         WHERE pm.category_id IN (SELECT id FROM stage_categories WHERE stage_id = $1) \
         ORDER BY pm.position, pm.created_at"
    );
    sqlx::query_as::<_, PoolMap>(&sql)
        .bind(stage_id)
        .fetch_all(pool)
        .await
}

// Moves a map into a category, or back to the generic pool (None), appended at the end.
pub async fn move_pool_map(pool: &PgPool, id: Uuid, category_id: Option<Uuid>) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE pool_maps
        SET category_id = $2,
            position = COALESCE(
                (SELECT MAX(position) + 1 FROM pool_maps WHERE category_id IS NOT DISTINCT FROM $2),
                0
            )
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(category_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_pool_map(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM pool_maps WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
