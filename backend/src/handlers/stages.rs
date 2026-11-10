use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{patch, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::ReorderBody;
use crate::{
    auth::AuthUser,
    db,
    discord::jobs::Job,
    draft::DraftOrder,
    error::{AppError, ErrorCode},
    role::Role,
    services, AppState,
};

// Routes under the `/api` nest. Stages are the tournament's rounds, which bracket matches
// also point at, so only hosts create, rename, reorder and delete them and say how their
// matches are played (the qualifier stage, best of, ban and pick order). Map poolers
// publish them and fill their pools (handlers/mappool.rs, which also lists and reads
// them); the pick'em's window per stage is in handlers/pickem.rs.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stages", post(create_stage))
        .route("/stages/reorder", post(reorder_stages))
        .route("/stages/qualifier", put(set_qualifier_stage))
        .route("/stages/{id}", patch(update_stage).delete(delete_stage))
        .route("/stages/{id}/best-of", put(set_stage_best_of))
        .route("/stages/{id}/draft-order", put(set_stage_draft_order))
}

#[derive(Debug, Deserialize)]
pub struct CreateStageBody {
    name: String,
}

// POST /api/stages — host-only.
async fn create_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateStageBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(ErrorCode::NameRequired));
    }
    Ok(Json(db::stages::create_stage(&state.db, name).await?))
}

// DELETE /api/stages/:id — host-only. Its maps go back to the generic pool and its
// bracket matches are left without a stage, so without potential matches.
async fn delete_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::stages::delete_stage(&state.db, id).await?;
    services::bracket::resync_potentials(&state).await?;
    state.live.bracket_changed();
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
async fn update_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateStageBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::MapPooler)?;
    if body.name.is_some() {
        user.require_role(Role::Host)?;
    }
    let name = body.name.as_deref().map(str::trim);
    if let Some(n) = name {
        if n.is_empty() {
            return Err(AppError::BadRequest(ErrorCode::NameRequired));
        }
    }
    Ok(Json(
        db::stages::update_stage(&state.db, id, name, body.published).await?,
    ))
}

// POST /api/stages/reorder — host-only. Persists a new stage order.
async fn reorder_stages(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ReorderBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::stages::reorder_stages(&state.db, &body.ids).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct QualifierStageBody {
    stage_id: Option<Uuid>,
}

// PUT /api/stages/qualifier — host-only. Picks the stage whose pool the qualifier
// lobbies play, or none.
async fn set_qualifier_stage(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<QualifierStageBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    match db::stages::set_qualifier_stage(&state.db, body.stage_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(AppError::NotFound),
        // Another host picked one at the same moment.
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("stages_qualifier_key") => {
            Err(AppError::Conflict(ErrorCode::QualifierStageChanged))
        }
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
async fn set_stage_best_of(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<BestOfBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if let Some(n) = body.best_of {
        if !(1..=MAX_BEST_OF).contains(&n) || n % 2 == 0 {
            return Err(AppError::BadRequest(ErrorCode::BadBestOf));
        }
    }
    let mut tx = state.db.begin().await?;
    let matches = db::stages::set_stage_best_of(&mut tx, id, body.best_of)
        .await?
        .ok_or(AppError::NotFound)?;
    let jobs = Job::sync_matches(matches);
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct DraftOrderBody {
    draft_order: Option<String>,
}

// PUT /api/stages/:id/draft-order — host-only. The order the stage's bracket matches
// ban and pick in ("p1-p2-b2-b1-p1-*", crate::draft), stored as it reads; empty or
// null goes back to the default. The referee page, the match page and the room's bot
// follow it.
async fn set_stage_draft_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<DraftOrderBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let order = match body.draft_order.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => Some(
            DraftOrder::parse(text)
                .map_err(AppError::BadRequest)?
                .format(),
        ),
    };
    if !db::stages::set_stage_draft_order(&state.db, id, order.as_deref()).await? {
        return Err(AppError::NotFound);
    }
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}
