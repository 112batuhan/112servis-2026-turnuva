use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    auth::AuthUser,
    db,
    error::{AppError, ErrorCode},
    role::Role,
    AppState,
};

// Generous for a rules post (the last tournament's was about 6k), small enough to keep
// one request sane.
const MAX_LENGTH: usize = 100_000;

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/rules", get(get_rules))
        .route("/rules", put(save_rules))
}

// GET /api/public/rules — the rules as BBCode; the page renders it.
async fn get_rules(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::rules::get(&state.db).await?))
}

#[derive(Debug, Deserialize)]
pub struct SaveRulesBody {
    bbcode: String,
    // The `updated_at` the editor loaded, so a save can't silently overwrite another
    // host's newer one.
    #[serde(with = "time::serde::rfc3339")]
    base: OffsetDateTime,
}

// PUT /api/rules — host-only.
async fn save_rules(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<SaveRulesBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if body.bbcode.len() > MAX_LENGTH {
        return Err(AppError::BadRequest(ErrorCode::RulesTooLong));
    }

    let saved = db::rules::save(&state.db, &body.bbcode, body.base, user.id)
        .await?
        .ok_or(AppError::Conflict(ErrorCode::RulesChanged))?;
    Ok(Json(saved))
}
