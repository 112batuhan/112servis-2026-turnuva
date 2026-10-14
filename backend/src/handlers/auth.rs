use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use oauth2::{reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Deserialize;

use crate::{
    auth::{self, AuthUser},
    db, discord_api,
    error::AppError,
    osu_api, rank_refresh, AppState,
};

// Routes under the `/auth` nest. Logout lives in the `user` handler but is an auth
// action, so it's grouped here.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/osu", get(osu_login))
        .route("/osu/callback", get(osu_callback))
        .route("/discord/link", get(discord_link))
        .route("/discord/callback", get(discord_callback))
        .route("/logout", post(super::user::logout))
}

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

// GET /auth/osu/callback — exchanges the code, upserts the user, and starts a session.
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

    let before = db::ranks::find_by_osu_id(&state.db, profile.id).await?;
    let user = db::user::upsert_osu_user(&state.db, &profile).await?;
    if let Some((user_id, old_rank)) = before {
        let change = db::ranks::RankChange {
            user_id,
            old_rank,
            new_rank: user.global_rank,
        };
        rank_refresh::notify_over_limit(&state, &[change]).await;
    }

    let jar = jar.add(auth::start_session(&state, user.id).await?);
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

    // The session reads Discord status from the database, so the link applies to
    // the very next request with no cookie changes.
    db::user::link_discord(&state.db, user.id, &profile).await?;
    Ok(Redirect::to(&state.config.frontend_url))
}
