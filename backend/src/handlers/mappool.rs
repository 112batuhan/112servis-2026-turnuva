use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::AuthUser, db, error::AppError, osu_api, role::Role, AppState};

// Routes under the `/api` nest. The authed endpoints require map_pooler+; the two
// `/public/*` ones take no AuthUser and only expose published stages.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stages", get(list_stages).post(create_stage))
        .route(
            "/stages/:id",
            get(get_stage).delete(delete_stage).patch(set_published),
        )
        .route("/stages/:id/categories", post(create_category))
        .route("/categories/:id", delete(delete_category))
        .route("/pool", post(add_map))
        .route("/maps/:id", patch(move_map).delete(delete_map))
        .route("/public/stages", get(list_public_stages))
        .route("/public/stages/:id", get(get_public_stage))
}

// Every authed map pool endpoint requires the map_pooler role (host qualifies too).
fn guard(user: &AuthUser) -> Result<(), AppError> {
    user.require_role(Role::MapPooler)
}

// ---------- stages ----------

pub async fn list_stages(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    Ok(Json(db::mappool::list_stages(&state.db).await?))
}

#[derive(Debug, Deserialize)]
pub struct CreateStageBody {
    name: String,
}

pub async fn create_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateStageBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
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
    guard(&user)?;
    let stage = db::mappool::get_stage(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let categories = db::mappool::list_categories(&state.db, id).await?;
    let maps = db::mappool::list_stage_maps(&state.db, id).await?;
    Ok(Json(db::mappool::StageDetail {
        stage,
        categories,
        maps,
    }))
}

pub async fn delete_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_stage(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SetPublishedBody {
    published: bool,
}

pub async fn set_published(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SetPublishedBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    Ok(Json(
        db::mappool::set_published(&state.db, id, body.published).await?,
    ))
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

pub async fn delete_category(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_category(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- pool maps ----------

// Resolves a beatmap's firm mod stats: star rating / AR / OD from the osu! API's
// difficulty attributes (skipped for nomod), and CS / HP / BPM / length computed
// deterministically. Locked onto the map at add time — never recomputed afterwards.
async fn resolve_stats(
    state: &AppState,
    beatmap: &db::mappool::Beatmap,
    mods: &str,
) -> Result<db::mappool::ModStats, AppError> {
    let parsed = crate::mods::parse(mods);

    let cs = crate::mods::modded_cs(beatmap.cs, &parsed);
    let hp = crate::mods::modded_hp(beatmap.hp, &parsed);
    let rate = crate::mods::rate(&parsed);
    let bpm = beatmap.bpm * rate;
    let total_length = (beatmap.total_length as f64 / rate).round() as i32;

    let (star_rating, ar, od) = if parsed.is_empty() {
        (beatmap.star_rating, beatmap.ar, beatmap.od)
    } else {
        let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
        let attr =
            osu_api::attributes::fetch(&state.http_client, &token, beatmap.id, &parsed).await?;
        (
            attr.star_rating,
            attr.approach_rate.unwrap_or(beatmap.ar),
            attr.overall_difficulty.unwrap_or(beatmap.od),
        )
    };

    Ok(db::mappool::ModStats {
        star_rating,
        ar,
        od,
        cs,
        hp,
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
// fetched from the osu! API and cached, and its mod stats are resolved and locked in.
pub async fn add_map(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<AddMapBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let mods = body.mods.trim().to_uppercase();

    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    let bm = osu_api::beatmap::fetch(&state.http_client, &token, body.beatmap_id).await?;

    let set = bm.beatmapset.as_ref();
    let new = db::mappool::NewBeatmap {
        id: bm.id,
        beatmapset_id: bm.beatmapset_id,
        artist: set.map(|s| s.artist.clone()).unwrap_or_default(),
        title: set.map(|s| s.title.clone()).unwrap_or_default(),
        version: bm.version.clone(),
        creator: set.map(|s| s.creator.clone()),
        star_rating: bm.difficulty_rating,
        bpm: bm.bpm.unwrap_or(0.0),
        total_length: bm.total_length,
        cs: bm.cs,
        ar: bm.ar,
        od: bm.accuracy,
        hp: bm.drain,
        mode: bm.mode.clone(),
        cover_url: set.and_then(|s| s.covers.cover.clone()),
    };
    db::mappool::upsert_beatmap(&state.db, &new).await?;

    let beatmap = db::mappool::get_beatmap(&state.db, bm.id)
        .await?
        .ok_or(AppError::NotFound)?;
    let stats = resolve_stats(&state, &beatmap, &mods).await?;

    let id = db::mappool::add_pool_map(&state.db, bm.id, &mods, &stats, user.id).await?;
    let map = db::mappool::get_pool_map(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(map))
}

#[derive(Debug, Deserialize)]
pub struct MoveMapBody {
    // Target category, or null/absent for the generic pool.
    #[serde(default)]
    category_id: Option<Uuid>,
}

// PATCH /api/maps/:id — move a map into a category or back to the generic pool. Stats
// and mods are locked, so nothing about them changes.
pub async fn move_map(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveMapBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::move_pool_map(&state.db, id, body.category_id).await?;
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
    let maps = db::mappool::list_public_maps(&state.db, id).await?;
    Ok(Json(db::mappool::PublicStageDetail {
        stage,
        categories,
        maps,
    }))
}
