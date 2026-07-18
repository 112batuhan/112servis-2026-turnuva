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

// A category is just a named group now — mods live on the maps, not the category. Its
// size is the number of slots it has (see Slot). `color` colour-codes it in the pool.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub stage_id: Uuid,
    pub name: String,
    pub position: i32,
    pub color: String,
}

// A slot in a category, with editor-only planning notes. Grouped by category_id on
// the frontend.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Slot {
    pub id: Uuid,
    pub category_id: Uuid,
    pub position: i32,
    pub editor_notes: String,
}

// Public view of a slot — the same, minus the editor notes.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicSlot {
    pub id: Uuid,
    pub category_id: Uuid,
    pub position: i32,
}

// A map in the pool: a beatmap locked to a mod combination with the stats it had when
// added. `slot_id` NULL means it's in the (global) generic pool; otherwise it fills
// that slot. Metadata (title, artist, ...) is joined from the cached beatmap.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PoolMap {
    pub id: Uuid,
    pub slot_id: Option<Uuid>,
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
    pub public_notes: String,
    pub editor_notes: String,
}

// Public view of a pool map — the same, minus the editor notes.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicPoolMap {
    pub id: Uuid,
    pub slot_id: Option<Uuid>,
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
    pub public_notes: String,
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

// A new map to store in the pool: osu! metadata (from the API) + its locked mod stats.
pub struct NewPoolMap {
    pub beatmap_id: i64,
    pub beatmapset_id: i64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub creator: Option<String>,
    pub mode: String,
    pub cover_url: Option<String>,
    pub mods: String,
    pub stats: ModStats,
}

// Everything the editor needs for one stage: its categories, plus every relevant map
// (the global generic pool + this stage's categorised maps).
#[derive(Debug, Serialize)]
pub struct StageDetail {
    #[serde(flatten)]
    pub stage: Stage,
    pub categories: Vec<Category>,
    pub slots: Vec<Slot>,
    pub maps: Vec<PoolMap>,
}

