use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};

use crate::{
    auth::{self, AuthUser},
    db,
    error::AppError,
    jwt, AppState,
};

// GET /api/me — returns the caller's profile. This is also the single place a
// token is reconciled: if the stored token_version has moved past the one in the
// JWT (e.g. an admin changed the role), re-issue the cookie with fresh claims.
// The frontend calls /api/me on load, so that page open is enough to catch up —
// no other endpoint checks token_version.
pub async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
    user: AuthUser,
) -> Result<Response, AppError> {
    let profile = db::user::find_user(&state.db, user.id)
        .await?
        .ok_or(AppError::Unauthenticated)?;

    let stored_version = db::user::token_version(&state.db, user.id)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    if stored_version != user.token_version {
        let discord_verified = profile.discord.is_some();
        let token = jwt::encode_token(
            profile.osu.id,
            profile.osu.role,
            profile.osu.token_version,
            discord_verified,
            state.config.jwt_secret.as_bytes(),
        )?;
        return Ok((
            jar.add(auth::auth_cookie(token, state.config.secure_cookies())),
            Json(profile),
        )
            .into_response());
    }

    Ok(Json(profile).into_response())
}

// POST /auth/logout — clears the auth cookie.
pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from(auth::AUTH_COOKIE));
    (jar, axum::http::StatusCode::NO_CONTENT)
}
