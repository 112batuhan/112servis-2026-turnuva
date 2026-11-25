//! The match's room page for its players (`/match/:id/players`): what they do there goes to
//! the room's bot, which runs the match (bancho/autoref/players.rs).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    bancho::PlayerAction,
    error::{AppError, MatchError},
    services::matches::load_match,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/matches/{id}/player", post(act))
}

// POST /api/matches/:id/player — a ban or pick, a warmup (or skipping it), an abort, a
// report or the emergency stop, by a player of the match. Whether it's theirs to do now
// (their team's turn, a map in its first 30 seconds…) is the bot's to say.
async fn act(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(action): Json<PlayerAction>,
) -> Result<StatusCode, AppError> {
    // A banned player can still tell the staff something's wrong.
    if !matches!(action, PlayerAction::Report { .. }) {
        user.require_not_banned()?;
    }
    if load_match(&state, id).await?.concluded() {
        return Err(MatchError::Concluded.into());
    }
    state.bancho.player(id, user.id, action).await?;
    Ok(StatusCode::NO_CONTENT)
}