// Read-only view of a published stage for the public page: categories + their slots
// (no editor notes) + their maps (no editor notes), no generic pool.
#[derive(Debug, Serialize)]
pub struct PublicStageDetail {
    #[serde(flatten)]
    pub stage: Stage,
    pub categories: Vec<Category>,
    pub slots: Vec<PublicSlot>,
    pub maps: Vec<PublicPoolMap>,
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

// Updates a stage's name and/or published flag; each field changes only when Some(..).
// Published stages are visible on the public page.
pub async fn update_stage(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    published: Option<bool>,
) -> sqlx::Result<Stage> {
    sqlx::query_as::<_, Stage>(
        "UPDATE stages SET name = COALESCE($2, name), published = COALESCE($3, published), \
         updated_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(name)
    .bind(published)
    .fetch_one(pool)
    .await
}

// Sets each stage's position to its index in `ids` (one statement via WITH ORDINALITY).
pub async fn reorder_stages(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE stages s SET position = v.ord - 1, updated_at = now() \
         FROM unnest($1::uuid[]) WITH ORDINALITY AS v(id, ord) WHERE s.id = v.id",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(())
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
        "SELECT id, stage_id, name, position, color FROM stage_categories WHERE stage_id = $1 ORDER BY position, created_at",
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
        RETURNING id, stage_id, name, position, color
        "#,
    )
    .bind(stage_id)
    .bind(name)
    .fetch_one(pool)
    .await
}

// Updates a category's name and/or colour; each field changes only when Some(..).
pub async fn update_category(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    color: Option<&str>,
) -> sqlx::Result<Category> {
    sqlx::query_as::<_, Category>(
        "UPDATE stage_categories SET name = COALESCE($2, name), color = COALESCE($3, color), \
         updated_at = now() WHERE id = $1 RETURNING id, stage_id, name, position, color",
    )
    .bind(id)
    .bind(name)
    .bind(color)
    .fetch_one(pool)
    .await
}

// Sets each category's position to its index in `ids` (see reorder_stages).
pub async fn reorder_categories(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE stage_categories c SET position = v.ord - 1, updated_at = now() \
         FROM unnest($1::uuid[]) WITH ORDINALITY AS v(id, ord) WHERE c.id = v.id",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(())
}

// Deletes a category. Its maps fall back to the generic pool via the ON DELETE SET
// NULL foreign key, and its slots are removed via ON DELETE CASCADE.
pub async fn delete_category(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM stage_categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- category slots ----------

// Slots for every category in a stage (editors — includes notes).
pub async fn list_slots(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<Slot>> {
    sqlx::query_as::<_, Slot>(
        "SELECT s.id, s.category_id, s.position, s.editor_notes FROM category_slots s \
         JOIN stage_categories c ON c.id = s.category_id \
         WHERE c.stage_id = $1 ORDER BY s.position, s.created_at",
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

// Slots for every category in a stage (public — no notes).
pub async fn list_public_slots(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<PublicSlot>> {
    sqlx::query_as::<_, PublicSlot>(
        "SELECT s.id, s.category_id, s.position FROM category_slots s \
         JOIN stage_categories c ON c.id = s.category_id \
         WHERE c.stage_id = $1 ORDER BY s.position, s.created_at",
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

pub async fn add_slot(pool: &PgPool, category_id: Uuid) -> sqlx::Result<Slot> {
    sqlx::query_as::<_, Slot>(
        r#"
        INSERT INTO category_slots (category_id, position)
        VALUES ($1, (SELECT COALESCE(MAX(position), -1) + 1 FROM category_slots WHERE category_id = $1))
        RETURNING id, category_id, position, editor_notes
        "#,
    )
    .bind(category_id)
    .fetch_one(pool)
    .await
}

pub async fn update_slot_notes(pool: &PgPool, id: Uuid, notes: &str) -> sqlx::Result<Slot> {
    sqlx::query_as::<_, Slot>(
        "UPDATE category_slots SET editor_notes = $2, updated_at = now() \
         WHERE id = $1 RETURNING id, category_id, position, editor_notes",
    )
    .bind(id)
    .bind(notes)
    .fetch_one(pool)
    .await
}

pub async fn delete_slot(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM category_slots WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- pool maps ----------

const POOL_MAP_COLS: &str = "pm.id, pm.slot_id, pm.position, pm.mods, \
     pm.beatmap_id, pm.beatmapset_id, pm.artist, pm.title, pm.version, pm.creator, pm.mode, pm.cover_url, \
     pm.star_rating, pm.bpm, pm.total_length, pm.cs, pm.ar, pm.od, pm.hp, pm.public_notes, pm.editor_notes";

// Public column list: same as above but without editor_notes.
const PUBLIC_MAP_COLS: &str = "pm.id, pm.slot_id, pm.position, pm.mods, \
     pm.beatmap_id, pm.beatmapset_id, pm.artist, pm.title, pm.version, pm.creator, pm.mode, pm.cover_url, \
     pm.star_rating, pm.bpm, pm.total_length, pm.cs, pm.ar, pm.od, pm.hp, pm.public_notes";

// Adds a map (osu! metadata + mods with locked stats) to the generic pool. Returns its id.
pub async fn add_pool_map(pool: &PgPool, m: &NewPoolMap, added_by: Uuid) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO pool_maps
            (beatmap_id, beatmapset_id, artist, title, version, creator, mode, cover_url,
             mods, added_by, position,
             star_rating, ar, od, cs, hp, bpm, total_length)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                (SELECT COALESCE(MAX(position), -1) + 1 FROM pool_maps WHERE slot_id IS NULL),
                $11, $12, $13, $14, $15, $16, $17)
        RETURNING id
        "#,
    )
    .bind(m.beatmap_id)
    .bind(m.beatmapset_id)
    .bind(&m.artist)
    .bind(&m.title)
    .bind(&m.version)
    .bind(&m.creator)
    .bind(&m.mode)
    .bind(&m.cover_url)
    .bind(&m.mods)
    .bind(added_by)
    .bind(m.stats.star_rating)
    .bind(m.stats.ar)
    .bind(m.stats.od)
    .bind(m.stats.cs)
    .bind(m.stats.hp)
    .bind(m.stats.bpm)
    .bind(m.stats.total_length)
    .fetch_one(pool)
    .await
}

pub async fn get_pool_map(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<PoolMap>> {
    let sql = format!("SELECT {POOL_MAP_COLS} FROM pool_maps pm WHERE pm.id = $1");
    sqlx::query_as::<_, PoolMap>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await
}

// Maps the editor shows for a stage: the global generic pool (slot_id NULL) plus the
// maps assigned to this stage's slots.
pub async fn list_stage_maps(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<PoolMap>> {
    let sql = format!(
        "SELECT {POOL_MAP_COLS} FROM pool_maps pm \
         WHERE pm.slot_id IS NULL OR pm.slot_id IN ( \
             SELECT s.id FROM category_slots s JOIN stage_categories c ON c.id = s.category_id \
             WHERE c.stage_id = $1) \
         ORDER BY pm.slot_id NULLS FIRST, pm.position, pm.created_at"
    );
    sqlx::query_as::<_, PoolMap>(&sql)
        .bind(stage_id)
        .fetch_all(pool)
        .await
}

// Maps the public page shows for a (published) stage: only the ones assigned to slots,
// and without editor notes.
pub async fn list_public_maps(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<PublicPoolMap>> {
    let sql = format!(
        "SELECT {PUBLIC_MAP_COLS} FROM pool_maps pm \
         WHERE pm.slot_id IN ( \
             SELECT s.id FROM category_slots s JOIN stage_categories c ON c.id = s.category_id \
             WHERE c.stage_id = $1) \
         ORDER BY pm.position, pm.created_at"
    );
    sqlx::query_as::<_, PublicPoolMap>(&sql)
        .bind(stage_id)
        .fetch_all(pool)
        .await
}

// Updates a map's notes; each field is only changed when Some(..) is passed.
pub async fn update_map_notes(
    pool: &PgPool,
    id: Uuid,
    public_notes: Option<&str>,
    editor_notes: Option<&str>,
) -> sqlx::Result<PoolMap> {
    sqlx::query("UPDATE pool_maps SET public_notes = COALESCE($2, public_notes), editor_notes = COALESCE($3, editor_notes) WHERE id = $1")
        .bind(id)
        .bind(public_notes)
        .bind(editor_notes)
        .execute(pool)
        .await?;
    get_pool_map(pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

// Assigns a map to a slot, or back to the generic pool (None). Since a slot holds at
// most one map, assigning first evicts whatever occupied the target slot back to the
// generic pool.
pub async fn move_pool_map(pool: &PgPool, id: Uuid, slot_id: Option<Uuid>) -> sqlx::Result<()> {
    if let Some(target) = slot_id {
        sqlx::query("UPDATE pool_maps SET slot_id = NULL WHERE slot_id = $1 AND id <> $2")
            .bind(target)
            .bind(id)
            .execute(pool)
            .await?;
    }
    sqlx::query(
        r#"
        UPDATE pool_maps
        SET slot_id = $2,
            position = COALESCE((SELECT MAX(position) + 1 FROM pool_maps WHERE slot_id IS NULL), 0)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(slot_id)
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
