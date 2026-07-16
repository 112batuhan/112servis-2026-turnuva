use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use oauth2::{reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Deserialize;

use crate::{
    auth::{self, AuthUser},
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
    state
        .csrf_tokens
        .lock()
        .unwrap()
        .insert(csrf_token.secret().clone(), ());
    Redirect::to(auth_url.as_str())
}

// GET /auth/osu/callback — exchanges the code, upserts the user, and issues a JWT.
pub async fn osu_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if state
        .csrf_tokens
        .lock()
        .unwrap()
        .remove(&params.state)
        .is_none()
    {
        return Err(AppError::InvalidLoginAttempt);
    }

    let token = state
        .osu_client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|err| AppError::OsuTokenExchange(err.to_string()))?;

    let profile =
        osu_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    let user = db::user::upsert_osu_user(&state.db, &profile).await?;
    let discord_verified = db::user::is_discord_linked(&state.db, user.id).await?;

    let token = jwt::encode_token(
        user.id,
        user.role,
        user.token_version,
        discord_verified,
        state.config.jwt_secret.as_bytes(),
    )?;
    let jar = jar.add(auth::auth_cookie(token, state.config.secure_cookies()));
    Ok((jar, Redirect::to(&state.config.frontend_url)))
}

// GET /auth/discord/link — starts the optional Discord link flow. The AuthUser
// extractor rejects the request with 401 unless there's already an osu! session.
pub async fn discord_link(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let (auth_url, csrf_token) = state
        .discord_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    state
        .csrf_tokens
        .lock()
        .unwrap()
        .insert(csrf_token.secret().clone(), ());
    Ok(Redirect::to(auth_url.as_str()))
}

// GET /auth/discord/callback — links the Discord profile to the currently logged-in user.
pub async fn discord_callback(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, AppError> {
    if state
        .csrf_tokens
        .lock()
        .unwrap()
        .remove(&params.state)
        .is_none()
    {
        return Err(AppError::InvalidLoginAttempt);
    }

    let token = state
        .discord_client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|err| AppError::DiscordTokenExchange(err.to_string()))?;

    let profile =
        discord_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    db::user::link_discord(&state.db, user.id, &profile).await?;

    // Re-issue so the linked status takes effect immediately, without waiting for
    // the next /api/me. Role and token_version are unchanged by linking.
    let token = jwt::encode_token(
        user.id,
        user.role,
        user.token_version,
        true,
        state.config.jwt_secret.as_bytes(),
    )?;
    let jar = jar.add(auth::auth_cookie(token, state.config.secure_cookies()));
    Ok((jar, Redirect::to(&state.config.frontend_url)))
}
