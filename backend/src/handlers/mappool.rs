use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::ReorderBody;
use crate::{auth::AuthUser, db, error::AppError, osu_api, role::Role, AppState};

// Routes under the `/api` nest: the stages' map pools. The authed endpoints require
// map_pooler+, except that pool testers and custom mappers can also read the stages
// (`view_guard`) but change nothing. Managing the stages themselves is in
// handlers/stages.rs. The two `/public/*` ones take no AuthUser and only expose
// published stages.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stages", get(list_stages))
        .route("/stages/:id", get(get_stage))
        .route("/stages/:id/pack", put(set_stage_pack_url))
        .route("/stages/:id/categories", post(create_category))
        .route("/categories/reorder", post(reorder_categories))
        .route(
            "/categories/:id",
            delete(delete_category).patch(update_category),
        )
        .route("/categories/:id/slots", post(add_slot))
        .route("/slots/:id", patch(update_slot).delete(delete_slot))
        .route("/pool", post(add_map))
        .route("/maps/:id", patch(move_map).delete(delete_map))
        .route("/maps/:id/notes", patch(update_map_notes))
        .route("/maps/:id/stats", post(recompute_map_stats))
        .route("/public/stages", get(list_public_stages))
        .route("/public/stages/:id", get(get_public_stage))
}

// Every authed map pool endpoint that changes something requires the map_pooler role
// (host qualifies too).
fn guard(user: &AuthUser) -> Result<(), AppError> {
    user.require_role(Role::MapPooler)
}

