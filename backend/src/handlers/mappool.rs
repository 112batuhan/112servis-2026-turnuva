use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser, db, discord::jobs::Job, error::AppError, osu_api, role::Role, AppState,
};

// Routes under the `/api` nest. The authed endpoints require map_pooler+, except that
// only hosts create, rename, reorder and delete stages: stages are the tournament's
// rounds, which bracket matches also point at, and that pool testers and custom
// mappers can also read the stages (`view_guard`) but change nothing. The two
// `/public/*` ones take no AuthUser and only expose published stages.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stages", get(list_stages).post(create_stage))
        .route("/stages/reorder", post(reorder_stages))
        .route("/stages/qualifier", put(set_qualifier_stage))
        .route(
            "/stages/:id",
            get(get_stage).delete(delete_stage).patch(update_stage),
        )
        .route("/stages/:id/best-of", put(set_stage_best_of))
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

pub async fn list_stages(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    view_guard(&user)?;
    Ok(Json(db::mappool::list_stages(&state.db).await?))
}

#[derive(Debug, Deserialize)]
pub struct CreateStageBody {
    name: String,
}

// POST /api/stages — host-only.
pub async fn create_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateStageBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("stage name is required"));
    }
    Ok(Json(db::mappool::create_stage(&state.db, name).await?))
}

// GET /api/stages/:id — a stage with its categories and every relevant map (the
// global generic pool plus this stage's categorised maps).
pub async fn get_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    view_guard(&user)?;
    let stage = db::mappool::get_stage(&state.db, id)
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

// DELETE /api/stages/:id — host-only. Its maps go back to the generic pool and its
// bracket matches are left without a stage.
pub async fn delete_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::mappool::delete_stage(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct UpdateStageBody {
    // Each field is optional — only the ones present are changed.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    published: Option<bool>,
}

// PATCH /api/stages/:id — rename (host-only) and/or publish/unpublish a stage's pool.
pub async fn update_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateStageBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    if body.name.is_some() {
        user.require_role(Role::Host)?;
    }
    let name = body.name.as_deref().map(str::trim);
    if let Some(n) = name {
        if n.is_empty() {
            return Err(AppError::BadRequest("stage name is required"));
        }
    }
    Ok(Json(
        db::mappool::update_stage(&state.db, id, name, body.published).await?,
    ))
}

// A new ordering for a set of items, most-first, given by their ids.
#[derive(Debug, Deserialize)]
pub struct ReorderBody {
    ids: Vec<Uuid>,
}

// POST /api/stages/reorder — host-only. Persists a new stage order.
pub async fn reorder_stages(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ReorderBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::mappool::reorder_stages(&state.db, &body.ids).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct QualifierStageBody {
    stage_id: Option<Uuid>,
}

// PUT /api/stages/qualifier — host-only. Picks the stage whose pool the qualifier
// lobbies play, or none.
pub async fn set_qualifier_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<QualifierStageBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    match db::mappool::set_qualifier_stage(&state.db, body.stage_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(AppError::NotFound),
        // Another host picked one at the same moment.
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("stages_qualifier_key") => Err(
            AppError::Conflict("the qualifier stage just changed, try again"),
        ),
        Err(err) => Err(err.into()),
    }
}

#[derive(Debug, Deserialize)]
pub struct BestOfBody {
    best_of: Option<i32>,
}

// The most maps a match may be played to; the column allows the same.
const MAX_BEST_OF: i32 = 25;

// PUT /api/stages/:id/best-of — host-only. How many maps the stage's bracket matches
// are played to (odd), or none. The referee page, the match pages and Discord show it.
pub async fn set_stage_best_of(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<BestOfBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if let Some(n) = body.best_of {
        if !(1..=MAX_BEST_OF).contains(&n) || n % 2 == 0 {
            return Err(AppError::BadRequest("best of must be an odd number from 1 to 25"));
        }
    }
    let matches = db::mappool::set_stage_best_of(&state.db, id, body.best_of)
        .await?
        .ok_or(AppError::NotFound)?;
    let jobs: Vec<Job> = matches
        .into_iter()
        .map(|match_id| Job::SyncMatch { match_id })
        .collect();
    state.discord.enqueue(&state.db, &jobs).await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

// ---------- categories ----------

#[derive(Debug, Deserialize)]
pub struct CreateCategoryBody {
    name: String,
}

pub async fn create_category(
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
pub async fn reorder_categories(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ReorderBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::reorder_categories(&state.db, &body.ids).await?;
    Ok(StatusCode::NO_CONTENT)
}

// PATCH /api/categories/:id — rename and/or recolour a category.
pub async fn update_category(
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

pub async fn delete_category(
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
pub async fn add_slot(
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
pub async fn update_slot(
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
pub async fn delete_slot(
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
    let parsed = crate::mods::parse(mods);

    let cs = crate::mods::modded_cs(bm.cs, &parsed);
    let rate = crate::mods::rate(&parsed);
    let bpm = bm.bpm.unwrap_or(0.0) * rate;
    let total_length = (bm.total_length as f64 / rate).round() as i32;

    // osu! names OD `accuracy`. SR/AR/OD come from the difficulty-attributes API under
    // mods; nomod uses the values already on the beatmap response.
    let (star_rating, ar, od) = if parsed.is_empty() {
        (bm.difficulty_rating, bm.ar, bm.accuracy)
    } else {
        let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
        let attr = osu_api::attributes::fetch(&state.http_client, &token, bm.id, &parsed).await?;
        (
            attr.star_rating,
            attr.approach_rate.unwrap_or(bm.ar),
            attr.overall_difficulty.unwrap_or(bm.accuracy),
        )
    };

    Ok(db::mappool::ModStats {
        star_rating,
        ar,
        od,
        cs,
        bpm,
        total_length,
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
pub async fn add_map(
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
pub async fn move_map(
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

// DELETE /api/maps/:id — remove a map from the pool entirely.
pub async fn delete_map(
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
pub async fn update_map_notes(
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

pub async fn list_public_stages(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::mappool::list_published_stages(&state.db).await?))
}

pub async fn get_public_stage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let stage = db::mappool::get_published_stage(&state.db, id)
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
