use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar};

use crate::{
    auth::{current_user_id, AUTH_COOKIE},
    db,
    error::AppError,
    AppState,
};

// GET /api/me — returns the logged-in user's stored profile.
pub async fn me(State(state): State<AppState>, jar: CookieJar) -> Result<impl IntoResponse, AppError> {
    let Some(user_id) = current_user_id(&jar, state.jwt_secret.as_bytes()) else {
        return Err(AppError::Unauthenticated);
    };

    let user = db::user::find_user(&state.db, user_id).await?.ok_or(AppError::Unauthenticated)?;
    Ok(Json(user))
}

// POST /auth/logout — clears the auth cookie.
pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from(AUTH_COOKIE));
    (jar, axum::http::StatusCode::NO_CONTENT)
}
