//! The seeding page (/admin/seeding, host-only): the qualifier's standings as the
//! bracket's seeding, rearranged by hand if need be, and the bracket made from it or
//! taken down again (crate::services::seeding).

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser, error::AppError, role::Role, seeding::BracketFormat, services::seeding,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/seeding",
        get(get_seeding)
            .put(save_order)
            .delete(reset_order)
            .post(make_bracket),
    )
}

// GET /api/seeding — the teams in seeding order with their qualifier standings, and
// whether a bracket can be made from it (or taken down).
async fn get_seeding(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    Ok(Json(seeding::view(&state).await?))
}

#[derive(Debug, Deserialize)]
struct OrderBody {
    // Best seed first.
    team_ids: Vec<Uuid>,
}

// PUT /api/seeding — saves the hosts' order.
async fn save_order(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<OrderBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    seeding::save_order(&state, Some(&body.team_ids)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/seeding — back to the qualifier's order.
async fn reset_order(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    seeding::save_order(&state, None).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct BracketBody {
    format: BracketFormat,
    // The whole order, best seed first; the format takes its top 32 or 16.
    team_ids: Vec<Uuid>,
    // Each week's stage, from the first round to the grand final.
    stage_ids: Vec<Option<Uuid>>,
}

// POST /api/seeding — makes the double-elimination bracket from the seeding. The teams
// under the cut don't play in it: they're out of the tournament.
async fn make_bracket(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<BracketBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    seeding::make_bracket(&state, body.format, &body.team_ids, &body.stage_ids).await?;
    Ok(StatusCode::NO_CONTENT)
}