// Reading the editor's view (unpublished stages, editor notes) is also open to pool
// testers and custom mappers.
fn view_guard(user: &AuthUser) -> Result<(), AppError> {
    if user.roles.can_view_pool() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

// ---------- stages ----------

async fn list_stages(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    view_guard(&user)?;
    Ok(Json(db::stages::list_stages(&state.db).await?))
}

// GET /api/stages/:id — a stage with its categories and every relevant map (the
// global generic pool plus this stage's categorised maps).
async fn get_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    view_guard(&user)?;
    let stage = db::stages::get_stage(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let categories = db::mappool::list_categories(&state.db, id).await?;
    let slots = db::mappool::list_slots(&state.db, id).await?;
    let maps = db::mappool::list_stage_maps(&state.db, id).await?;
    Ok(Json(db::mappool::StageDetail {
        stage,
        categories,
        slots,
        maps,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PackUrlBody {
    pack_url: Option<String>,
}

// The longest pack link kept; the column allows the same.
const MAX_PACK_URL: usize = 2048;

// PUT /api/stages/:id/pack — map poolers. The link to the stage's beatmap pack, hosted
// elsewhere; empty or null takes it off. The public mappool page offers it.
async fn set_stage_pack_url(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PackUrlBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let url = match body.pack_url.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => {
            let parsed = reqwest::Url::parse(text)
                .map_err(|_| AppError::BadRequest("the pack link isn't a valid address"))?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err(AppError::BadRequest(
                    "the pack link must start with http:// or https://",
                ));
            }
            if text.len() > MAX_PACK_URL {
                return Err(AppError::BadRequest("the pack link is too long"));
            }
            Some(text)
        }
    };
    if !db::stages::set_stage_pack_url(&state.db, id, url).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- categories ----------

#[derive(Debug, Deserialize)]
pub struct CreateCategoryBody {
    name: String,
}

async fn create_category(
    State(state): State<AppState>,
    user: AuthUser,
    Path(stage_id): Path<Uuid>,
    Json(body): Json<CreateCategoryBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("category name is required"));
    }
    Ok(Json(
        db::mappool::create_category(&state.db, stage_id, name).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryBody {
    // Each field is optional — only the ones present are changed.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    color: Option<String>,
}

// POST /api/categories/reorder — persist a new category order.
async fn reorder_categories(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ReorderBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::reorder_categories(&state.db, &body.ids).await?;
    Ok(StatusCode::NO_CONTENT)
}

// PATCH /api/categories/:id — rename and/or recolour a category.
async fn update_category(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCategoryBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let name = body.name.as_deref().map(str::trim);
    if let Some(n) = name {
        if n.is_empty() {
            return Err(AppError::BadRequest("category name is required"));
        }
    }
    Ok(Json(
        db::mappool::update_category(&state.db, id, name, body.color.as_deref()).await?,
    ))
}

async fn delete_category(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_category(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- category slots ----------

// POST /api/categories/:id/slots — add a slot to a category.
async fn add_slot(
    State(state): State<AppState>,
    user: AuthUser,
    Path(category_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    Ok(Json(db::mappool::add_slot(&state.db, category_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSlotBody {
    editor_notes: String,
}

// PATCH /api/slots/:id — update a slot's editor notes.
async fn update_slot(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateSlotBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    Ok(Json(
        db::mappool::update_slot_notes(&state.db, id, &body.editor_notes).await?,
    ))
}

// DELETE /api/slots/:id — remove a slot.
async fn delete_slot(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_slot(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- pool maps ----------

// Resolves a beatmap's firm mod stats: star rating / AR / OD from the osu! API's
// difficulty attributes (skipped for nomod), and CS / BPM / length computed
// deterministically. Locked onto the map at add time — never recomputed afterwards.
async fn resolve_stats(
    state: &AppState,
    bm: &osu_api::beatmap::Beatmap,
    mods: &str,
) -> Result<db::mappool::ModStats, AppError> {
    use crate::mods::{modded_ar, modded_cs, modded_od};

    let parsed = crate::mods::parse(mods);
    let rate = crate::mods::rate(&parsed);

    // Only the star rating needs osu!'s difficulty calculation (nomod uses the value on
    // the beatmap response). A nomod map also gets its HR stats, for freemod picks.
    // osu! names OD `accuracy`.
    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    let star_rating = if parsed.is_empty() {
        bm.difficulty_rating
    } else {
        osu_api::attributes::fetch(&state.http_client, &token, bm.id, &parsed)
            .await?
            .star_rating
    };
    let hr = if parsed.is_empty() {
        let hr = vec!["HR".to_string()];
        let attr = osu_api::attributes::fetch(&state.http_client, &token, bm.id, &hr).await?;
        Some(db::mappool::HrStats {
            star_rating: attr.star_rating,
            cs: modded_cs(bm.cs, &hr),
            ar: modded_ar(bm.ar, &hr),
            od: modded_od(bm.accuracy, &hr),
        })
    } else {
        None
    };

    Ok(db::mappool::ModStats {
        star_rating,
        ar: modded_ar(bm.ar, &parsed),
        od: modded_od(bm.accuracy, &parsed),
        cs: modded_cs(bm.cs, &parsed),
        bpm: bm.bpm.unwrap_or(0.0) * rate,
        total_length: (bm.total_length as f64 / rate).round() as i32,
        hr,
    })
}

#[derive(Debug, Deserialize)]
pub struct AddMapBody {
    beatmap_id: i64,
    #[serde(default)]
    mods: String,
}

// POST /api/pool — add a map (beatmap id + mods) to the generic pool. The beatmap is
// fetched from the osu! API, its mod stats are resolved, and the whole thing (metadata +
// locked stats) is stored self-contained in one pool_maps row.
async fn add_map(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<AddMapBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let mods = body.mods.trim().to_uppercase();

    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    let bm = osu_api::beatmap::fetch(&state.http_client, &token, body.beatmap_id).await?;
    let stats = resolve_stats(&state, &bm, &mods).await?;

    let set = bm.beatmapset.as_ref();
    let new = db::mappool::NewPoolMap {
        beatmap_id: bm.id,
        beatmapset_id: bm.beatmapset_id,
        artist: set.map(|s| s.artist.clone()).unwrap_or_default(),
        title: set.map(|s| s.title.clone()).unwrap_or_default(),
        version: bm.version.clone(),
        creator: set.map(|s| s.creator.clone()),
        mode: bm.mode.clone(),
        cover_url: set.and_then(|s| s.covers.cover.clone()),
        mods,
        stats,
    };
    let id = db::mappool::add_pool_map(&state.db, &new, user.id).await?;
    let map = db::mappool::get_pool_map(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(map))
}

#[derive(Debug, Deserialize)]
pub struct MoveMapBody {
    // Target slot, or null/absent for the generic pool.
    #[serde(default)]
    slot_id: Option<Uuid>,
}

// PATCH /api/maps/:id — assign a map to a slot or back to the generic pool. Stats and
// mods are locked, so nothing about them changes.
async fn move_map(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveMapBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::move_pool_map(&state.db, id, body.slot_id).await?;
    let map = db::mappool::get_pool_map(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(map))
}

// POST /api/maps/:id/stats — resolve the map's stats again from osu! and lock them in
// anew: for maps added while AR/OD missed their mods or before HR stats were stored, or
// a beatmap osu! has since changed. Mods, metadata, slot and notes stay.
async fn recompute_map_stats(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let map = db::mappool::get_pool_map(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    let bm = osu_api::beatmap::fetch(&state.http_client, &token, map.beatmap_id).await?;
    let stats = resolve_stats(&state, &bm, &map.mods).await?;
    db::mappool::update_pool_map_stats(&state.db, id, &stats).await?;
    let map = db::mappool::get_pool_map(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(map))
}

// DELETE /api/maps/:id — remove a map from the pool entirely.
async fn delete_map(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_pool_map(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct UpdateMapNotesBody {
    // Each is optional — only the fields present are changed.
    #[serde(default)]
    public_notes: Option<String>,
    #[serde(default)]
    editor_notes: Option<String>,
}

// PATCH /api/maps/:id/notes — update a map's public and/or editor notes.
async fn update_map_notes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateMapNotesBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let map = db::mappool::update_map_notes(
        &state.db,
        id,
        body.public_notes.as_deref(),
        body.editor_notes.as_deref(),
    )
    .await?;
    Ok(Json(map))
}

// ---------- public (unauthenticated) — published stages only ----------

async fn list_public_stages(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::stages::list_published_stages(&state.db).await?))
}

async fn get_public_stage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let stage = db::stages::get_published_stage(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let categories = db::mappool::list_categories(&state.db, id).await?;
    let slots = db::mappool::list_public_slots(&state.db, id).await?;
    let maps = db::mappool::list_public_maps(&state.db, id).await?;
    Ok(Json(db::mappool::PublicStageDetail {
        stage,
        categories,
        slots,
        maps,
    }))
}
