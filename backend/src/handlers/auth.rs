use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::{reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Deserialize;
use time::Duration;

use crate::{
    auth::{current_user_id, AUTH_COOKIE},
    db, discord_api,
    error::AppError,
    jwt, osu_api, AppState,
};

// Both OAuth providers redirect back with the same `?code=&state=` query shape.
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

// GET /auth/osu — starts the primary login flow.
pub async fn osu_login(State(state): State<AppState>) -> impl IntoResponse {
    let (auth_url, csrf_token) = state
        .osu_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    state.csrf_tokens.lock().unwrap().insert(csrf_token.secret().clone(), ());
    Redirect::to(auth_url.as_str())
}

// GET /auth/osu/callback — exchanges the code, upserts the user, and issues a JWT.
pub async fn osu_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if state.csrf_tokens.lock().unwrap().remove(&params.state).is_none() {
        return Err(AppError::InvalidLoginAttempt);
    }

    let token = state
        .osu_client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|err| AppError::OsuTokenExchange(err.to_string()))?;

    let profile = osu_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    let user = db::user::upsert_osu_user(&state.db, &profile).await?;

    let token = jwt::encode_token(user.id, state.jwt_secret.as_bytes())?;

    let mut cookie = Cookie::new(AUTH_COOKIE, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Duration::seconds(jwt::TOKEN_TTL_SECS as i64));
    cookie.set_secure(false);

    let jar = jar.add(cookie);
    Ok((jar, Redirect::to(&state.frontend_url)))
}

// GET /auth/discord/link — starts the optional Discord link flow; requires an osu! session already.
pub async fn discord_link(State(state): State<AppState>, jar: CookieJar) -> Result<impl IntoResponse, AppError> {
    if current_user_id(&jar, state.jwt_secret.as_bytes()).is_none() {
        return Err(AppError::Unauthenticated);
    }

    let (auth_url, csrf_token) = state
        .discord_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    state.csrf_tokens.lock().unwrap().insert(csrf_token.secret().clone(), ());
    Ok(Redirect::to(auth_url.as_str()))
}

// GET /auth/discord/callback — links the Discord profile to the currently logged-in user.
pub async fn discord_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let Some(user_id) = current_user_id(&jar, state.jwt_secret.as_bytes()) else {
        return Err(AppError::Unauthenticated);
    };

    if state.csrf_tokens.lock().unwrap().remove(&params.state).is_none() {
        return Err(AppError::InvalidLoginAttempt);
    }

    let token = state
        .discord_client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|err| AppError::DiscordTokenExchange(err.to_string()))?;

    let profile = discord_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    db::user::link_discord(&state.db, user_id, &profile).await?;

    Ok(Redirect::to(&state.frontend_url))
}
