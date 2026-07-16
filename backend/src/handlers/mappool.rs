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
        .route("/pool", post(add_to_generic))
        .route("/pool/:beatmap_id", delete(remove_from_generic))
        .route("/stages/:id/entries", post(add_entry))
        .route("/entries/:id", patch(move_entry).delete(delete_entry))
        .route("/public/stages", get(list_public_stages))
        .route("/public/stages/:id", get(get_public_stage))
}

// Every authed map pool endpoint requires the map_pooler role (host qualifies too).
fn guard(user: &AuthUser) -> Result<(), AppError> {
    user.require_role(Role::MapPooler)
}

// GET /api/stages — the list of stages (for the stage selector).
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

// POST /api/stages — create a new stage.
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

// GET /api/stages/:id — a stage with its categories, categorised entries, and the
// shared generic pool (minus maps already categorised in this stage).
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
    let entries = db::mappool::list_entries(&state.db, id).await?;
    let generic = db::mappool::list_generic(&state.db).await?;
    Ok(Json(db::mappool::StageDetail {
        stage,
        categories,
        entries,
        generic,
    }))
}

// DELETE /api/stages/:id
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

// PATCH /api/stages/:id — publish or unpublish a stage.
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

#[derive(Debug, Deserialize)]
pub struct CreateCategoryBody {
    name: String,
    #[serde(default)]
    modifier: Option<String>,
}

// POST /api/stages/:id/categories — add a mod category to a stage.
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
    let modifier = body
        .modifier
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    Ok(Json(
        db::mappool::create_category(&state.db, stage_id, name, modifier).await?,
    ))
}

// DELETE /api/categories/:id
pub async fn delete_category(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_category(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AddBeatmapBody {
    beatmap_id: i64,
}

// POST /api/pool — add a beatmap (by osu! id) to the global generic pool. The
// beatmap is fetched from the osu! API and cached before being added to the library.
pub async fn add_to_generic(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<AddBeatmapBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;

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
    db::mappool::add_to_generic(&state.db, bm.id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/pool/:beatmap_id — remove a beatmap from the library and every stage.
pub async fn remove_from_generic(
    State(state): State<AppState>,
    user: AuthUser,
    Path(beatmap_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::remove_from_generic(&state.db, beatmap_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AddEntryBody {
    beatmap_id: i64,
    category_id: Uuid,
}

// POST /api/stages/:id/entries — categorise a library map into a stage category.
pub async fn add_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Path(stage_id): Path<Uuid>,
    Json(body): Json<AddEntryBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    let entry_id = db::mappool::add_entry(
        &state.db,
        stage_id,
        body.beatmap_id,
        body.category_id,
        user.id,
    )
    .await?;
    let entry = db::mappool::get_entry(&state.db, entry_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(entry))
}

#[derive(Debug, Deserialize)]
pub struct MoveEntryBody {
    category_id: Uuid,
}

// PATCH /api/entries/:id — move an entry to another category.
pub async fn move_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveEntryBody>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::move_entry(&state.db, id, body.category_id).await?;
    let entry = db::mappool::get_entry(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(entry))
}

// DELETE /api/entries/:id — uncategorise (send the map back to the generic pool).
pub async fn delete_entry(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    guard(&user)?;
    db::mappool::delete_entry(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- public (unauthenticated) — published stages only ----
// These take no AuthUser, so they run for anyone, and only ever expose stages that
// have been published.

// GET /api/public/stages — published stages (for the public stage selector).
pub async fn list_public_stages(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::mappool::list_published_stages(&state.db).await?))
}

// GET /api/public/stages/:id — a published stage's categories and their maps.
pub async fn get_public_stage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let stage = db::mappool::get_published_stage(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let categories = db::mappool::list_categories(&state.db, id).await?;
    let entries = db::mappool::list_entries(&state.db, id).await?;
    Ok(Json(db::mappool::PublicStageDetail {
        stage,
        categories,
        entries,
    }))
}
