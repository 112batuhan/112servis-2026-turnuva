use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};

use crate::{
    auth::AuthUser,
    db,
    error::{AppError, AuthError},
    AppState,
};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/public/staff", get(list_public_staff))
}

// GET /api/me — returns the caller's profile.
async fn me(State(state): State<AppState>, user: AuthUser) -> Result<impl IntoResponse, AppError> {
    let profile = db::user::find_user(&state.db, user.id)
        .await?
        .ok_or(AuthError::Unauthenticated)?;
    Ok(Json(profile))
}

// GET /api/public/staff — everyone holding a staff role, with their Discord.
async fn list_public_staff(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::user::list_staff(&state.db).await?))
}
