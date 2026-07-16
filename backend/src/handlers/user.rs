use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth, db, error::AppError, jwt, role::Role, AppState};

// GET /api/me — returns the logged-in user's stored profile, and refreshes the
// auth cookie so an active client picks up role changes within one poll.
pub async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let Some(caller) = auth::current_user(&jar, state.jwt_secret.as_bytes()) else {
        return Err(AppError::Unauthenticated);
    };

    let profile = db::user::find_user(&state.db, caller.id)
        .await?
        .ok_or(AppError::Unauthenticated)?;

    // Re-sign from the DB row so role/token_version/discord status stay current.
    let discord_verified = profile.discord.is_some();
    let token = jwt::encode_token(
        profile.osu.id,
        profile.osu.role,
        profile.osu.token_version,
        discord_verified,
        state.jwt_secret.as_bytes(),
    )?;
    let jar = jar.add(auth::auth_cookie(token));

    Ok((jar, Json(profile)))
}

#[derive(Debug, Deserialize)]
pub struct SetRoleBody {
    role: Role,
}

// POST /api/users/{id}/role — host-only. Changes a user's role; the token_version
// bump inside set_role invalidates that user's existing JWTs immediately.
pub async fn set_role(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_id): Path<Uuid>,
    Json(body): Json<SetRoleBody>,
) -> Result<impl IntoResponse, AppError> {
    auth::require_role(&state.db, &jar, state.jwt_secret.as_bytes(), Role::Host).await?;

    let user = db::user::set_role(&state.db, user_id, body.role).await?;
    Ok(Json(user))
}

// GET /api/users — host-only. Lists every registered user with their linked
// Discord account (if any) folded in.
pub async fn list_users(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    auth::require_role(&state.db, &jar, state.jwt_secret.as_bytes(), Role::Host).await?;

    let users = db::user::list_users(&state.db).await?;
    Ok(Json(users))
}

// POST /auth/logout — clears the auth cookie.
pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from(auth::AUTH_COOKIE));
    (jar, axum::http::StatusCode::NO_CONTENT)
}
