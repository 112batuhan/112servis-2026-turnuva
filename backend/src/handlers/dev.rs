use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use crate::{auth, db, error::AppError, AppState};

// Testing helpers, mounted only when DEV_LOGIN is on. They let anyone sign in as
// any account without osu!, so turn it off once the site goes public.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dev/accounts", get(list_accounts))
        .route("/dev/login/:id", post(login_as))
        .route("/dev/seed", post(seed).delete(clear))
}

pub async fn list_accounts(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::dev::list_accounts(&state.db).await?))
}

// POST /api/dev/login/:id — replaces the current session with one for `id`.
pub async fn login_as(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if !db::dev::user_exists(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    let jar = auth::end_session(&state, jar).await?;
    let jar = jar.add(auth::start_session(&state, id).await?);
    Ok((jar, StatusCode::NO_CONTENT))
}

pub async fn seed(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    db::dev::seed(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn clear(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    db::dev::clear(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
