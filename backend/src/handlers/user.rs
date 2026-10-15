use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    auth::{self, AuthUser},
    db,
    error::AppError,
    AppState,
};

// Routes under the `/api` nest. (Logout is routed in the `auth` handler.)
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/public/staff", get(list_public_staff))
}

// GET /api/me — returns the caller's profile.
pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let profile = db::user::find_user(&state.db, user.id)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    Ok(Json(profile))
}

// GET /api/public/staff — everyone holding a staff role, with their Discord.
async fn list_public_staff(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::user::list_staff(&state.db).await?))
}

// POST /auth/logout — deletes the session and clears its cookie.
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let jar = auth::end_session(&state, jar).await?;
    Ok((jar, axum::http::StatusCode::NO_CONTENT))
}
