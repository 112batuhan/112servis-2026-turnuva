use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{self, user::UserProfile},
    error::{AppError, AuthError},
    AppState,
};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/public/staff", get(list_public_staff))
}

#[derive(Serialize)]
struct Me {
    #[serde(flatten)]
    profile: UserProfile,
    // The team they play in, so the pages can tell which matches are theirs (their room
    // page rather than the public one).
    team_id: Option<Uuid>,
}

// GET /api/me — returns the caller's profile.
async fn me(State(state): State<AppState>, user: AuthUser) -> Result<impl IntoResponse, AppError> {
    let profile = db::user::find_user(&state.db, user.id)
        .await?
        .ok_or(AuthError::Unauthenticated)?;
    let team_id = db::team::find_team_id(&state.db, user.id).await?;
    Ok(Json(Me { profile, team_id }))
}

// GET /api/public/staff — everyone holding a staff role, with their Discord.
async fn list_public_staff(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::user::list_staff(&state.db).await?))
}
