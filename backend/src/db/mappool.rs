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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub stage_id: Uuid,
    pub name: String,
    pub modifier: Option<String>,
    pub position: i32,
}

// A beatmap in the global generic pool (the shared library, no stage/category).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GenericEntry {
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

// A categorised placement of a beatmap within one stage, joined with its beatmap.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PoolEntry {
    pub id: Uuid,
    pub category_id: Uuid,
    pub position: i32,
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

// Everything the map pool page needs for one stage: its categories, the categorised
// entries, and the shared generic pool minus whatever is already categorised here.
#[derive(Debug, Serialize)]
pub struct StageDetail {
    #[serde(flatten)]
    pub stage: Stage,
    pub categories: Vec<Category>,
    pub entries: Vec<PoolEntry>,
    pub generic: Vec<GenericEntry>,
}

// Read-only view of a published stage for the public page: categories + their maps,
// no generic pool.
#[derive(Debug, Serialize)]
pub struct PublicStageDetail {
    #[serde(flatten)]
    pub stage: Stage,
    pub categories: Vec<Category>,
    pub entries: Vec<PoolEntry>,
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

// Published stages only (for the public page).
pub async fn list_published_stages(pool: &PgPool) -> sqlx::Result<Vec<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE published ORDER BY position, created_at")
        .fetch_all(pool)
        .await
}

// A single stage, only if it's published.
pub async fn get_published_stage(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Stage>> {
    sqlx::query_as::<_, Stage>("SELECT * FROM stages WHERE id = $1 AND published")
        .bind(id)
        .fetch_optional(pool)
        .await
}

// ---------- categories ----------

pub async fn list_categories(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<Category>> {
    sqlx::query_as::<_, Category>(
        "SELECT id, stage_id, name, modifier, position FROM stage_categories WHERE stage_id = $1 ORDER BY position, created_at",
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

pub async fn create_category(
    pool: &PgPool,
    stage_id: Uuid,
    name: &str,
    modifier: Option<&str>,
) -> sqlx::Result<Category> {
    sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO stage_categories (stage_id, name, modifier, position)
        VALUES ($1, $2, $3, (SELECT COALESCE(MAX(position), -1) + 1 FROM stage_categories WHERE stage_id = $1))
        RETURNING id, stage_id, name, modifier, position
        "#,
    )
    .bind(stage_id)
    .bind(name)
    .bind(modifier)
    .fetch_one(pool)
    .await
}

// Deletes a category; its entries return to the generic pool (their maps are still
// in the global library, just no longer categorised in this stage).
pub async fn delete_category(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM pool_entries WHERE category_id = $1")
        .bind(id)
        .execute(pool)
        .await?;
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

// ---------- generic pool (global library) ----------

const BEATMAP_COLS: &str =
    "b.id AS beatmap_id, b.beatmapset_id, b.artist, b.title, b.version, b.creator, \
     b.star_rating, b.bpm, b.total_length, b.cs, b.ar, b.od, b.hp, b.mode, b.cover_url";

pub async fn add_to_generic(pool: &PgPool, beatmap_id: i64, added_by: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO generic_pool (beatmap_id, added_by) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(beatmap_id)
    .bind(added_by)
    .execute(pool)
    .await?;
    Ok(())
}

// Removes a beatmap from the library and from every stage it was categorised in.
pub async fn remove_from_generic(pool: &PgPool, beatmap_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM pool_entries WHERE beatmap_id = $1")
        .bind(beatmap_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM generic_pool WHERE beatmap_id = $1")
        .bind(beatmap_id)
        .execute(pool)
        .await?;
    Ok(())
}

// The generic pool: library maps not placed in any category, in any stage. A map
// lives in exactly one place, so once it's categorised anywhere it disappears from
// the generic pool in every stage. This is global, hence no stage parameter.
pub async fn list_generic(pool: &PgPool) -> sqlx::Result<Vec<GenericEntry>> {
    let sql = format!(
        "SELECT {BEATMAP_COLS} FROM generic_pool g JOIN beatmaps b ON b.id = g.beatmap_id \
         WHERE NOT EXISTS (SELECT 1 FROM pool_entries pe WHERE pe.beatmap_id = g.beatmap_id) \
         ORDER BY g.created_at"
    );
    sqlx::query_as::<_, GenericEntry>(&sql)
        .fetch_all(pool)
        .await
}

// ---------- pool entries (categorised placements) ----------

pub async fn list_entries(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<PoolEntry>> {
    let sql = format!(
        "SELECT pe.id, pe.category_id, pe.position, {BEATMAP_COLS} \
         FROM pool_entries pe JOIN beatmaps b ON b.id = pe.beatmap_id \
         WHERE pe.stage_id = $1 AND pe.category_id IS NOT NULL ORDER BY pe.position, pe.created_at"
    );
    sqlx::query_as::<_, PoolEntry>(&sql)
        .bind(stage_id)
        .fetch_all(pool)
        .await
}

pub async fn get_entry(pool: &PgPool, entry_id: Uuid) -> sqlx::Result<Option<PoolEntry>> {
    let sql = format!(
        "SELECT pe.id, pe.category_id, pe.position, {BEATMAP_COLS} \
         FROM pool_entries pe JOIN beatmaps b ON b.id = pe.beatmap_id \
         WHERE pe.id = $1 AND pe.category_id IS NOT NULL"
    );
    sqlx::query_as::<_, PoolEntry>(&sql)
        .bind(entry_id)
        .fetch_optional(pool)
        .await
}

// Places a generic-pool map into a stage category. A map can be placed in only one
// category tournament-wide, so if it's somehow already placed elsewhere (e.g. two
// poolers dropping it at once) this moves it. Returns the entry id.
pub async fn add_entry(
    pool: &PgPool,
    stage_id: Uuid,
    beatmap_id: i64,
    category_id: Uuid,
    added_by: Uuid,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO pool_entries (stage_id, beatmap_id, category_id, added_by, position)
        VALUES ($1, $2, $3, $4,
                (SELECT COALESCE(MAX(position), -1) + 1 FROM pool_entries WHERE stage_id = $1 AND category_id = $3))
        ON CONFLICT (beatmap_id) DO UPDATE
        SET stage_id = EXCLUDED.stage_id, category_id = EXCLUDED.category_id, position = EXCLUDED.position
        RETURNING id
        "#,
    )
    .bind(stage_id)
    .bind(beatmap_id)
    .bind(category_id)
    .bind(added_by)
    .fetch_one(pool)
    .await
}

// Moves an entry to another category, appended at the end.
pub async fn move_entry(pool: &PgPool, entry_id: Uuid, category_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE pool_entries pe
        SET category_id = $2,
            position = COALESCE(
                (SELECT MAX(position) + 1 FROM pool_entries WHERE stage_id = pe.stage_id AND category_id = $2),
                0
            )
        WHERE pe.id = $1
        "#,
    )
    .bind(entry_id)
    .bind(category_id)
    .execute(pool)
    .await?;
    Ok(())
}

// Uncategorises an entry — the map returns to this stage's generic view (it's still
// in the global library).
pub async fn delete_entry(pool: &PgPool, entry_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM pool_entries WHERE id = $1")
        .bind(entry_id)
        .execute(pool)
        .await?;
    Ok(())
}
